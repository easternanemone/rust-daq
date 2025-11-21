# ScriptEngine Trait Integration (bd-xagb)

## Summary

Successfully integrated the ScriptEngine trait into the CLI, replacing legacy ScriptHost usage. The CLI now uses the modern, backend-agnostic ScriptEngine interface while maintaining full backward compatibility with existing Rhai scripts.

## Changes Made

### 1. Updated `src/main.rs`

**Before:**
- Used legacy `ScriptHost::with_hardware()`
- Directly manipulated Rhai Engine and Scope
- Tightly coupled to Rhai implementation

**After:**
- Uses `ScriptEngine` trait with `RhaiEngine::with_hardware()`
- Backend-agnostic interface via trait methods
- Hardware registered via `set_global()` method
- Clean separation between CLI and scripting backend

```rust
// New implementation
let mut engine = RhaiEngine::with_hardware()?;
engine.set_global("stage", ScriptValue::new(StageHandle { ... }))?;
engine.set_global("camera", ScriptValue::new(CameraHandle { ... }))?;
let result = engine.execute_script(&script_content).await?;
```

### 2. Added `RhaiEngine::with_hardware()` Constructor

New constructor in `src/scripting/rhai_engine.rs` that:
- Pre-registers all hardware bindings (stage, camera methods)
- Includes comprehensive documentation with examples
- Lists all available hardware methods for scripts
- Maintains safety limits (10,000 operation limit)

**Hardware Methods Available:**
- **Stage:** `move_abs()`, `move_rel()`, `position()`, `wait_settled()`
- **Camera:** `arm()`, `trigger()`, `resolution()`
- **Utility:** `sleep(seconds)`

### 3. Improved `register_function()` Documentation

**Problem:** Rhai requires compile-time type information for function registration via `Engine::register_fn()`. The generic `ScriptEngine::register_function()` interface cannot support this without macros.

**Solution:** Enhanced error message that:
- Explains the compile-time limitation clearly
- Provides three alternative solutions
- Shows example custom constructor code
- Directs users to `with_hardware()` for common use cases

**Error Message Includes:**
1. Use `RhaiEngine::with_hardware()` for hardware bindings
2. Create custom constructors that call `Engine::register_fn()` before Arc::new()
3. Use PyO3Engine for runtime function registration

### 4. Enhanced Type Conversion

Updated `script_value_to_dynamic()` to support hardware types:
- `StageHandle` - for motion control devices
- `CameraHandle` - for camera/triggerable devices
- All basic types (i64, f64, bool, String)
- Rhai Dynamic values

### 5. Added Comprehensive Tests

New tests in `src/scripting/rhai_engine.rs`:
- `test_register_function_error_is_informative` - Verifies helpful error message
- `test_with_hardware_constructor` - Tests hardware method availability

All existing tests continue to pass (100/100).

## Testing

### Manual Testing
```bash
# Test stage control
cargo run --bin rust_daq -- run examples/simple_scan.rhai

# Test camera triggering
cargo run --bin rust_daq -- run examples/triggered_acquisition.rhai
```

Both scripts execute successfully with the new ScriptEngine interface.

### Automated Testing
```bash
# All library tests pass
cargo test --lib
# Result: 100 passed; 0 failed

# Specific RhaiEngine tests
cargo test --lib rhai_engine::tests
# Result: 11 passed; 0 failed
```

## Architecture Benefits

### Backend Agnostic
The CLI is no longer tied to Rhai. Future backends (Python/PyO3, Lua, JavaScript) can be swapped by changing one line:
```rust
// Switch to Python backend
let mut engine = PyO3Engine::with_hardware()?;
```

### Trait-Based Design
All scripting operations go through the `ScriptEngine` trait:
- `execute_script(script)` - Run scripts
- `validate_script(script)` - Check syntax
- `set_global(name, value)` - Set variables
- `get_global(name)` - Retrieve variables
- `clear_globals()` - Reset state
- `register_function(name, fn)` - Add custom functions (backend-dependent)

