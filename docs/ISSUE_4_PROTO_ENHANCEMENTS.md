# Issue #4: Proto Definition Enhancements for CLI Client

## Summary

Enhanced the protobuf schema (`proto/daq.proto`) and gRPC server implementation (`src/grpc/server.rs`) to provide better observability and control for CLI clients.

## Changes Made

### 1. Enhanced ScriptStatus Message

**Location:** `proto/daq.proto`

Added detailed execution information:
- `script_id` (string) - Which script was executed
- `progress_percent` (uint32) - Estimated progress (0-100)
- `current_line` (string) - Current line being executed (if available)
- Updated state enum to include "STOPPED"

**Before:**
```protobuf
message ScriptStatus {
  string execution_id = 1;
  string state = 2; // PENDING, RUNNING, COMPLETED, ERROR
  string error_message = 3;
  uint64 start_time_ns = 4;
  uint64 end_time_ns = 5;
}
```

**After:**
```protobuf
message ScriptStatus {
  string execution_id = 1;
  string state = 2; // PENDING, RUNNING, COMPLETED, ERROR, STOPPED
  string error_message = 3;
  uint64 start_time_ns = 4;
  uint64 end_time_ns = 5;

  // Detailed execution info
  string script_id = 6;         // Which script was executed
  uint32 progress_percent = 7;  // Estimated progress (0-100)
  string current_line = 8;      // Current line being executed (if available)
}
```

### 2. Enhanced StopScript RPC

**Location:** `proto/daq.proto`

Added force flag for graceful vs immediate termination:

```protobuf
message StopRequest {
  string execution_id = 1;
  bool force = 2;  // If true, immediately kill; if false, try graceful stop
}

message StopResponse {
  bool stopped = 1;
  string message = 2;  // Explanation of stop result
}
```

**Server Implementation:** Updated to handle force flag and provide meaningful status messages (though actual cancellation is TODO for future enhancement).

### 3. New ListScripts RPC

**Location:** `proto/daq.proto`

List all uploaded scripts with metadata:

```protobuf
rpc ListScripts(ListScriptsRequest) returns (ListScriptsResponse);

message ListScriptsRequest {
  // Empty for now, could add filtering later
}

message ListScriptsResponse {
  repeated ScriptInfo scripts = 1;
}

message ScriptInfo {
  string script_id = 1;
  string name = 2;
  uint64 upload_time_ns = 3;
  map<string, string> metadata = 4;
}
```

**Server Implementation:** Tracks script metadata in `script_metadata: HashMap<String, ScriptMetadata>`.

### 4. New ListExecutions RPC

**Location:** `proto/daq.proto`

List all script executions with optional filtering:

```protobuf
rpc ListExecutions(ListExecutionsRequest) returns (ListExecutionsResponse);

message ListExecutionsRequest {
  optional string script_id = 1;  // Filter by script ID
  optional string state = 2;      // Filter by state
}

message ListExecutionsResponse {
  repeated ScriptStatus executions = 1;
}
```

**Server Implementation:**
- Filters by script_id and/or state
- Sorts by start time (most recent first)
- Returns full ScriptStatus for each execution

### 5. New GetDaemonInfo RPC

**Location:** `proto/daq.proto`

Get daemon version and capabilities:

```protobuf
rpc GetDaemonInfo(DaemonInfoRequest) returns (DaemonInfoResponse);

message DaemonInfoRequest {
  // Empty
}

message DaemonInfoResponse {
  string version = 1;
  repeated string features = 2;          // e.g., ["storage_hdf5", "networking"]
  repeated string available_hardware = 3; // e.g., ["Stage", "Camera"]
  uint64 uptime_seconds = 4;
}
```

**Server Implementation:**
- Returns `CARGO_PKG_VERSION` for version
- Lists compiled features (networking, storage_hdf5, storage_arrow)
- Reports available hardware (currently mocked)
- Tracks uptime since server start

## Server Implementation Details

### New Data Structures

```rust
/// Metadata about an uploaded script
#[derive(Clone)]
struct ScriptMetadata {
    name: String,
    upload_time: u64,
    metadata: HashMap<String, String>,
}

/// Enhanced ExecutionState
#[derive(Clone)]
struct ExecutionState {
    script_id: String,
    state: String,
    start_time: u64,
    end_time: Option<u64>,
    error: Option<String>,
    progress_percent: u32,      // NEW
    current_line: String,       // NEW
}

/// Enhanced DaqServer
pub struct DaqServer {
    script_host: Arc<RwLock<ScriptHost>>,
    scripts: Arc<RwLock<HashMap<String, String>>>,
    script_metadata: Arc<RwLock<HashMap<String, ScriptMetadata>>>, // NEW
    executions: Arc<RwLock<HashMap<String, ExecutionState>>>,
    start_time: SystemTime,  // NEW for uptime tracking
    // ... other fields
}
```

### RPC Implementation Summary

1. **upload_script** - Now stores metadata alongside script content
2. **start_script** - Initializes progress tracking fields
3. **stop_script** - Handles force flag, updates state to STOPPED
4. **get_script_status** - Returns enhanced status with progress
5. **list_scripts** (NEW) - Returns all scripts with metadata
6. **list_executions** (NEW) - Returns filtered execution list
7. **get_daemon_info** (NEW) - Returns daemon info and capabilities

## Backward Compatibility

All changes maintain backward compatibility:
- New fields are optional in proto3
- Existing RPCs retain their original signatures
- New RPCs are additions, not replacements

## Compilation

```bash
# Library compiles cleanly with networking feature
cargo check --lib --features networking
```

Status: ✅ **Successfully compiles**

## Future Enhancements (TODOs in Code)

1. **Script Cancellation:** Implement actual tokio::task::JoinHandle-based cancellation in stop_script
2. **Progress Tracking:** Integrate with script engine to report actual progress
3. **Line Tracking:** Report current line being executed from script engine
4. **Hardware Registry:** Query actual hardware capabilities instead of mock list

## Testing

Existing tests updated to use enhanced types. All compilation checks pass with only unrelated warnings in other modules.

## Files Modified

1. `/Users/briansquires/code/rust-daq/proto/daq.proto` - Protocol definitions
2. `/Users/briansquires/code/rust-daq/src/grpc/server.rs` - Server implementation
3. Auto-generated: `target/debug/build/rust_daq-.../out/daq.rs` (via build.rs)
