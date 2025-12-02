pub mod hardware_service;
pub mod module_service;
pub mod plugin_service;
pub mod preset_service;
pub mod scan_service;
pub mod storage_service;
/// gRPC server for remote DAQ control (Phase 3)
///
/// This module provides a gRPC server that exposes the DAQ system for remote control.
/// It allows clients to upload and execute Rhai scripts, monitor system status,
/// stream measurement data, and directly control hardware devices.
///
/// # Features
/// - Script upload and validation
/// - Remote script execution
/// - Status monitoring
/// - Live data streaming
/// - Direct hardware control (bd-4x6q)
/// - Coordinated multi-axis scanning (bd-4le6)
///
/// # Example
/// ```no_run
/// use rust_daq::grpc::start_server;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let addr = "127.0.0.1:50051".parse()?;
///     start_server(addr).await?;
///     Ok(())
/// }
/// ```
pub mod server;

/// Protocol Buffer definitions for the DAQ Control Service
#[allow(missing_docs)] // Auto-generated protobuf code
pub mod proto {
    //! Generated Protocol Buffer definitions from `proto/daq.proto`
    //!
    //! This module contains auto-generated code and provides:
    //! - `ControlService` trait for script management
    //! - `HardwareService` trait for direct device control (bd-4x6q)
    //! - `ScanService` trait for coordinated scans (bd-4le6)
    //! - Server and client implementations for all services
    //! - Request/Response message types for all RPC methods

    tonic::include_proto!("daq");
}

pub use hardware_service::HardwareServiceImpl;
pub use module_service::ModuleServiceImpl;
pub use plugin_service::PluginServiceImpl;
pub use preset_service::{default_preset_storage_path, PresetServiceImpl};
pub use scan_service::ScanServiceImpl;
pub use server::{start_server, start_server_with_hardware, DaqServer};
pub use storage_service::StorageServiceImpl;

// Re-export commonly used proto types - ControlService
pub use proto::control_service_client::ControlServiceClient;
pub use proto::control_service_server::{ControlService, ControlServiceServer};
pub use proto::{
    DataPoint, MeasurementRequest, ScriptStatus, StartRequest, StartResponse, StatusRequest,
    StopRequest, StopResponse, SystemStatus, UploadRequest, UploadResponse,
};

// Re-export HardwareService types (bd-4x6q)
pub use proto::hardware_service_client::HardwareServiceClient;
pub use proto::hardware_service_server::{HardwareService, HardwareServiceServer};
pub use proto::{
    DeviceInfo, DeviceMetadata, DeviceStateRequest, DeviceStateResponse, DeviceStateSubscribeRequest,
    DeviceStateUpdate, ListDevicesRequest, ListDevicesResponse, MoveRequest, MoveResponse,
    PositionUpdate, ReadValueRequest, ReadValueResponse, StopMotionRequest, StopMotionResponse,
    StreamValuesRequest, ValueUpdate,
    // Frame streaming types (bd-p6vz)
    StartStreamRequest, StartStreamResponse, StopStreamRequest, StopStreamResponse,
    StreamFramesRequest, FrameData,
    // Exposure control types (bd-tm0b)
    SetExposureRequest, SetExposureResponse, GetExposureRequest, GetExposureResponse,
    // Laser control types (bd-pwjo)
    SetShutterRequest, SetShutterResponse, GetShutterRequest, GetShutterResponse,
    SetWavelengthRequest, SetWavelengthResponse, GetWavelengthRequest, GetWavelengthResponse,
    SetEmissionRequest, SetEmissionResponse, GetEmissionRequest, GetEmissionResponse,
};

// Re-export ScanService types (bd-4le6)
pub use proto::scan_service_client::ScanServiceClient;
pub use proto::scan_service_server::{ScanService, ScanServiceServer};
pub use proto::{
    AxisConfig, CreateScanRequest, CreateScanResponse, GetScanStatusRequest, ListScansRequest,
    ListScansResponse, PauseScanRequest, PauseScanResponse, ResumeScanRequest, ResumeScanResponse,
    ScanConfig, ScanProgress, ScanState, ScanStatus, ScanType, StartScanRequest, StartScanResponse,
    StopScanRequest, StopScanResponse, StreamScanProgressRequest,
};

// Re-export PresetService types (bd-akcm)
pub use proto::preset_service_client::PresetServiceClient;
pub use proto::preset_service_server::{PresetService, PresetServiceServer};
pub use proto::{
    DeletePresetRequest, DeletePresetResponse, GetPresetRequest, ListPresetsRequest,
    ListPresetsResponse, LoadPresetRequest, LoadPresetResponse, Preset, PresetMetadata,
    SavePresetRequest, SavePresetResponse,
};

