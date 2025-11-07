# DAQ-28 Phase 2: Advanced Dynamic Configuration Features

## Context

This document specifies the Phase 2 advanced features for dynamic configuration support. Phase 1 (MVP) has been completed and provides basic runtime add/remove/update operations.

### MVP Implementation (Phase 1 - Completed)

**Commit:** `538db51` - feat(config): Add MVP dynamic configuration support

**Implemented:**
- `DaqCommand::AddInstrumentDynamic` - Spawn instrument from inline TOML config
- `DaqCommand::RemoveInstrumentDynamic` - Stop and remove with dependency warnings  
- `DaqCommand::UpdateInstrumentParameter` - Send parameter updates to running instruments
- Handler methods in `DaqManagerActor`: `add_instrument_dynamic()`, `remove_instrument_dynamic()`, `update_instrument_parameter()`
- Integration tests (8 passing tests in `tests/dynamic_config_test.rs`)

**MVP Limitations:**
- Configuration is NOT persisted - changes lost on restart
- No processor support for dynamically added instruments
- Dependency validation only logs warnings, doesn't enforce
- Parameter updates only support string values (`ParameterValue::String`)
- No transaction/rollback capability
- No TOML file hot-reload

### Files Modified in MVP
- `src/messages.rs` - DaqCommand enum extensions
- `src/app_actor.rs` - Handler method implementations  
- `tests/dynamic_config_test.rs` - Integration tests

---

## Phase 2 Requirements

Implement production-ready dynamic configuration with:
1. **TOML File Persistence** - Changes written back to config files
2. **Hot-Reload System** - Watch config files and reload on changes
3. **Transaction System** - Atomic updates with staging/validation/commit
4. **Dependency Tracking** - Enforce module-instrument relationships
5. **Configuration Versioning** - Snapshot-based rollback capability

---

## Feature 1: TOML File Persistence

### Overview
Extend dynamic operations to persist changes back to TOML configuration files using atomic writes.

### Requirements

**1.1 Persist Add Operations**
- When `add_instrument_dynamic()` succeeds, append instrument config to `config/default.toml`
- Use atomic write pattern: write to temp file → validate → rename  
- Preserve TOML formatting and comments where possible
- Add timestamp comment: `# Added dynamically: 2025-10-23T14:30:00Z`

**1.2 Persist Remove Operations**
- When `remove_instrument_dynamic()` succeeds, remove config from TOML
- Preserve surrounding structure and comments
- Add comment placeholder: `# Removed: <instrument_id> at <timestamp>`

**1.3 Persist Parameter Updates**
- When `update_instrument_parameter()` succeeds, update TOML value
- Maintain type information (convert string back to appropriate TOML type)
- Add inline comment documenting the change

**1.4 Error Handling**
- If TOML write fails, the operation succeeds in-memory but logs error
- Add `DaqCommand::SyncConfigToFile` to retry failed writes
- Return detailed errors for file permission, parse, or validation failures

### Technical Approach

```rust
// New module: src/config/persistence.rs

pub struct ConfigPersistence {
    config_path: PathBuf,
    write_lock: Arc<Mutex<()>>,
}

impl ConfigPersistence {
    /// Atomically add instrument to TOML file
    pub async fn add_instrument(&self, id: &str, config: &toml::Value) -> Result<()> {
        let _lock = self.write_lock.lock().await;
        
        // 1. Read current TOML
        let current = self.read_toml().await?;
        
        // 2. Modify in-memory
        let mut doc = current.as_table_mut()?.clone();
        doc["instruments"][id] = config.clone();
        
        // 3. Write to temp file
        let temp_path = self.config_path.with_extension("tmp");
        self.write_toml(&temp_path, &doc).await?;
        
        // 4. Validate temp file parses correctly
        Settings::try_from_file(&temp_path)?;
        
        // 5. Atomic rename
        tokio::fs::rename(&temp_path, &self.config_path).await?;
        
        Ok(())
    }
    
    /// Remove instrument from TOML file
    pub async fn remove_instrument(&self, id: &str) -> Result<()> { /* ... */ }
    
    /// Update parameter in TOML file  
    pub async fn update_parameter(&self, id: &str, param: &str, value: &toml::Value) -> Result<()> { /* ... */ }
}
```

### Dependencies
- `toml` crate (already in Cargo.toml)
- `toml_edit` crate for preserving formatting (add to Cargo.toml)

