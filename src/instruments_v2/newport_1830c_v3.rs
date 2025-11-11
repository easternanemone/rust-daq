//! Newport 1830C Power Meter V3 implementation.
//!
//! This module provides a V3 instrument driver that natively implements the
//! [`Instrument`](crate::core_v3::Instrument) and [`PowerMeter`](crate::core_v3::PowerMeter)
//! traits. The design follows the Phase 2 migration plan documented in
//! `docs/plans/2025-10-25-phase-2-instrument-migrations.md` and establishes a
//! reusable serial abstraction so unit tests can operate entirely in-process
//! without depending on hardware.

use crate::core::ParameterValue;
use crate::core_v3::{
    Command, Instrument, InstrumentState, Measurement, ParameterBase, PowerMeter, Response,
};
use crate::parameter::{Parameter, ParameterBuilder};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use futures::executor;
use log::{debug, info, warn};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::{broadcast, oneshot, RwLock};
use tokio::task::JoinHandle;
use tokio::time::Duration;

type SerialHandle = Arc<dyn SerialDevice>;

/// Trait abstraction over serial communication so we can substitute mocks in
/// tests while using the real `serialport` backend in production.
#[async_trait]
trait SerialDevice: Send + Sync {
    async fn connect(&self) -> Result<()>;
    async fn disconnect(&self) -> Result<()>;
    async fn query(&self, command: &str) -> Result<String>;
    async fn send(&self, command: &str) -> Result<()>;
}

/// Driver configuration parsed from TOML/JSON.
#[derive(Clone, Debug)]
struct NewportConfig {
    port: String,
    baud_rate: u32,
    timeout_ms: u64,
    poll_hz: f64,
    wavelength_nm: f64,
    range: RangeSetting,
    units: PowerUnits,
}

impl NewportConfig {
    fn from_settings(settings: &JsonValue) -> Result<Self> {
        let port = settings
            .get("port")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Newport1830CV3 requires 'port' in config"))?
            .to_string();

        let baud_rate = settings
            .get("baud_rate")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .unwrap_or(9600);

        let timeout_ms = settings
            .get("timeout_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(500);

        let poll_hz = settings
            .get("poll_hz")
            .or_else(|| settings.get("polling_hz"))
            .or_else(|| settings.get("sampling_rate"))
            .and_then(|v| v.as_f64())
            .unwrap_or(10.0);

        let wavelength_nm = settings
            .get("wavelength_nm")
            .and_then(|v| v.as_f64())
            .unwrap_or(532.0);

        let range = RangeSetting::from_value(settings.get("range"))?;
        let units = settings
            .get("units")
            .and_then(|v| v.as_str())
            .map(PowerUnits::from_str)
            .transpose()?
            .unwrap_or(PowerUnits::Watts);

        Ok(Self {
            port,
            baud_rate,
            timeout_ms,
            poll_hz: poll_hz.clamp(0.1, 200.0),
            wavelength_nm,
            range,
            units,
        })
    }
}

#[derive(Clone, Copy, Debug)]
enum RangeSetting {
    Auto,
    Watts(f64),
}

impl RangeSetting {
    fn from_value(value: Option<&JsonValue>) -> Result<Self> {
        match value {
            Some(JsonValue::String(s)) if s.eq_ignore_ascii_case("auto") => Ok(Self::Auto),
            Some(JsonValue::Number(n)) => n
                .as_f64()
                .map(Self::Watts)
                .ok_or_else(|| anyhow!("Invalid numeric range")),
            Some(JsonValue::String(s)) => s
                .parse::<f64>()
                .map(Self::Watts)
                .map_err(|e| anyhow!("Invalid range '{}': {}", s, e)),
            Some(_) => Err(anyhow!("Unsupported type for 'range'")),
            None => Ok(Self::Auto),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PowerUnits {
    Watts,
    DBm,
    DB,
    Relative,
}

impl PowerUnits {
    fn code(self) -> i32 {
        match self {
            PowerUnits::Watts => 0,
            PowerUnits::DBm => 1,
            PowerUnits::DB => 2,
            PowerUnits::Relative => 3,
        }
    }

    fn abbreviation(self) -> &'static str {
        match self {
            PowerUnits::Watts => "W",
            PowerUnits::DBm => "dBm",
            PowerUnits::DB => "dB",
            PowerUnits::Relative => "REL",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            PowerUnits::Watts => "Watts",
            PowerUnits::DBm => "dBm",
            PowerUnits::DB => "dB",
            PowerUnits::Relative => "REL",
        }
    }
}

impl FromStr for PowerUnits {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "watts" | "w" => Ok(PowerUnits::Watts),
            "dbm" => Ok(PowerUnits::DBm),
            "db" => Ok(PowerUnits::DB),
            "rel" | "relative" => Ok(PowerUnits::Relative),
            _ => Err(anyhow!(
                "Unsupported unit '{}'. Expected watts|dBm|dB|REL",
                value
            )),
        }
    }
}

impl std::fmt::Display for PowerUnits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.abbreviation())
    }
}

