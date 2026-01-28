//! Run the 4D waveplate calibration TEST (small parameters)
use scripting::traits::ScriptEngine;
use scripting::RhaiEngine;
use tracing_subscriber::EnvFilter;

const SCRIPT: &str = include_str!("../../../examples/examples/waveplate_calibration_4d_test.rhai");

const MAX_OPERATIONS: u64 = 1_000_000;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    println!("Starting 4D waveplate calibration TEST...\n");
    println!("  This is a quick test with small parameters");
    println!("  2 wavelengths × 3 LP × 2 HWP × 2 QWP = 24 points");
    println!();

    let mut engine =
        RhaiEngine::with_hardware_and_limit(MAX_OPERATIONS).expect("Failed to create RhaiEngine");

    match engine.execute_script(SCRIPT).await {
        Ok(result) => {
            println!("\n=== Test completed successfully ===");
            println!("Result: {:?}", result);
        }
        Err(e) => {
            eprintln!("\n=== Test failed ===");
            eprintln!("Error: {:?}", e);
            std::process::exit(1);
        }
    }
}