### Testing
- Test atomic write on permission error (should rollback)
- Test concurrent writes (lock should serialize)
- Test TOML parse error handling
- Test formatting preservation with comments
- Test recovery from partial writes (temp file cleanup)

---

## Feature 2: Hot-Reload System

### Overview
Watch config file for external changes and automatically reload Settings, spawning/stopping instruments as needed.

### Requirements

**2.1 File Watching**
- Use `notify` crate to watch `config/default.toml`
- Debounce rapid changes (500ms delay after last change)
- Ignore changes triggered by our own writes (track modification timestamps)

**2.2 Diff Calculation**
- Compare old vs new `Settings.instruments` HashMaps
- Identify: added instruments, removed instruments, modified parameters
- Generate `ConfigDiff` struct with change details

**2.3 Differential Application**
- For added instruments: spawn via existing `spawn_instrument()` logic
- For removed instruments: stop via existing `stop_instrument()` logic
- For modified parameters: send `InstrumentCommand::SetParameter`
- Process changes in dependency order (dependencies before dependents)

**2.4 Error Handling**
- If reload fails validation, log error but keep current config
- Emit `DaqEvent::ConfigReloadFailed` with error details
- Add `DaqCommand::GetLastReloadStatus` for GUI inspection

### Technical Approach

```rust
// New module: src/config/hot_reload.rs

use notify::{Watcher, RecursiveMode, Event};

pub struct ConfigWatcher {
    watcher: RecommendedWatcher,
    actor_tx: mpsc::Sender<DaqCommand>,
    last_modified: Arc<Mutex<SystemTime>>,
}

impl ConfigWatcher {
    pub fn start(config_path: PathBuf, actor_tx: mpsc::Sender<DaqCommand>) -> Result<Self> {
        let (tx, mut rx) = mpsc::channel(32);
        
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
            if let Ok(event) = res {
                let _ = tx.blocking_send(event);
            }
        })?;
        
        watcher.watch(&config_path, RecursiveMode::NonRecursive)?;
        
        // Spawn task to handle events
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                // Debounce and handle reload
                Self::handle_file_change(event, &actor_tx).await;
            }
        });
        
        Ok(Self { watcher, actor_tx, last_modified: Arc::new(Mutex::new(SystemTime::now())) })
    }
    
    async fn handle_file_change(event: Event, actor_tx: &mpsc::Sender<DaqCommand>) {
        // Wait 500ms for rapid changes to settle
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        // Load new config
        let new_settings = match Settings::try_from_file("config/default.toml") {
            Ok(s) => s,
            Err(e) => {
                error!("Config reload failed: {}", e);
                return;
            }
        };
        
        // Send reload command to actor
        let (cmd, rx) = DaqCommand::reload_config(new_settings);
        let _ = actor_tx.send(cmd).await;
        let _ = rx.await;
    }
}

// Add to DaqCommand enum:
ReloadConfig {
    new_settings: Settings,
    response: oneshot::Sender<Result<ConfigDiff>>,
},

// Add to DaqManagerActor:
async fn reload_config(&mut self, new_settings: Settings) -> Result<ConfigDiff> {
    let diff = ConfigDiff::calculate(&self.settings, &new_settings);
    
    // Apply added instruments
    for id in &diff.added {
        self.spawn_instrument(id).await?;
    }
    
    // Update modified parameters  
    for (id, params) in &diff.modified {
        for (param, value) in params {
            self.update_instrument_parameter(id, param, value).await?;
        }
    }
    
    // Remove deleted instruments
    for id in &diff.removed {
        self.stop_instrument(id).await?;
    }
    
    // Update settings reference
    self.settings = Arc::new(new_settings);
    
    Ok(diff)
}
```

### Dependencies
- `notify = "6.1"` - Cross-platform file watching
- Add to Cargo.toml: `notify = "6.1"`

### Testing
- Test file modification detection
- Test config parse error doesn't crash watcher
- Test rapid changes are debounced
- Test instruments are spawned/stopped correctly
- Test parameter updates are applied
- Test our own writes don't trigger reload loop

---

## Feature 3: Transaction System

### Overview
Implement staging → validation → commit workflow for complex multi-step configuration changes.

### Requirements