struct ParameterHandles {
    wavelength_nm: Arc<RwLock<Parameter<f64>>>,
    range_watts: Arc<RwLock<Parameter<f64>>>,
    poll_rate_hz: Arc<RwLock<Parameter<f64>>>,
    units: Arc<RwLock<Parameter<String>>>,
}

/// Newport 1830C V3 instrument implementation.
pub struct Newport1830CV3 {
    id: String,
    state: InstrumentState,
    serial: SerialHandle,
    config: NewportConfig,
    data_tx: broadcast::Sender<Measurement>,
    parameters: HashMap<String, Box<dyn ParameterBase>>,
    handles: ParameterHandles,
    poll_task: Option<JoinHandle<()>>,
    poll_shutdown: Option<oneshot::Sender<()>>,
}

impl Newport1830CV3 {
    /// Instantiate from configuration JSON (InstrumentManagerV3 entry).
    pub fn from_config(id: &str, cfg: &JsonValue) -> Result<Box<dyn Instrument>> {
        let config = NewportConfig::from_settings(cfg)?;
        let serial = build_serial_handle(&config)?;
        Ok(Box::new(Self::with_serial(id.into(), config, serial)))
    }

    #[cfg(test)]
    fn with_mock(id: &str, config: NewportConfig, serial: SerialHandle) -> Self {
        Self::with_serial(id.to_string(), config, serial)
    }

    fn with_serial(id: String, config: NewportConfig, serial: SerialHandle) -> Self {
        let (parameters, handles) = build_parameters(&config);
        let (data_tx, _rx) = broadcast::channel(1024);

        Self {
            id,
            state: InstrumentState::Uninitialized,
            serial,
            config,
            data_tx,
            parameters,
            handles,
            poll_task: None,
            poll_shutdown: None,
        }
    }

    async fn configure_from_params(&mut self) -> Result<()> {
        let wavelength = self.handles.wavelength_nm.read().await.get();
        let range = self.handles.range_watts.read().await.get();
        let units = {
            let guard = self.handles.units.read().await;
            PowerUnits::from_str(&guard.get()).unwrap_or(self.config.units)
        };

        self.set_wavelength_internal(wavelength).await?;
        self.set_range_internal(range).await?;
        self.set_units_internal(units).await?;
        Ok(())
    }

    async fn start_polling(&mut self) -> Result<()> {
        if self.poll_task.is_some() {
            return Ok(());
        }

        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let serial = self.serial.clone();
        let data_tx = self.data_tx.clone();
        let id = self.id.clone();
        let poll_param = self.handles.poll_rate_hz.clone();
        let units_param = self.handles.units.clone();

        self.poll_task = Some(tokio::spawn(async move {
            poll_loop(serial, data_tx, id, poll_param, units_param, shutdown_rx).await;
        }));
        self.poll_shutdown = Some(shutdown_tx);
        Ok(())
    }