// Re-export ModuleService types (bd-xx7f)
pub use proto::module_service_client::ModuleServiceClient;
pub use proto::module_service_server::{ModuleService, ModuleServiceServer};
pub use proto::{
    // Module type discovery
    ListModuleTypesRequest, ListModuleTypesResponse, ModuleTypeSummary,
    GetModuleTypeInfoRequest, ModuleTypeInfo, ModuleRole, ModuleParameter,
    // Module lifecycle
    CreateModuleRequest, CreateModuleResponse, DeleteModuleRequest, DeleteModuleResponse,
    ListModulesRequest, ListModulesResponse, GetModuleStatusRequest, ModuleStatus, ModuleState,
    // Module configuration
    ConfigureModuleRequest, ConfigureModuleResponse, GetModuleConfigRequest, ModuleConfig,
    // Device assignment
    AssignDeviceRequest, AssignDeviceResponse, UnassignDeviceRequest, UnassignDeviceResponse,
    ListAssignmentsRequest, ListAssignmentsResponse, DeviceAssignment,
    // Module execution control
    StartModuleRequest, StartModuleResponse, PauseModuleRequest, PauseModuleResponse,
    ResumeModuleRequest, ResumeModuleResponse, StopModuleRequest, StopModuleResponse,
    // Module event streaming
    StreamModuleEventsRequest, ModuleEvent, ModuleEventSeverity,
    StreamModuleDataRequest, ModuleDataPoint,
};

// Re-export RunEngineService types (bd-niy4)
pub use proto::run_engine_service_client::RunEngineServiceClient;
pub use proto::run_engine_service_server::{RunEngineService, RunEngineServiceServer};
pub use proto::{
    // Plan type discovery
    ListPlanTypesRequest, ListPlanTypesResponse, PlanTypeSummary,
    GetPlanTypeInfoRequest, PlanTypeInfo, PlanParameter, PlanDeviceRole,
    // Plan execution
    QueuePlanRequest, QueuePlanResponse, StartEngineRequest, StartEngineResponse,
    PauseEngineRequest, PauseEngineResponse, ResumeEngineRequest, ResumeEngineResponse,
    AbortPlanRequest, AbortPlanResponse, HaltEngineRequest, HaltEngineResponse,
    GetEngineStatusRequest, EngineStatus, EngineState,
    // Document streaming
    StreamDocumentsRequest, Document, DocumentType,
    StartDocument, DescriptorDocument, EventDocument, StopDocument,
};

// Re-export StorageService types (bd-p6im)
pub use proto::storage_service_client::StorageServiceClient;
pub use proto::storage_service_server::{StorageService, StorageServiceServer};
pub use proto::{
    // Storage configuration
    ConfigureStorageRequest, ConfigureStorageResponse, GetStorageConfigRequest, StorageConfig,
    Hdf5Config,
    // Recording control
    StartRecordingRequest, StartRecordingResponse, StopRecordingRequest, StopRecordingResponse,
    GetRecordingStatusRequest, RecordingStatus, RecordingState,
    // Acquisition management
    ListAcquisitionsRequest, ListAcquisitionsResponse, AcquisitionSummary,
    GetAcquisitionInfoRequest, AcquisitionInfo, DatasetInfo, Hdf5Structure,
    DeleteAcquisitionRequest, DeleteAcquisitionResponse,
    // Data export
    FlushToStorageRequest, FlushToStorageResponse,
    StreamRecordingProgressRequest, RecordingProgress,
};

// Re-export PluginService types (bd-22si.6.1)
pub use proto::plugin_service_client::PluginServiceClient;
pub use proto::plugin_service_server::{PluginService, PluginServiceServer};
pub use proto::{
    // Plugin discovery
    ListPluginsRequest, ListPluginsResponse, PluginSummary,
    GetPluginInfoRequest, PluginInfo, PluginProtocol, PluginCapabilities,
    PluginReadable, PluginMovable, PluginAxis, PluginSettable,
    PluginSwitchable, PluginActionable, PluginLoggable, PluginScriptable,
    PluginUiElement,
    // Plugin instance management
    SpawnPluginRequest, SpawnPluginResponse,
    ListPluginInstancesRequest, ListPluginInstancesResponse, PluginInstanceSummary,
    GetPluginInstanceStatusRequest, PluginInstanceStatus,
    DestroyPluginInstanceRequest, DestroyPluginInstanceResponse,
};