**3.1 Transaction API**
```rust
// New DaqCommand variants:
BeginTransaction { response: oneshot::Sender<Result<TransactionId>> }
StageInstrumentAdd { transaction_id: TransactionId, id: String, config: toml::Value, ... }
StageInstrumentRemove { transaction_id: TransactionId, id: String, ... }
StageParameterUpdate { transaction_id: TransactionId, id: String, param: String, value: String, ... }
ValidateTransaction { transaction_id: TransactionId, response: oneshot::Sender<Result<ValidationReport>> }
CommitTransaction { transaction_id: TransactionId, response: oneshot::Sender<Result<()>> }
RollbackTransaction { transaction_id: TransactionId, response: oneshot::Sender<Result<()>> }
```

**3.2 Transaction Lifecycle**
1. **Begin**: Create transaction ID, allocate staging area
2. **Stage**: Accumulate changes without applying
3. **Validate**: Check all constraints (dependencies, conflicts, resource limits)
4. **Commit**: Apply all changes atomically, persist to TOML
5. **Rollback**: Discard staged changes

**3.3 Validation Rules**
- Check instrument type exists in registry
- Verify no duplicate IDs
- Ensure dependencies exist before dependents
- Validate TOML structure matches schema
- Check resource availability (e.g., serial ports not in use)

**3.4 Atomicity Guarantees**
- All changes succeed or all fail (no partial application)
- If any step fails during commit, rollback all applied changes
- Transaction state survives actor restart (persist to disk)

### Technical Approach

```rust
// New module: src/config/transaction.rs

pub struct Transaction {
    id: TransactionId,
    staged_adds: HashMap<String, toml::Value>,
    staged_removes: HashSet<String>,
    staged_updates: HashMap<String, HashMap<String, String>>,
    created_at: SystemTime,
}

pub struct TransactionManager {
    active_transactions: HashMap<TransactionId, Transaction>,
    max_age: Duration,
}

impl TransactionManager {
    pub fn begin(&mut self) -> TransactionId {
        let id = TransactionId(Uuid::new_v4());
        let tx = Transaction { id, staged_adds: HashMap::new(), /* ... */ };
        self.active_transactions.insert(id, tx);
        id
    }
    
    pub fn stage_add(&mut self, id: TransactionId, instrument_id: String, config: toml::Value) -> Result<()> {
        let tx = self.active_transactions.get_mut(&id)?;
        tx.staged_adds.insert(instrument_id, config);
        Ok(())
    }
    
    pub async fn validate(&self, id: TransactionId, current_settings: &Settings, registry: &InstrumentRegistry) -> Result<ValidationReport> {
        let tx = self.active_transactions.get(&id)?;
        let mut report = ValidationReport::new();
        
        // Check for duplicate IDs
        for instrument_id in tx.staged_adds.keys() {
            if current_settings.instruments.contains_key(instrument_id) {
                report.errors.push(format!("Instrument '{}' already exists", instrument_id));
            }
        }
        
        // Check instrument types exist
        for (instrument_id, config) in &tx.staged_adds {
            let instrument_type = config.get("type").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("Missing type"))?;
            if registry.get(instrument_type).is_none() {
                report.errors.push(format!("Unknown instrument type '{}'", instrument_type));
            }
        }
        
        // Check dependencies...
        
        Ok(report)
    }
    
    pub async fn commit(&mut self, id: TransactionId, actor: &mut DaqManagerActor) -> Result<()> {
        let tx = self.active_transactions.remove(&id)?;
        
        // Checkpoint for rollback
        let checkpoint = self.create_checkpoint(actor).await?;
        
        // Apply all staged changes
        let result = self.apply_transaction(&tx, actor).await;
        
        if let Err(e) = result {
            // Rollback to checkpoint
            self.restore_checkpoint(checkpoint, actor).await?;
            return Err(e);
        }
        
        Ok(())
    }
}
```

### Dependencies
- `uuid = { version = "1.0", features = ["v4"] }` - Transaction IDs

### Testing
- Test multi-step transaction commit
- Test partial failure triggers rollback
- Test transaction timeout/expiry
- Test concurrent transactions are isolated
- Test validation catches all error cases
- Test checkpoint/restore works correctly

---

## Feature 4: Dependency Tracking

### Overview
Enforce module-instrument dependencies to prevent removing instruments that are in use.

### Requirements

**4.1 Dependency Graph**
- Build graph of module → instrument assignments
- Track which modules use which instruments and in what roles
- Update graph on `assign_instrument_to_module()` and `start_module()`