    async fn stop_polling(&mut self) {
        if let Some(tx) = self.poll_shutdown.take() {
            let _ = tx.send(());
        }

        if let Some(task) = self.poll_task.take() {
            task.abort();
        }
    }

    async fn handle_command(&mut self, cmd: Command) -> Result<Response> {
        match cmd {
            Command::Start => {
                self.start_polling().await?;
                self.state = InstrumentState::Running;
                Ok(Response::State(self.state))
            }
            Command::Stop => {
                self.stop_polling().await;
                self.state = InstrumentState::Idle;
                Ok(Response::State(self.state))
            }
            Command::GetState => Ok(Response::State(self.state)),
            Command::GetParameter(name) => {
                let value = self
                    .parameters
                    .get(&name)
                    .ok_or_else(|| anyhow!("Unknown parameter '{}'", name))?
                    .value_json();
                Ok(Response::Parameter(value))
            }
            Command::SetParameter(name, value) => {
                self.apply_parameter(&name, value).await?;
                Ok(Response::Ok)
            }
            Command::Configure { params } => {
                for (name, value) in params {
                    self.apply_parameter(&name, parameter_value_to_json(value))
                        .await?;
                }
                Ok(Response::Ok)
            }
            Command::Custom(op, _) if op == "read_power" => {
                let value = self.read_power().await?;
                Ok(Response::Custom(serde_json::json!({ "power_w": value })))
            }
            _ => Ok(Response::Error("Unsupported command".to_string())),
        }
    }

    async fn apply_parameter(&mut self, name: &str, value: JsonValue) -> Result<()> {
        match name {
            "wavelength_nm" => {
                self.set_wavelength_internal(value.as_f64().unwrap_or(self.config.wavelength_nm))
                    .await
            }
            "range" | "range_watts" => {
                let watts = if let Some(value) = value.as_f64() {
                    value
                } else if let Some(text) = value.as_str() {
                    if text.eq_ignore_ascii_case("auto") {
                        0.0
                    } else {
                        text.parse().unwrap_or(0.0)
                    }
                } else {
                    0.0
                };
                self.set_range_internal(watts).await
            }
            "poll_rate_hz" => self.set_poll_rate(value.as_f64().unwrap_or(10.0)).await,
            "units" => {
                self.set_units_internal(PowerUnits::from_str(
                    value.as_str().unwrap_or(self.config.units.display_name()),
                )?)
                .await
            }
            _ => Err(anyhow!("Unknown parameter '{}'", name)),
        }
    }

    async fn set_poll_rate(&self, hz: f64) -> Result<()> {
        let hz = hz.clamp(0.1, 200.0);
        self.handles.poll_rate_hz.write().await.set(hz).await
    }

    async fn read_power(&self) -> Result<f64> {
        let response = self.serial.query("PM:P?").await?;
        response
            .trim()
            .parse::<f64>()
            .with_context(|| format!("Failed to parse power '{}'", response))
    }

    async fn set_wavelength_internal(&mut self, nm: f64) -> Result<()> {
        if !(200.0..=1800.0).contains(&nm) {
            return Err(anyhow!("Wavelength {} nm out of range (200-1800)", nm));
        }

        self.serial
            .send(&format!("PM:Lambda {:.2}", nm))
            .await
            .context("Failed to set wavelength")?;
        self.handles.wavelength_nm.write().await.set(nm).await?;
        Ok(())
    }

    async fn set_range_internal(&mut self, watts: f64) -> Result<()> {
        let idx = watts_to_range_index(watts);
        self.serial
            .send(&format!("PM:Range {}", idx))
            .await
            .context("Failed to set range")?;
        self.handles
            .range_watts
            .write()
            .await
            .set(watts.max(0.0))
            .await?;
        Ok(())
    }

