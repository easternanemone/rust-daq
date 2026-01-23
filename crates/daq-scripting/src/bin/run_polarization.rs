//! Run the polarization characterization experiment
use daq_scripting::RhaiEngine;
use daq_scripting::traits::ScriptEngine;

const SCRIPT: &str = include_str!("../../../daq-examples/examples/polarization_characterization.rhai");

#[tokio::main]
async fn main() {
    println!("Starting polarization characterization experiment...\n");

    let mut engine = RhaiEngine::with_hardware().expect("Failed to create RhaiEngine");

    match engine.execute_script(SCRIPT).await {
        Ok(result) => {
            println!("\n=== Experiment completed successfully ===");
            println!("Result: {:?}", result);
        }
        Err(e) => {
            eprintln!("\n=== Experiment failed ===");
            eprintln!("Error: {:?}", e);
            std::process::exit(1);
        }
    }
}