**4.2 Removal Validation**
- Before removing instrument, query dependency graph
- If instrument is assigned to any module, return error listing modules
- `force=true` flag bypasses check (with warning)

**4.3 Cascade Operations** (Optional)
- `remove_instrument_cascade()` - Also removes dependent modules
- Require explicit confirmation for cascade deletes

**4.4 GUI Integration**
- Provide `DaqCommand::GetInstrumentDependencies` for GUI display
- Show warning dialog before removing in-use instruments

### Technical Approach

```rust
// New module: src/config/dependencies.rs

pub struct DependencyGraph {
    // Maps instrument_id -> Set of (module_id, role)
    instrument_to_modules: HashMap<String, HashSet<(String, String)>>,
}

impl DependencyGraph {
    pub fn add_assignment(&mut self, module_id: &str, role: &str, instrument_id: &str) {
        self.instrument_to_modules
            .entry(instrument_id.to_string())
            .or_insert_with(HashSet::new)
            .insert((module_id.to_string(), role.to_string()));
    }
    
    pub fn remove_assignment(&mut self, module_id: &str, instrument_id: &str) {
        if let Some(modules) = self.instrument_to_modules.get_mut(instrument_id) {
            modules.retain(|(mid, _)| mid != module_id);
        }
    }
    
    pub fn get_dependents(&self, instrument_id: &str) -> Vec<(String, String)> {
        self.instrument_to_modules
            .get(instrument_id)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }
    
    pub fn can_remove(&self, instrument_id: &str) -> Result<(), Vec<String>> {
        let dependents = self.get_dependents(instrument_id);
        if dependents.is_empty() {
            Ok(())
        } else {
            let module_ids: Vec<String> = dependents.into_iter().map(|(mid, _)| mid).collect();
            Err(module_ids)
        }
    }
}

// Add to DaqManagerActor:
dependency_graph: DependencyGraph,

async fn remove_instrument_dynamic(&mut self, id: &str, force: bool) -> Result<()> {
    if !force {
        if let Err(dependents) = self.dependency_graph.can_remove(id) {
            return Err(anyhow!(
                "Cannot remove instrument '{}': in use by modules {:?}. Use force=true to override.",
                id, dependents
            ));
        }
    }
    
    // Proceed with removal...
    self.dependency_graph.remove_all(id); // Clean up graph
    self.stop_instrument(id).await?;
    Ok(())
}
```

### Testing
- Test assignment tracking
- Test removal blocked when instrument in use
- Test force removal bypasses check
- Test graph cleanup on module/instrument removal
- Test concurrent access to dependency graph

---

## Feature 5: Configuration Versioning

### Overview
Snapshot-based configuration versioning with rollback capability.

### Requirements

**5.1 Automatic Snapshots**
- Create snapshot before every config change
- Store in `.daq/config_versions/` directory
- Filename format: `config-{timestamp}-{short_hash}.toml`
- Keep last 10 snapshots by default (configurable)

**5.2 Manual Snapshots**
- `DaqCommand::CreateConfigSnapshot { label: Option<String> }`
- Labeled snapshots are exempt from auto-cleanup

**5.3 Rollback**
- `DaqCommand::ListConfigVersions` - Returns list with timestamps and labels
- `DaqCommand::RollbackToVersion { version_id: String }` - Restores snapshot and reloads

**5.4 Diff Viewer**
- `DaqCommand::CompareConfigVersions { version_a, version_b }` - Returns diff
- Use `similar` crate for line-by-line diff

### Technical Approach

```rust
// New module: src/config/versioning.rs

pub struct VersionManager {
    versions_dir: PathBuf,
    max_versions: usize,
}

impl VersionManager {
    pub async fn create_snapshot(&self, settings: &Settings, label: Option<String>) -> Result<VersionId> {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let hash = self.compute_hash(settings);
        
        let filename = if let Some(label) = label {
            format!("config-{}-{}-{}.toml", timestamp, hash, label)
        } else {
            format!("config-{}-{}.toml", timestamp, hash)
        };
        
        let path = self.versions_dir.join(&filename);
        
        // Serialize settings to TOML
        let toml_str = toml::to_string_pretty(settings)?;
        tokio::fs::write(&path, toml_str).await?;
        
        // Cleanup old snapshots
        self.cleanup_old_versions().await?;
        
        Ok(VersionId(filename))
    }
    
    pub async fn rollback(&self, version_id: &VersionId) -> Result<Settings> {
        let path = self.versions_dir.join(&version_id.0);
        let toml_str = tokio::fs::read_to_string(&path).await?;
        let settings: Settings = toml::from_str(&toml_str)?;
        Ok(settings)
    }
    
    pub async fn list_versions(&self) -> Result<Vec<VersionInfo>> {
        let mut versions = Vec::new();
        let mut entries = tokio::fs::read_dir(&self.versions_dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let metadata = entry.metadata().await?;
            let name = entry.file_name().to_string_lossy().to_string();
            
            versions.push(VersionInfo {
                id: VersionId(name.clone()),
                created_at: metadata.created()?,
                label: Self::extract_label(&name),
            });
        }
        
        versions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(versions)
    }
}
```

