//! gRPC client for communicating with the DAQ daemon.

use anyhow::Result;
use tonic::transport::Channel;

use daq_proto::daq::{
    control_service_client::ControlServiceClient,
    hardware_service_client::HardwareServiceClient,
    scan_service_client::ScanServiceClient,
    storage_service_client::StorageServiceClient,
    module_service_client::ModuleServiceClient,
    // Request/Response types
    DaemonInfoRequest, ListDevicesRequest, ListScansRequest, ListScriptsRequest,
    ListExecutionsRequest, MoveRequest, ReadValueRequest, DeviceStateRequest,
    // Storage types
    GetStorageConfigRequest, GetRecordingStatusRequest, StartRecordingRequest,
    StopRecordingRequest, ListAcquisitionsRequest,
    // Module types
    ListModuleTypesRequest, ListModulesRequest, CreateModuleRequest,
    StartModuleRequest, StopModuleRequest, AssignDeviceRequest,
    // Scan types
    CreateScanRequest, StartScanRequest, StopScanRequest, PauseScanRequest,
    ResumeScanRequest, ScanConfig,
};

/// gRPC client wrapper for the DAQ daemon
#[derive(Clone)]
pub struct DaqClient {
    control: ControlServiceClient<Channel>,
    hardware: HardwareServiceClient<Channel>,
    scan: ScanServiceClient<Channel>,
    storage: StorageServiceClient<Channel>,
    module: ModuleServiceClient<Channel>,
}

impl DaqClient {
    /// Connect to the DAQ daemon at the given address
    pub async fn connect(address: &str) -> Result<Self> {
        let channel = Channel::from_shared(address.to_string())?
            .connect()
            .await?;

        Ok(Self {
            control: ControlServiceClient::new(channel.clone()),
            hardware: HardwareServiceClient::new(channel.clone()),
            scan: ScanServiceClient::new(channel.clone()),
            storage: StorageServiceClient::new(channel.clone()),
            module: ModuleServiceClient::new(channel),
        })
    }

    /// Get daemon information (version, capabilities, etc.)
    pub async fn get_daemon_info(&mut self) -> Result<daq_proto::daq::DaemonInfoResponse> {
        let response = self.control.get_daemon_info(DaemonInfoRequest {}).await?;
        Ok(response.into_inner())
    }

    // =========================================================================
    // Hardware Service
    // =========================================================================

    /// List all devices
    pub async fn list_devices(&mut self) -> Result<Vec<daq_proto::daq::DeviceInfo>> {
        let response = self.hardware.list_devices(ListDevicesRequest {
            capability_filter: None,
        }).await?;
        Ok(response.into_inner().devices)
    }

    /// Get device state
    pub async fn get_device_state(&mut self, device_id: &str) -> Result<daq_proto::daq::DeviceStateResponse> {
        let response = self.hardware.get_device_state(DeviceStateRequest {
            device_id: device_id.to_string(),
        }).await?;
        Ok(response.into_inner())
    }

    /// Move device to absolute position
    pub async fn move_absolute(&mut self, device_id: &str, position: f64) -> Result<daq_proto::daq::MoveResponse> {
        let response = self.hardware.move_absolute(MoveRequest {
            device_id: device_id.to_string(),
            value: position,
            wait_for_completion: Some(false),
            timeout_ms: None,
        }).await?;
        Ok(response.into_inner())
    }

    /// Move device by relative amount
    pub async fn move_relative(&mut self, device_id: &str, distance: f64) -> Result<daq_proto::daq::MoveResponse> {
        let response = self.hardware.move_relative(MoveRequest {
            device_id: device_id.to_string(),
            value: distance,
            wait_for_completion: Some(false),
            timeout_ms: None,
        }).await?;
        Ok(response.into_inner())
    }

    /// Read value from device
    pub async fn read_value(&mut self, device_id: &str) -> Result<daq_proto::daq::ReadValueResponse> {
        let response = self.hardware.read_value(ReadValueRequest {
            device_id: device_id.to_string(),
        }).await?;
        Ok(response.into_inner())
    }

    // =========================================================================
    // Control Service (Scripts)
    // =========================================================================

    /// List all scripts
    pub async fn list_scripts(&mut self) -> Result<Vec<daq_proto::daq::ScriptInfo>> {
        let response = self.control.list_scripts(ListScriptsRequest {}).await?;
        Ok(response.into_inner().scripts)
    }

    /// List all executions
    pub async fn list_executions(&mut self) -> Result<Vec<daq_proto::daq::ScriptStatus>> {
        let response = self.control.list_executions(ListExecutionsRequest {
            script_id: None,
            state: None,
        }).await?;
        Ok(response.into_inner().executions)
    }

    // =========================================================================
    // Scan Service
    // =========================================================================

    /// List all scans
    pub async fn list_scans(&mut self) -> Result<Vec<daq_proto::daq::ScanStatus>> {
        let response = self.scan.list_scans(ListScansRequest {
            state_filter: None,
        }).await?;
        Ok(response.into_inner().scans)
    }

    /// Create a new scan
    pub async fn create_scan(&mut self, config: ScanConfig) -> Result<daq_proto::daq::CreateScanResponse> {
        let response = self.scan.create_scan(CreateScanRequest {
            config: Some(config),
        }).await?;
        Ok(response.into_inner())
    }

