# Phase 2: Scripting Engine (Weeks 3-4)

## Phase 2: Scripting Engine Epic
type: epic
priority: P0
parent: bd-oq51
description: |
  Implement Rhai scripting engine for hot-swappable experiment logic.

  OBJECTIVE: Run hardware loops without recompiling Rust.
  TIMELINE: Weeks 3-4
  PARALLELIZABLE: Tasks D, E can overlap; F depends on D+E

  SUCCESS CRITERIA:
  - Rhai engine embedded in rust-daq-core
  - Scientists can write .rhai scripts to control mock hardware
  - Scripts execute with < 50ms overhead per command
  - Safety: Infinite loops auto-terminate after 10000 operations
  - CLI: rust-daq run script.rhai works end-to-end

## Task D: Rhai Setup and Integration
type: task
priority: P0
parent: bd-oq51.2
description: |
  Add Rhai dependency and create ScriptHost wrapper.

  CARGO.TOML:
  ```toml
  [dependencies]
  rhai = { version = "1.16", features = ["sync", "only_i64"] }
  tokio = { version = "1", features = ["rt-multi-thread", "macros", "time"] }
  ```

  REFERENCE IMPLEMENTATION (src/scripting/engine.rs):
  ```rust
  use rhai::{Engine, Scope, Dynamic};
  use tokio::runtime::Handle;

  pub struct ScriptHost {
      engine: Engine,
      runtime: Handle,
  }

  impl ScriptHost {
      pub fn new(runtime: Handle) -> Self {
          let mut engine = Engine::new();

          // Safety: Limit operations to prevent infinite loops
          engine.on_progress(|count| {
              if count > 10000 {
                  Some("Safety limit exceeded".into())
              } else {
                  None
              }
          });

          Self { engine, runtime }
      }

      pub fn run_script(&self, script: &str, scope: &mut Scope)
          -> Result<Dynamic, Box<rhai::EvalAltResult>>
      {
          self.engine.eval_with_scope(scope, script)
      }
  }
  ```

  ACCEPTANCE:
  - src/scripting/engine.rs exists
  - ScriptHost struct compiles
  - Safety callback enforces operation limit
  - Unit test: infinite loop script terminates with error

## Task E: Hardware Bindings for Rhai
type: task
priority: P0
parent: bd-oq51.2
deps: bd-oq51.2.1
description: |
  Bridge async Rust hardware traits to synchronous Rhai scripts.

  CREATE: src/scripting/bindings.rs

  CRITICAL PATTERN (Sync→Async Bridge):
  ```rust
  use tokio::task::block_in_place;
  use tokio::runtime::Handle;

  /// Handle exposed to Rhai scripts
  #[derive(Clone)]
  pub struct StageHandle {
      pub driver: Arc<dyn Movable>,
  }

  pub fn register_hardware(engine: &mut Engine) {
      // Register custom type
      engine.register_type_with_name::<StageHandle>("Stage");

      // Register async methods (Bridging pattern)
      engine.register_fn("move_abs", |stage: &mut StageHandle, pos: f64| {
          // CRITICAL: block_in_place allows Rhai (sync) to wait for Rust (async)
          block_in_place(|| {
              Handle::current().block_on(stage.driver.move_abs(pos))
          }).unwrap()
      });

      engine.register_fn("position", |stage: &StageHandle| -> f64 {
          block_in_place(|| {
              Handle::current().block_on(stage.driver.position())
          }).unwrap()
      });
  }
  ```

  EXPOSED FUNCTIONS:
  - move_abs(stage, position)
  - move_rel(stage, distance)
  - position(stage) -> f64
  - trigger(camera)
  - sleep(seconds)

  ACCEPTANCE:
  - Rhai script can call stage.move_abs(10.0)
  - Script blocks until hardware completes
  - No panics from async/sync mismatch
  - Example script executes successfully

## Task F: CLI Rewrite for Script Execution
type: task
priority: P0
parent: bd-oq51.2
deps: bd-oq51.2.1,bd-oq51.2.2
description: |
  Rewrite src/main.rs to support daemon mode and script execution.

  CLI INTERFACE:
  ```bash
  # Run script once (for testing)
  rust-daq run experiment.rhai

  # Start daemon (for remote control)
  rust-daq daemon --port 50051

  # Run script with custom hardware config
  rust-daq run --config hardware.toml experiment.rhai
  ```

  IMPLEMENTATION (src/main.rs):
  ```rust
  use clap::{Parser, Subcommand};

  #[derive(Parser)]
  struct Cli {
      #[command(subcommand)]
      command: Commands,
  }

  #[derive(Subcommand)]
  enum Commands {
      /// Run a Rhai script once
      Run {
          /// Path to .rhai script file
          script: PathBuf,

          /// Optional hardware config
          #[arg(long)]
          config: Option<PathBuf>,
      },

      /// Start daemon for remote control
      Daemon {
          /// gRPC port
          #[arg(long, default_value = "50051")]
          port: u16,
      },
  }

  #[tokio::main]
  async fn main() -> Result<()> {
      let cli = Cli::parse();

      match cli.command {
          Commands::Run { script, config } => {
              let script_content = tokio::fs::read_to_string(script).await?;
              run_script_once(script_content, config).await
          },
          Commands::Daemon { port } => {
              start_daemon(port).await
          },
      }
  }
  ```

  EXAMPLE SCRIPT (examples/simple_scan.rhai):
  ```rhai
  // Simple stage scan experiment
  print("Starting scan...");

  for i in 0..10 {
      let pos = i * 1.0;
      stage.move_abs(pos);
      print(`Moved to ${pos}`);
      sleep(0.1);
  }

  print("Scan complete!");
  ```

  ACCEPTANCE:
  - rust-daq run examples/simple_scan.rhai executes
  - Script controls MockStage
  - Output shows movement logs
  - Script completes without errors
  - CLI help text displays correctly