    async fn set_units_internal(&mut self, units: PowerUnits) -> Result<()> {
        self.serial
            .send(&format!("PM:Units {}", units.code()))
            .await
            .context("Failed to set units")?;
        self.handles
            .units
            .write()
            .await
            .set(units.display_name().to_string())
            .await?;
        Ok(())
    }
}

#[async_trait]
impl Instrument for Newport1830CV3 {
    fn id(&self) -> &str {
        &self.id
    }

    fn state(&self) -> InstrumentState {
        self.state
    }

    async fn initialize(&mut self) -> Result<()> {
        if self.state != InstrumentState::Uninitialized {
            return Err(anyhow!("Instrument already initialized"));
        }

        self.serial.connect().await?;
        let idn = self
            .serial
            .query("*IDN?")
            .await
            .context("Failed to read identification")?;
        info!("Newport1830CV3 '{}' identified as {}", self.id, idn.trim());

        self.configure_from_params().await?;
        self.state = InstrumentState::Idle;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.stop_polling().await;
        self.serial.disconnect().await?;
        self.state = InstrumentState::ShuttingDown;
        Ok(())
    }

    fn data_channel(&self) -> broadcast::Receiver<Measurement> {
        self.data_tx.subscribe()
    }

    async fn execute(&mut self, cmd: Command) -> Result<Response> {
        self.handle_command(cmd).await
    }

    fn parameters(&self) -> &HashMap<String, Box<dyn ParameterBase>> {
        &self.parameters
    }

    fn parameters_mut(&mut self) -> &mut HashMap<String, Box<dyn ParameterBase>> {
        &mut self.parameters
    }
}

#[async_trait]
impl PowerMeter for Newport1830CV3 {
    async fn set_wavelength(&mut self, nm: f64) -> Result<()> {
        self.set_wavelength_internal(nm).await
    }

    async fn set_range(&mut self, watts: f64) -> Result<()> {
        self.set_range_internal(watts).await
    }

    async fn zero(&mut self) -> Result<()> {
        self.serial
            .send("PM:DS:Clear")
            .await
            .context("Failed to zero power meter")
    }
}

fn watts_to_range_index(watts: f64) -> u8 {
    if watts <= 0.0 {
        return 0;
    }

    match watts {
        w if w >= 1.0 => 1,
        w if w >= 0.1 => 2,
        w if w >= 0.01 => 3,
        w if w >= 0.001 => 4,
        w if w >= 0.0001 => 5,
        w if w >= 0.00001 => 6,
        w if w >= 0.000001 => 7,
        _ => 8,
    }
}

fn parameter_value_to_json(value: ParameterValue) -> JsonValue {
    match value {
        ParameterValue::Bool(b) => JsonValue::Bool(b),
        ParameterValue::Int(i) => JsonValue::Number(i.into()),
        ParameterValue::Float(f) => serde_json::Number::from_f64(f)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null),
        ParameterValue::String(s) => JsonValue::String(s),
        ParameterValue::FloatArray(values) => JsonValue::Array(
            values
                .into_iter()
                .map(|v| {
                    serde_json::Number::from_f64(v)
                        .map(JsonValue::Number)
                        .unwrap_or(JsonValue::Null)
                })
                .collect(),
        ),
        ParameterValue::IntArray(values) => JsonValue::Array(
            values
                .into_iter()
                .map(|v| JsonValue::Number(v.into()))
                .collect(),
        ),
        ParameterValue::Array(values) => {
            JsonValue::Array(values.into_iter().map(parameter_value_to_json).collect())
        }
        ParameterValue::Object(map) => {
            let converted: serde_json::Map<String, JsonValue> = map
                .into_iter()
                .map(|(k, v)| (k, parameter_value_to_json(v)))
                .collect();
            JsonValue::Object(converted)
        }
        ParameterValue::Null => JsonValue::Null,
    }
}