### Dependencies
- `similar = "2.3"` - Text diffing library

### Testing
- Test snapshot creation
- Test auto-cleanup keeps only N versions
- Test labeled snapshots are preserved
- Test rollback restores settings correctly
- Test diff generation works
- Test concurrent snapshot creation

---

## Implementation Plan

### Week 1: TOML Persistence & Hot-Reload
- Implement `ConfigPersistence` module
- Integrate atomic writes into handler methods
- Implement `ConfigWatcher` with `notify` crate
- Add `DaqCommand::ReloadConfig`
- Write tests for persistence and hot-reload

### Week 2: Transaction System
- Implement `Transaction` and `TransactionManager`
- Add transaction-related DaqCommand variants
- Implement validation rules
- Implement checkpoint/rollback for atomicity
- Write tests for transaction lifecycle

### Week 3: Dependency Tracking & Versioning
- Implement `DependencyGraph`
- Update `assign_instrument_to_module()` to track dependencies
- Enforce dependency checks in `remove_instrument_dynamic()`
- Implement `VersionManager` with snapshot creation
- Add rollback functionality
- Write tests for dependencies and versioning

### Week 4: Integration & Polish
- Integration testing across all features
- Performance testing (snapshot overhead, transaction latency)
- Documentation updates
- GUI integration (if applicable)
- Code review and refinements

---

## Success Criteria

### Functional Requirements
- [ ] All configuration changes persist to TOML files
- [ ] Hot-reload detects external changes within 1 second
- [ ] Transactions provide all-or-nothing semantics
- [ ] Dependency tracking prevents breaking module assignments
- [ ] Rollback restores previous configuration state
- [ ] All features have >90% test coverage

### Performance Requirements
- [ ] Snapshot creation <100ms for typical configs
- [ ] Transaction commit <500ms for 10 operations
- [ ] Hot-reload reload <200ms for typical configs
- [ ] Dependency graph queries <10ms

### Reliability Requirements
- [ ] Atomic writes never corrupt TOML files
- [ ] Failed transactions always rollback completely
- [ ] File watcher recovers from errors gracefully
- [ ] No data loss during concurrent operations

---

## Testing Strategy

### Unit Tests
- Each module has comprehensive unit tests
- Mock dependencies where appropriate
- Test error paths and edge cases

### Integration Tests
- End-to-end transaction workflows
- Hot-reload with actual file modifications
- Rollback scenarios
- Concurrent operations

### Performance Tests
- Benchmark snapshot creation
- Measure transaction overhead
- Profile hot-reload latency

### Chaos Tests
- Random config modifications
- Simulated file permission errors
- Concurrent transactions
- Rapid file changes

---

## Migration Path

### Phase 1 → Phase 2 Migration
1. Existing MVP code remains functional
2. New features are additive (no breaking changes)
3. Users can opt-in to persistence per-operation
4. Default behavior: persist=true for production, persist=false for tests

### Configuration
```toml
[application.dynamic_config]
enable_persistence = true
enable_hot_reload = true
enable_transactions = true
enable_versioning = true
snapshot_retention = 10
```

---

## Open Questions

1. Should hot-reload be opt-in or opt-out?
2. What should transaction timeout be? (suggest 5 minutes)
3. Should we support cross-file config (e.g., instrument configs in separate files)?
4. How to handle conflicts if file is modified during active transaction?
5. Should rollback trigger a full application restart or just config reload?

---

## References

- MVP Implementation: commit `538db51`
- Settings Structure: `src/config.rs`
- DaqCommand Enum: `src/messages.rs`
- DaqManagerActor: `src/app_actor.rs`
- Original Issue: daq-28 in beads tracker
