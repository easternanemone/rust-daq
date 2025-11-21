# Testing Guide for Issue #4: Proto Enhancements

## Quick Verification

The enhanced protobuf definitions compile successfully:

```bash
cargo check --lib --features networking
# ✅ Compiles successfully
```

## New RPC Endpoints Available

### 1. ListScripts

**Purpose:** Get a list of all uploaded scripts with metadata

**Request:**
```protobuf
message ListScriptsRequest {
  // Empty
}
```

**Response:**
```protobuf
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

**Example usage (gRPC client):**
```rust
let request = tonic::Request::new(ListScriptsRequest {});
let response = client.list_scripts(request).await?;

for script in response.into_inner().scripts {
    println!("Script: {} ({})", script.name, script.script_id);
    println!("  Uploaded: {}", script.upload_time_ns);
}
```

### 2. ListExecutions

**Purpose:** Get filtered list of script executions

**Request:**
```protobuf
message ListExecutionsRequest {
  optional string script_id = 1;  // Filter by script ID
  optional string state = 2;      // Filter by state (RUNNING, COMPLETED, ERROR, STOPPED)
}
```

**Response:**
```protobuf
message ListExecutionsResponse {
  repeated ScriptStatus executions = 1;
}
```

**Example usage:**
```rust
// Get all running executions
let request = tonic::Request::new(ListExecutionsRequest {
    script_id: None,
    state: Some("RUNNING".to_string()),
});
let response = client.list_executions(request).await?;

// Get all executions for specific script
let request = tonic::Request::new(ListExecutionsRequest {
    script_id: Some("uuid-here".to_string()),
    state: None,
});
```

### 3. GetDaemonInfo

**Purpose:** Get daemon version, features, and capabilities

**Request:**
```protobuf
message DaemonInfoRequest {
  // Empty
}
```

**Response:**
```protobuf
message DaemonInfoResponse {
  string version = 1;
  repeated string features = 2;          // e.g., ["networking", "storage_hdf5"]
  repeated string available_hardware = 3; // e.g., ["MockStage", "MockCamera"]
  uint64 uptime_seconds = 4;
}
```

**Example usage:**
```rust
let request = tonic::Request::new(DaemonInfoRequest {});
let response = client.get_daemon_info(request).await?;
let info = response.into_inner();

println!("Daemon version: {}", info.version);
println!("Features: {:?}", info.features);
println!("Uptime: {} seconds", info.uptime_seconds);
```

## Enhanced Existing RPCs

### GetScriptStatus (Enhanced)

Now returns additional fields:
- `script_id` - Which script is running
- `progress_percent` - Progress estimate (0-100)
- `current_line` - Current line being executed

```rust
let status = client.get_script_status(StatusRequest {
    execution_id: "uuid".to_string(),
}).await?.into_inner();

println!("Script: {}", status.script_id);
println!("State: {}", status.state);
println!("Progress: {}%", status.progress_percent);
```

### StopScript (Enhanced)

Now supports force flag:

```rust
// Graceful stop
let response = client.stop_script(StopRequest {
    execution_id: "uuid".to_string(),
    force: false,
}).await?.into_inner();

// Force stop
let response = client.stop_script(StopRequest {
    execution_id: "uuid".to_string(),
    force: true,
}).await?.into_inner();

println!("Stopped: {}", response.stopped);
println!("Message: {}", response.message);
```

## Manual Testing Workflow

### 1. Start the daemon

```bash
cargo run --bin daemon --features networking
```

### 2. Use grpcurl for testing

```bash
# Install grpcurl if needed
brew install grpcurl

# List available services
grpcurl -plaintext localhost:50051 list

# Get daemon info
grpcurl -plaintext localhost:50051 daq.ControlService/GetDaemonInfo

# Upload a script
grpcurl -plaintext -d '{
  "script_content": "let x = 42;",
  "name": "test_script",
  "metadata": {"author": "test"}
}' localhost:50051 daq.ControlService/UploadScript

# List scripts
grpcurl -plaintext localhost:50051 daq.ControlService/ListScripts

# Start script (use script_id from upload response)
grpcurl -plaintext -d '{
  "script_id": "UUID_HERE"
}' localhost:50051 daq.ControlService/StartScript

# List executions
grpcurl -plaintext localhost:50051 daq.ControlService/ListExecutions

# List only running executions
grpcurl -plaintext -d '{
  "state": "RUNNING"
}' localhost:50051 daq.ControlService/ListExecutions

# Stop execution
grpcurl -plaintext -d '{
  "execution_id": "UUID_HERE",
  "force": false
}' localhost:50051 daq.ControlService/StopScript
```

## Integration with CLI Client

The CLI client (in development) will use these RPCs:

1. **list** command → `ListScripts` and `ListExecutions`
2. **status** command → `GetScriptStatus` (enhanced)
3. **stop** command → `StopScript` (with --force flag)
4. **info** command → `GetDaemonInfo`

## Expected Behavior

### ListScripts
- Returns empty list initially
- After uploads, returns all scripts with their metadata
- Preserves upload order (by timestamp)

### ListExecutions
- Returns empty list initially
- After executions, returns most recent first
- Filters work correctly (by script_id and/or state)
- Shows complete execution history

### GetDaemonInfo
- Returns current package version (from Cargo.toml)
- Lists compiled features (networking, storage_hdf5, storage_arrow)
- Reports available hardware (currently mocked)
- Uptime increases with server runtime

### StopScript
- Returns appropriate message for force vs graceful
- Cannot stop non-running executions
- Updates execution state to STOPPED
- Records end time

## Known Limitations

Current implementation has these TODOs:

1. **Script Cancellation:** Stop currently marks state as STOPPED but doesn't actually cancel the running task (requires tokio::task::JoinHandle integration)

2. **Progress Tracking:** Always reports 0% for running scripts, 100% for completed (requires script engine integration)

3. **Line Tracking:** Always empty string (requires script engine integration)

4. **Hardware Registry:** Returns mock hardware list (requires actual hardware discovery)

These are noted in the code with TODO comments for future enhancement.