fn build_parameters(
    config: &NewportConfig,
) -> (HashMap<String, Box<dyn ParameterBase>>, ParameterHandles) {
    let mut map: HashMap<String, Box<dyn ParameterBase>> = HashMap::new();

    let wavelength_param = ParameterBuilder::new("wavelength_nm", config.wavelength_nm)
        .description("Laser wavelength used for calibration")
        .unit("nm")
        .range(200.0, 1800.0)
        .build();
    let (wavelength_entry, wavelength_handle) = shared_parameter(wavelength_param);
    map.insert("wavelength_nm".into(), wavelength_entry);

    let range_param = ParameterBuilder::new(
        "range_watts",
        match config.range {
            RangeSetting::Auto => 0.0,
            RangeSetting::Watts(w) => w,
        },
    )
    .description("Maximum expected power before autorange kicks in")
    .unit("W")
    .range(0.0, 10.0)
    .build();
    let (range_entry, range_handle) = shared_parameter(range_param);
    map.insert("range_watts".into(), range_entry);

    let poll_param = ParameterBuilder::new("poll_rate_hz", config.poll_hz)
        .description("Polling rate for PM:P? queries")
        .unit("Hz")
        .range(0.1, 200.0)
        .build();
    let (poll_entry, poll_handle) = shared_parameter(poll_param);
    map.insert("poll_rate_hz".into(), poll_entry);

    let units_param = ParameterBuilder::new("units", config.units.display_name().to_string())
        .description("Displayed output units for the sensor")
        .choices(vec![
            "Watts".to_string(),
            "dBm".to_string(),
            "dB".to_string(),
            "REL".to_string(),
        ])
        .build();
    let (units_entry, units_handle) = shared_parameter(units_param);
    map.insert("units".into(), units_entry);

    (
        map,
        ParameterHandles {
            wavelength_nm: wavelength_handle,
            range_watts: range_handle,
            poll_rate_hz: poll_handle,
            units: units_handle,
        },
    )
}

fn shared_parameter<T>(param: Parameter<T>) -> (Box<dyn ParameterBase>, Arc<RwLock<Parameter<T>>>)
where
    T: Clone
        + Send
        + Sync
        + PartialEq
        + PartialOrd
        + std::fmt::Debug
        + Serialize
        + DeserializeOwned
        + 'static,
{
    let name = param.name().to_string();
    let shared = Arc::new(RwLock::new(param));
    let binding = SharedParameter {
        name,
        inner: shared.clone(),
    };
    (Box::new(binding), shared)
}

struct SharedParameter<T>
where
    T: Clone
        + Send
        + Sync
        + PartialEq
        + PartialOrd
        + std::fmt::Debug
        + Serialize
        + DeserializeOwned
        + 'static,
{
    name: String,
    inner: Arc<RwLock<Parameter<T>>>,
}

impl<T> ParameterBase for SharedParameter<T>
where
    T: Clone
        + Send
        + Sync
        + PartialEq
        + PartialOrd
        + std::fmt::Debug
        + Serialize
        + DeserializeOwned
        + 'static,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn value_json(&self) -> JsonValue {
        let guard = self.inner.blocking_read();
        serde_json::to_value(guard.get()).unwrap_or(JsonValue::Null)
    }

    fn set_json(&mut self, value: JsonValue) -> Result<()> {
        let new_value: T = serde_json::from_value(value)?;
        let mut guard = self.inner.blocking_write();
        executor::block_on(guard.set(new_value))
    }

    fn constraints_json(&self) -> JsonValue {
        let guard = self.inner.blocking_read();
        serde_json::to_value(guard.constraints()).unwrap_or(JsonValue::Null)
    }
}