### Type Safety
`ScriptValue` wraps `Box<dyn Any + Send + Sync>` for safe cross-boundary data transfer:
```rust
let value = ScriptValue::new(42_i64);
let number: i64 = value.downcast().unwrap();
```

### Async-First
All execution is async-compatible via tokio:
```rust
let result = engine.execute_script(script).await?;
```

## Migration Path

### For Existing Code
**Legacy ScriptHost still works** - The old API is preserved in `src/scripting/engine.rs` for backward compatibility.

### For New Code
Use the ScriptEngine trait:
```rust
use rust_daq::scripting::{ScriptEngine, RhaiEngine, ScriptValue};

let mut engine = RhaiEngine::with_hardware()?;
engine.set_global("stage", ScriptValue::new(stage_handle))?;
engine.execute_script("stage.move_abs(10.0);").await?;
```

## Function Registration Limitation

### The Problem
Rhai's `Engine::register_fn()` uses generics:
```rust
engine.register_fn("add", |x: i64, y: i64| x + y);
```

This requires compile-time type information that cannot be preserved through `Box<dyn Any>`.

### The Workaround
For hardware bindings, use `RhaiEngine::with_hardware()`.

For custom functions, create a specialized constructor:
```rust
impl RhaiEngine {
    pub fn with_custom_functions() -> Result<Self, ScriptError> {
        let mut engine = Engine::new();
        engine.on_progress(|count| ...); // Safety limits

        // Register custom functions
        engine.register_fn("my_function", |x: i64| x * 2);
        engine.register_fn("process_data", |data: String| {
            // Custom processing
        });

        Ok(Self {
            engine: Arc::new(engine),
            scope: Arc::new(Mutex::new(Scope::new()))
        })
    }
}
```

### Alternative: PyO3Engine
Python backends support runtime function registration without compile-time types:
```rust
let mut engine = PyO3Engine::new()?;
engine.register_function("my_function", Box::new(python_function))?;
```

## Examples

### Basic Scripting
```rust
use rust_daq::scripting::{ScriptEngine, RhaiEngine, ScriptValue};

let mut engine = RhaiEngine::new()?;
engine.set_global("wavelength", ScriptValue::new(800_i64))?;

let script = r#"
    print(`Wavelength: ${wavelength} nm`);
    wavelength * 2
"#;

let result = engine.execute_script(script).await?;
let value: i64 = result.downcast().unwrap();
println!("Result: {}", value); // 1600
```

### Hardware Control
```rust
use rust_daq::scripting::{ScriptEngine, RhaiEngine, ScriptValue, StageHandle};
use rust_daq::hardware::mock::MockStage;

let mut engine = RhaiEngine::with_hardware()?;
engine.set_global("stage", ScriptValue::new(StageHandle {
    driver: Arc::new(MockStage::new()),
}))?;

let script = r#"
    stage.move_abs(10.0);
    stage.wait_settled();
    let pos = stage.position();
    print(`Position: ${pos}mm`);
"#;

engine.execute_script(script).await?;
```

## Success Criteria

✅ **Task 1:** Read relevant files - Complete
✅ **Task 2:** Understand register_function limitation - Documented
✅ **Task 3:** Replace ScriptHost with ScriptEngine in main.rs - Complete
✅ **Task 4:** Fix/document RhaiEngine::register_function - Complete with helpful error
✅ **Task 5:** Add example documentation - Complete
✅ **Task 6:** Test with script examples - All scripts work
✅ **Task 7:** Commit changes - Ready for commit

## Next Steps

This implementation unblocks PR #105 (script_runner CLI) by providing a clean, trait-based scripting interface. The script_runner can now use `ScriptEngine` without depending on Rhai-specific APIs.

## Files Modified

- `src/main.rs` - CLI now uses ScriptEngine trait
- `src/scripting/rhai_engine.rs` - Added with_hardware(), improved error messages, enhanced type conversion
- All tests passing (100/100)

## Backward Compatibility

✅ All existing Rhai scripts continue to work
✅ Legacy ScriptHost API preserved
✅ No breaking changes to public API
✅ Examples run without modification
