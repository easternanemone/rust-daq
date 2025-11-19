# Pillar 4: Build Scripting Layer - Task Breakdown

## P4.1: Define ScriptEngine Trait
type: task
priority: P0
parent: bd-kal8.4
description: |
  Create abstract ScriptEngine trait for pluggable scripting backends.

  TRAIT DEFINITION:
  - fn execute_script(&mut self, script: &str) -> Result<ScriptResult>
  - fn register_instrument(&mut self, name: &str, handle: InstrumentHandle)
  - fn register_function(&mut self, name: &str, callback: Box<dyn Fn>)

  ACCEPTANCE:
  - src/scripting/engine.rs created
  - ScriptEngine trait defined
  - ScriptResult enum for return values

## P4.2: Implement PyO3 ScriptEngine Backend
type: task
priority: P0
parent: bd-kal8.4
deps: bd-kal8.4.1
description: |
  Create Python scripting backend using PyO3.

  IMPLEMENTATION:
  - PyO3 embedded interpreter
  - Expose V3 Measurement types to Python
  - Expose instrument control methods
  - Handle async/await bridge (tokio → Python asyncio)

  EXAMPLE SCRIPT:
  ```python
  motor = instruments.get("esp300")
  power = instruments.get("newport")

  motor.move_absolute(45.0)
  reading = power.read_power()
  print(f"Power at 45°: {reading}")
  ```

  ACCEPTANCE:
  - PyO3ScriptEngine implements ScriptEngine trait
  - Example script runs successfully
  - Command latency < 50ms overhead

## P4.3: Create script_runner CLI Command
type: task
priority: P0
parent: bd-kal8.4
deps: bd-kal8.4.2
description: |
  Add CLI command to run experiment scripts.

  USAGE:
  cargo run -- script run experiment.py
  cargo run -- script validate experiment.py

  FEATURES:
  - Load config.toml
  - Initialize instruments
  - Run script
  - Save results to file

  ACCEPTANCE:
  - CLI command works
  - Scripts can control hardware
  - Error handling with clear messages

## P4.4: Expose V3 APIs to Python via PyO3 Bindings
type: task
priority: P0
parent: bd-kal8.4
deps: bd-kal8.4.2
description: |
  Create Python bindings for V3 instrument APIs.

  UNBLOCKS: bd-12 (Python bindings epic)

  BINDINGS:
  - Measurement enum → Python dataclass
  - InstrumentHandle → Python object
  - Async methods → Python coroutines

  EXAMPLE:
  ```python
  import rust_daq

  config = rust_daq.Config.from_file("config.toml")
  manager = rust_daq.InstrumentManager(config)

  motor = manager.get_instrument("esp300")
  await motor.move_absolute(90.0)
  pos = await motor.get_position()
  ```

  ACCEPTANCE:
  - Python module compiles
  - Example script works
  - Documentation generated

## P4.5: Implement Alternative Scripting Backend (Rhai/Lua)
type: task
priority: P1
parent: bd-kal8.4
deps: bd-kal8.4.1
description: |
  Create alternative scripting engine for lightweight scripting.

  OPTIONS:
  1. Rhai (pure Rust, fast)
  2. Lua (proven in scientific tools)

  EXAMPLE (Rhai):
  ```rhai
  let motor = instruments.get("esp300");
  let power = instruments.get("newport");

  motor.move_absolute(45.0);
  let reading = power.read_power();
  print(`Power at 45°: ${reading}`);
  ```

  ACCEPTANCE:
  - Alternative backend implements ScriptEngine trait
  - Example script runs
  - Comparison benchmarks vs PyO3