async fn poll_loop(
    serial: SerialHandle,
    data_tx: broadcast::Sender<Measurement>,
    id: String,
    poll_param: Arc<RwLock<Parameter<f64>>>,
    units_param: Arc<RwLock<Parameter<String>>>,
    mut shutdown_rx: oneshot::Receiver<()>,
) {
    loop {
        let poll_interval = {
            let guard = poll_param.read().await;
            Duration::from_secs_f64(1.0 / guard.get().clamp(0.1, 200.0))
        };

        let units_label = units_param.read().await.get();
        let measurement_unit = PowerUnits::from_str(&units_label)
            .map(|u| u.abbreviation().to_string())
            .unwrap_or(units_label.clone());

        tokio::select! {
            _ = &mut shutdown_rx => break,
            _ = tokio::time::sleep(poll_interval) => {
                match serial.query("PM:P?").await {
                    Ok(resp) => match resp.trim().parse::<f64>() {
                        Ok(value) => {
                            let measurement = Measurement::Scalar {
                                name: format!("{}_power", id),
                                value,
                                unit: measurement_unit.clone(),
                                timestamp: Utc::now(),
                            };
                            let _ = data_tx.send(measurement);
                        }
                        Err(err) => warn!("{}: failed to parse power '{}': {}", id, resp, err),
                    },
                    Err(err) => warn!("{}: power query failed: {}", id, err),
                }
            }
        }
    }
}

#[cfg(feature = "instrument_serial")]
fn build_serial_handle(config: &NewportConfig) -> Result<SerialHandle> {
    Ok(Arc::new(SerialPortDevice::new(
        config.port.clone(),
        config.baud_rate,
        Duration::from_millis(config.timeout_ms),
    )))
}

#[cfg(not(feature = "instrument_serial"))]
fn build_serial_handle(_config: &NewportConfig) -> Result<SerialHandle> {
    Err(anyhow!(
        "instrument_serial feature disabled - Newport1830CV3 cannot run without serial support"
    ))
}

#[cfg(feature = "instrument_serial")]
#[derive(Clone)]
struct SerialPortDevice {
    inner: Arc<SerialPortInner>,
}

#[cfg(feature = "instrument_serial")]
struct SerialPortInner {
    port_name: String,
    baud_rate: u32,
    timeout: Duration,
    port: Arc<std::sync::Mutex<Option<Box<dyn serialport::SerialPort + Send>>>>,
}

#[cfg(feature = "instrument_serial")]
impl SerialPortDevice {
    fn new(port_name: String, baud_rate: u32, timeout: Duration) -> Self {
        Self {
            inner: Arc::new(SerialPortInner {
                port_name,
                baud_rate,
                timeout,
                port: Arc::new(std::sync::Mutex::new(None)),
            }),
        }
    }
}

#[cfg(feature = "instrument_serial")]
#[async_trait]
impl SerialDevice for SerialPortDevice {
    async fn connect(&self) -> Result<()> {
        let port = serialport::new(&self.inner.port_name, self.inner.baud_rate)
            .timeout(self.inner.timeout)
            .open()
            .with_context(|| {
                format!(
                    "Failed to open serial port '{}' @ {} baud",
                    self.inner.port_name, self.inner.baud_rate
                )
            })?;
        *self.inner.port.lock().unwrap() = Some(port);
        Ok(())
    }

    async fn disconnect(&self) -> Result<()> {
        self.inner.port.lock().unwrap().take();
        Ok(())
    }

