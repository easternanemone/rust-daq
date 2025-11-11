use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use rust_daq::app_actor::DaqManagerActor;
use rust_daq::config::{
    ApplicationSettings, DataDistributorSettings, Settings, StorageSettings, TimeoutSettings,
};
use rust_daq::data::registry::ProcessorRegistry;
use rust_daq::instrument::InstrumentRegistry;
use rust_daq::instrument::InstrumentRegistryV2;
use rust_daq::measurement::{
    instrument_measurement::InstrumentMeasurement, DataDistributor, DataDistributorConfig,
    SubscriberMetricsSnapshot,
};
use rust_daq::messages::DaqCommand;
use rust_daq::modules::ModuleRegistry;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

#[tokio::test]
async fn test_get_metrics_command_response_structure() {
    let harness = TestHarness::new(4).await;

    let (cmd, rx) = DaqCommand::subscribe_to_data();
    harness.cmd_tx.send(cmd).await.unwrap();
    let _data_rx = rx.await.expect("subscription response");

    let (cmd, rx) = DaqCommand::get_metrics();
    harness.cmd_tx.send(cmd).await.unwrap();
    let metrics = rx.await.expect("metrics response");

    assert_eq!(metrics.len(), 1);
    let snapshot = &metrics[0];
    assert_eq!(snapshot.subscriber, "dynamic_subscriber");
    assert_eq!(snapshot.channel_capacity, 4);
}

#[tokio::test]
async fn test_metrics_snapshot_accuracy() {
    let distributor = build_distributor(1);
    let mut _receiver = distributor.subscribe("snapshot_accuracy").await;

    distributor.broadcast(Arc::new(1_u32)).await.unwrap();
    distributor.broadcast(Arc::new(2_u32)).await.unwrap();
    distributor.broadcast(Arc::new(3_u32)).await.unwrap();

    let snapshot = single_snapshot(&distributor, "snapshot_accuracy").await;
    assert_eq!(snapshot.total_sent, 2);
    assert_eq!(snapshot.total_dropped, 1);
    assert!((snapshot.drop_rate_percent - 33.333).abs() < 0.1);
}

#[tokio::test]
async fn test_drop_rate_calculation() {
    let distributor = build_distributor(2);
    let mut _receiver = distributor.subscribe("drop_rate").await;

    for _ in 0..4 {
        distributor.broadcast(Arc::new(42_u32)).await.unwrap();
    }

    let snapshot = single_snapshot(&distributor, "drop_rate").await;
    assert_eq!(snapshot.total_sent + snapshot.total_dropped, 4);
    assert!(snapshot.drop_rate_percent >= 25.0);
}

#[tokio::test]
async fn test_channel_occupancy_reporting() {
    let distributor = build_distributor(4);
    let mut _receiver = distributor.subscribe("occupancy").await;

    distributor.broadcast(Arc::new(7_u32)).await.unwrap();
    distributor.broadcast(Arc::new(8_u32)).await.unwrap();
    distributor.broadcast(Arc::new(9_u32)).await.unwrap();

    let snapshot = single_snapshot(&distributor, "occupancy").await;
    assert_eq!(snapshot.channel_occupancy, 3);
    assert_eq!(snapshot.channel_capacity, 4);
}

struct TestHarness {
    cmd_tx: mpsc::Sender<DaqCommand>,
    _runtime: Arc<Runtime>,
}

impl TestHarness {
    async fn new(capacity: usize) -> Self {
        let settings = build_settings(capacity);
        let runtime = Arc::new(Runtime::new().expect("runtime"));
        let actor = DaqManagerActor::new(
            settings,
            Arc::new(InstrumentRegistry::<InstrumentMeasurement>::new()),
            Arc::new(InstrumentRegistryV2::new()),
            Arc::new(ProcessorRegistry::new()),
            Arc::new(ModuleRegistry::<InstrumentMeasurement>::new()),
            runtime.clone(),
        )
        .expect("actor");

        let (cmd_tx, cmd_rx) = mpsc::channel(32);
        runtime.clone().spawn(actor.run(cmd_rx));

        Self {
            cmd_tx,
            _runtime: runtime,
        }
    }
}

fn build_settings(capacity: usize) -> Settings {
    Settings {
        log_level: "info".to_string(),
        application: ApplicationSettings {
            broadcast_channel_capacity: 8,
            command_channel_capacity: 8,
            data_distributor: DataDistributorSettings {
                subscriber_capacity: capacity,
                warn_drop_rate_percent: 50.0,
                error_saturation_percent: 90.0,
                metrics_window_secs: 1,
            },
            timeouts: TimeoutSettings::default(),
        },
        storage: StorageSettings {
            default_path: "./data".to_string(),
            default_format: "csv".to_string(),
        },
        instruments: HashMap::new(),
        processors: None,
        instruments_v3: Vec::new(),
    }
}

fn build_distributor(capacity: usize) -> Arc<DataDistributor<Arc<u32>>> {
    let config =
        DataDistributorConfig::with_thresholds(capacity, 50.0, 90.0, Duration::from_secs(1));
    Arc::new(DataDistributor::with_config(config))
}

async fn single_snapshot(
    distributor: &Arc<DataDistributor<Arc<u32>>>,
    name: &str,
) -> SubscriberMetricsSnapshot {
    let metrics = distributor.metrics_snapshot().await;
    metrics
        .into_iter()
        .find(|s| s.subscriber == name)
        .expect("subscriber metrics")
}