    /// Start a scan
    pub async fn start_scan(&mut self, scan_id: &str) -> Result<daq_proto::daq::StartScanResponse> {
        let response = self.scan.start_scan(StartScanRequest {
            scan_id: scan_id.to_string(),
        }).await?;
        Ok(response.into_inner())
    }

    /// Pause a scan
    pub async fn pause_scan(&mut self, scan_id: &str) -> Result<daq_proto::daq::PauseScanResponse> {
        let response = self.scan.pause_scan(PauseScanRequest {
            scan_id: scan_id.to_string(),
        }).await?;
        Ok(response.into_inner())
    }

    /// Resume a scan
    pub async fn resume_scan(&mut self, scan_id: &str) -> Result<daq_proto::daq::ResumeScanResponse> {
        let response = self.scan.resume_scan(ResumeScanRequest {
            scan_id: scan_id.to_string(),
        }).await?;
        Ok(response.into_inner())
    }

    /// Stop a scan
    pub async fn stop_scan(&mut self, scan_id: &str, emergency: bool) -> Result<daq_proto::daq::StopScanResponse> {
        let response = self.scan.stop_scan(StopScanRequest {
            scan_id: scan_id.to_string(),
            emergency_stop: emergency,
        }).await?;
        Ok(response.into_inner())
    }

    // =========================================================================
    // Storage Service
    // =========================================================================

    /// Get storage configuration
    pub async fn get_storage_config(&mut self) -> Result<daq_proto::daq::StorageConfig> {
        let response = self.storage.get_storage_config(GetStorageConfigRequest {}).await?;
        Ok(response.into_inner())
    }

    /// Get recording status
    pub async fn get_recording_status(&mut self) -> Result<daq_proto::daq::RecordingStatus> {
        let response = self.storage.get_recording_status(GetRecordingStatusRequest {
            recording_id: None,
        }).await?;
        Ok(response.into_inner())
    }

    /// Start recording
    pub async fn start_recording(&mut self, name: &str) -> Result<daq_proto::daq::StartRecordingResponse> {
        let response = self.storage.start_recording(StartRecordingRequest {
            name: name.to_string(),
            metadata: Default::default(),
            config_override: None,
            scan_id: None,
            run_uid: None,
        }).await?;
        Ok(response.into_inner())
    }

    /// Stop recording
    pub async fn stop_recording(&mut self) -> Result<daq_proto::daq::StopRecordingResponse> {
        let response = self.storage.stop_recording(StopRecordingRequest {
            recording_id: None,
            final_metadata: Default::default(),
        }).await?;
        Ok(response.into_inner())
    }

    /// List acquisitions
    pub async fn list_acquisitions(&mut self) -> Result<Vec<daq_proto::daq::AcquisitionSummary>> {
        let response = self.storage.list_acquisitions(ListAcquisitionsRequest {
            name_pattern: None,
            after_timestamp_ns: None,
            before_timestamp_ns: None,
            limit: Some(100),
            offset: None,
        }).await?;
        Ok(response.into_inner().acquisitions)
    }

    // =========================================================================
    // Module Service
    // =========================================================================

    /// List module types
    pub async fn list_module_types(&mut self) -> Result<Vec<daq_proto::daq::ModuleTypeSummary>> {
        let response = self.module.list_module_types(ListModuleTypesRequest {
            required_capability: None,
        }).await?;
        Ok(response.into_inner().module_types)
    }

    /// List module instances
    pub async fn list_modules(&mut self) -> Result<Vec<daq_proto::daq::ModuleStatus>> {
        let response = self.module.list_modules(ListModulesRequest {
            type_filter: None,
            state_filter: None,
        }).await?;
        Ok(response.into_inner().modules)
    }

    /// Create a module instance
    pub async fn create_module(&mut self, type_id: &str, name: &str) -> Result<daq_proto::daq::CreateModuleResponse> {
        let response = self.module.create_module(CreateModuleRequest {
            type_id: type_id.to_string(),
            instance_name: name.to_string(),
            initial_config: Default::default(),
        }).await?;
        Ok(response.into_inner())
    }

    /// Start a module
    pub async fn start_module(&mut self, module_id: &str) -> Result<daq_proto::daq::StartModuleResponse> {
        let response = self.module.start_module(StartModuleRequest {
            module_id: module_id.to_string(),
        }).await?;
        Ok(response.into_inner())
    }

    /// Stop a module
    pub async fn stop_module(&mut self, module_id: &str) -> Result<daq_proto::daq::StopModuleResponse> {
        let response = self.module.stop_module(StopModuleRequest {
            module_id: module_id.to_string(),
            force: false,
        }).await?;
        Ok(response.into_inner())
    }

    /// Assign device to module role
    pub async fn assign_device(&mut self, module_id: &str, role_id: &str, device_id: &str) -> Result<daq_proto::daq::AssignDeviceResponse> {
        let response = self.module.assign_device(AssignDeviceRequest {
            module_id: module_id.to_string(),
            role_id: role_id.to_string(),
            device_id: device_id.to_string(),
        }).await?;
        Ok(response.into_inner())
    }
}