    async fn query(&self, command: &str) -> Result<String> {
        let command = format!("{}\r\n", command);
        let port = self.inner.port.clone();
        let timeout = self.inner.timeout;

        tokio::task::spawn_blocking(move || -> Result<String> {
            use std::io::{Read, Write};
            let mut guard = port
                .lock()
                .map_err(|_| anyhow!("Serial port mutex poisoned"))?;
            let port = guard
                .as_mut()
                .ok_or_else(|| anyhow!("Serial port not connected"))?;

            port.write_all(command.as_bytes())?;
            port.flush()?;
            debug!("Sent serial command: {}", command.trim());

            let start = std::time::Instant::now();
            let mut response = String::new();
            let mut byte = [0u8; 1];
            loop {
                if start.elapsed() > timeout {
                    return Err(anyhow!("Serial read timeout after {:?}", timeout));
                }

                match port.read(&mut byte) {
                    Ok(0) => continue,
                    Ok(_) => {
                        let ch = byte[0] as char;
                        response.push(ch);
                        if ch == '\n' {
                            break;
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::TimedOut => continue,
                    Err(e) => return Err(anyhow!("Serial read error: {}", e)),
                }
            }

            Ok(response.trim().to_string())
        })
        .await
        .context("Serial I/O join error")?
    }

    async fn send(&self, command: &str) -> Result<()> {
        let command = format!("{}\r\n", command);
        let port = self.inner.port.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            use std::io::Write;
            let mut guard = port
                .lock()
                .map_err(|_| anyhow!("Serial port mutex poisoned"))?;
            let port = guard
                .as_mut()
                .ok_or_else(|| anyhow!("Serial port not connected"))?;
            port.write_all(command.as_bytes())?;
            port.flush()?;
            debug!("Sent serial command: {}", command.trim());
            Ok(())
        })
        .await
        .context("Serial send join error")?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tokio::time::{timeout, Duration as TokioDuration};

    struct MockSerialTransport {
        state: Arc<Mutex<MockState>>,
    }

    struct MockState {
        connected: bool,
        commands: Vec<String>,
        responses: VecDeque<Result<String>>,
    }

    impl MockSerialTransport {
        fn new() -> Self {
            Self {
                state: Arc::new(Mutex::new(MockState {
                    connected: false,
                    commands: Vec::new(),
                    responses: VecDeque::new(),
                })),
            }
        }

        fn handle(&self) -> SerialHandle {
            Arc::new(self.clone()) as SerialHandle
        }

        async fn push_response<S: Into<String>>(&self, response: S) {
            self.state
                .lock()
                .await
                .responses
                .push_back(Ok(response.into()));
        }

        async fn push_error(&self, message: &str) {
            self.state
                .lock()
                .await
                .responses
                .push_back(Err(anyhow!(message.to_string())));
        }

        async fn commands(&self) -> Vec<String> {
            self.state.lock().await.commands.clone()
        }
    }

    impl Clone for MockSerialTransport {
        fn clone(&self) -> Self {
            Self {
                state: self.state.clone(),
            }
        }
    }

    #[async_trait]
    impl SerialDevice for MockSerialTransport {
        async fn connect(&self) -> Result<()> {
            self.state.lock().await.connected = true;
            Ok(())
        }

        async fn disconnect(&self) -> Result<()> {
            self.state.lock().await.connected = false;
            Ok(())
        }

        async fn query(&self, command: &str) -> Result<String> {
            let mut state = self.state.lock().await;
            if !state.connected {
                return Err(anyhow!("not connected"));
            }

            state.commands.push(command.to_string());
            state
                .responses
                .pop_front()
                .unwrap_or_else(|| Err(anyhow!("No mock response queued")))
        }

        async fn send(&self, command: &str) -> Result<()> {
            let mut state = self.state.lock().await;
            if !state.connected {
                return Err(anyhow!("not connected"));
            }
            state.commands.push(command.to_string());
            Ok(())
        }
    }

    fn test_config() -> NewportConfig {
        NewportConfig {
            port: "mock".into(),
            baud_rate: 9600,
            timeout_ms: 200,
            poll_hz: 5.0,
            wavelength_nm: 532.0,
            range: RangeSetting::Auto,
            units: PowerUnits::Watts,
        }
    }

    #[tokio::test]
    async fn initializes_and_configures() {
        let mock = MockSerialTransport::new();
        mock.push_response("NEWPORT,1830C").await;
        let mut instrument = instrument_with_mock(&mock);
        instrument.initialize().await.unwrap();

        let commands = mock.commands().await;
        assert_eq!(commands.first().unwrap(), "*IDN?");
        assert!(commands.iter().any(|c| c.starts_with("PM:Lambda")));
        assert!(commands.iter().any(|c| c.starts_with("PM:Range")));
        assert!(commands.iter().any(|c| c.starts_with("PM:Units")));
    }

    #[tokio::test]
    async fn power_meter_trait_methods_issue_commands() {
        let mock = MockSerialTransport::new();
        mock.push_response("NEWPORT").await;
        let mut instrument = instrument_with_mock(&mock);
        instrument.initialize().await.unwrap();

        instrument.set_wavelength(633.0).await.unwrap();
        instrument.set_range(0.05).await.unwrap();
        instrument.zero().await.unwrap();

        let commands = mock.commands().await;
        assert!(commands.iter().any(|c| c == "PM:Lambda 633.00"));
        assert!(commands.iter().any(|c| c == "PM:Range 3"));
        assert!(commands.iter().any(|c| c == "PM:DS:Clear"));
    }

    #[tokio::test]
    async fn lifecycle_transitions_update_state() {
        let mock = MockSerialTransport::new();
        mock.push_response("NEWPORT").await;
        let mut instrument = instrument_with_mock(&mock);
        assert_eq!(instrument.state(), InstrumentState::Uninitialized);

        instrument.initialize().await.unwrap();
        assert_eq!(instrument.state(), InstrumentState::Idle);

        instrument.execute(Command::Start).await.unwrap();
        assert_eq!(instrument.state(), InstrumentState::Running);

        instrument.execute(Command::Stop).await.unwrap();
        assert_eq!(instrument.state(), InstrumentState::Idle);

        instrument.shutdown().await.unwrap();
        assert_eq!(instrument.state(), InstrumentState::ShuttingDown);
    }

    #[tokio::test]
    async fn polling_emits_scalar_measurements() {
        let mock = MockSerialTransport::new();
        mock.push_response("NEWPORT").await;
        mock.push_response("0.001").await;
        mock.push_response("0.002").await;
        let mut instrument = instrument_with_mock(&mock);
        instrument.initialize().await.unwrap();

        let mut rx = instrument.data_channel();
        instrument.execute(Command::Start).await.unwrap();

        let measurement = timeout(TokioDuration::from_millis(700), rx.recv())
            .await
            .expect("measurement timeout")
            .unwrap();

        match measurement {
            Measurement::Scalar { value, unit, .. } => {
                assert_eq!(unit, "W");
                assert!(value >= 0.0);
            }
            _ => panic!("expected scalar measurement"),
        }

        instrument.execute(Command::Stop).await.unwrap();
        instrument.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn polling_recovers_from_parse_errors() {
        let mock = MockSerialTransport::new();
        mock.push_response("NEWPORT").await;
        mock.push_response("??").await; // parse error
        mock.push_response("0.004").await; // valid
        let mut instrument = instrument_with_mock(&mock);
        instrument.initialize().await.unwrap();
        let mut rx = instrument.data_channel();
        instrument.execute(Command::Start).await.unwrap();

        let measurement = timeout(TokioDuration::from_secs(1), rx.recv())
            .await
            .expect("measurement timeout")
            .unwrap();

        match measurement {
            Measurement::Scalar { value, .. } => {
                assert!((value - 0.004).abs() < f64::EPSILON);
            }
            _ => panic!("expected scalar measurement"),
        }

        instrument.execute(Command::Stop).await.unwrap();
        instrument.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn mock_serial_enforces_connection_state() {
        let mock = MockSerialTransport::new();
        assert!(mock.query("PM:P?").await.is_err());
        mock.connect().await.unwrap();
        mock.push_response("0.1").await;
        assert!(mock.query("PM:P?").await.is_ok());
        mock.disconnect().await.unwrap();
        assert!(!mock.state.lock().await.connected);
    }

    fn instrument_with_mock(mock: &MockSerialTransport) -> Newport1830CV3 {
        Newport1830CV3::with_mock("test", test_config(), mock.handle())
    }
}
