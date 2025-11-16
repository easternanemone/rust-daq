# Measurement Processor Developer Guide (V4 Architecture)

This guide outlines the development of data processors within the V4 architecture, leveraging `apache/arrow-rs` for data representation and integrating with the Kameo actor framework. Processors transform raw instrument data into more meaningful forms (e.g., filtering, FFT, statistical analysis).

## Core Principles in V4

1.  **Arrow-Native Data:** All measurements are represented as `arrow::record_batch::RecordBatch` instances. This provides high-performance, columnar data structures.
2.  **Actor-Based Processing:** Processors are typically implemented as dedicated Kameo actors or as methods within instrument actors, ensuring isolated, concurrent, and supervised execution.
3.  **Stream-Oriented:** Data flows through the system as streams of `RecordBatch`es.

## Quick Start: Implementing a Processor Actor

A measurement processor in V4 is a Kameo actor that receives `RecordBatch`es, performs transformations, and then publishes new `RecordBatch`es.

```rust
use kameo::{Actor, Context, Message};
use arrow::record_batch::RecordBatch;
use arrow::array::{Float64Array, UInt64Array};
use arrow::datatypes::{Schema, Field, DataType};
use std::sync::Arc;

// Define the message the processor actor will receive
#[derive(Message)]
pub struct ProcessData(pub RecordBatch);

pub struct MyProcessorActor {
    // Actor state, e.g., filter coefficients, previous data
    filter_state: Vec<f64>,
}

impl Actor for MyProcessorActor {
    type State = Self;
    type Supervision = kameo::SupervisionStrategy; // Define supervision strategy
    
    fn new() -> Self::State {
        MyProcessorActor {
            filter_state: vec![0.1, 0.2, 0.1], // Example state
        }
    }
}

// Implement message handler for ProcessData
impl Message<ProcessData> for MyProcessorActor {
    type Result = RecordBatch; // Or () if publishing directly

    async fn handle(&mut self, message: ProcessData, _ctx: &mut Context<Self>) -> Self::Result {
        let input_batch = message.0;
        
        // --- Example Processing: Simple Moving Average Filter ---
        // This is a placeholder; actual processing would use ndarray, rustfft, etc.
        
        // Assume input_batch has a "value" column of Float64
        let values_array = input_batch.column_by_name("value")
            .and_then(|col| col.as_any().downcast_ref::<Float64Array>())
            .expect("Input batch must have a 'value' column of Float64");

        let processed_values: Vec<f64> = values_array.iter()
            .map(|val| val.unwrap_or_default() * 0.5) // Simple example filter
            .collect();

        let processed_array = Arc::new(Float64Array::from(processed_values));

        // Create a new RecordBatch with processed data
        let schema = Arc::new(Schema::new(vec![
            Field::new("timestamp", DataType::UInt64, false),
            Field::new("channel", DataType::Utf8, false),
            Field::new("value", DataType::Float64, false),
        ]));
        
        // In a real scenario, you'd preserve other columns or add new ones
        RecordBatch::try_new(
            schema,
            vec![
                input_batch.column_by_name("timestamp").unwrap().clone(),
                input_batch.column_by_name("channel").unwrap().clone(),
                processed_array,
            ],
        ).expect("Failed to create processed RecordBatch")
    }
}

// Example of how to send data to the processor actor
#[tokio::main]
async fn main() {
    let processor_ref = MyProcessorActor::spawn().await;

    // Create a dummy RecordBatch
    let schema = Arc::new(Schema::new(vec![
        Field::new("timestamp", DataType::UInt64, false),
        Field::new("channel", DataType::Utf8, false),
        Field::new("value", DataType::Float64, false),
    ]));
    let timestamps = Arc::new(UInt64Array::from(vec![1, 2, 3]));
    let channels = Arc::new(arrow::array::StringArray::from(vec!["sensor1", "sensor1", "sensor1"]));
    let values = Arc::new(Float64Array::from(vec![10.0, 12.0, 11.0]));
    let input_batch = RecordBatch::try_new(schema, vec![timestamps, channels, values]).unwrap();

    let processed_batch = processor_ref.send(ProcessData(input_batch)).await.unwrap();
    println!("Processed Batch: {:?}", processed_batch);
}
```

## Integration with Instrument Actors

Instrument actors will typically acquire raw data and convert it into `RecordBatch`es. These batches can then be sent to one or more processor actors.

```rust
// Inside an InstrumentActor's message handler
async fn handle_acquisition(&mut self, _ctx: &mut Context<Self>) {
    let raw_data = self.acquire_from_hardware().await;
    let record_batch = self.convert_to_arrow(raw_data);

    // Send to a processor actor
    let processor_ref = self.processor_actor_ref.clone();
    let processed_batch = processor_ref.send(ProcessData(record_batch)).await.unwrap();

    // Publish processed_batch to other subscribers (e.g., GUI, Storage)
    self.publish_data(processed_batch).await;
}
```

## Common Processing Patterns

### 1. Filtering Data

Processors can filter `RecordBatch`es based on criteria (e.g., remove outliers, select specific channels).

### 2. Transforming Data

Convert data from one form to another (e.g., time-domain to frequency-domain using `rustfft`, or applying calibration curves).

### 3. Aggregating Data

Compute statistics (mean, min, max) over time windows or groups of data using `polars` or custom logic.

## Error Handling

Processor actors should implement robust error handling. If a processing step fails for a `RecordBatch`, the actor can:
*   Log the error using `tracing`.
*   Skip the problematic batch.
*   Return an error message (if the message expects a result).
*   Utilize Kameo's supervision strategies for actor restarts.

## Testing Processor Actors

Processor actors are easily testable by sending them `RecordBatch` messages and asserting on the output `RecordBatch`es. Mocking can be used for external dependencies.

```rust
#[tokio::test]
async fn test_my_processor_actor() {
    let processor_ref = MyProcessorActor::spawn().await;

    // Create a dummy input RecordBatch
    let schema = Arc::new(Schema::new(vec![
        Field::new("timestamp", DataType::UInt64, false),
        Field::new("channel", DataType::Utf8, false),
        Field::new("value", DataType::Float64, false),
    ]));
    let timestamps = Arc::new(UInt64Array::from(vec![1, 2, 3]));
    let channels = Arc::new(arrow::array::StringArray::from(vec!["test", "test", "test"]));
    let values = Arc::new(Float64Array::from(vec![10.0, 20.0, 30.0]));
    let input_batch = RecordBatch::try_new(schema, vec![timestamps, channels, values]).unwrap();

    let processed_batch = processor_ref.send(ProcessData(input_batch)).await.unwrap();

    // Assertions on the processed_batch
    let output_values = processed_batch.column_by_name("value")
        .and_then(|col| col.as_any().downcast_ref::<Float64Array>())
        .unwrap();
    assert_eq!(output_values.value(0), 5.0); // Expecting 0.5 multiplier from example
}
```

## See Also

*   [V4 System Architecture](../../ARCHITECTURE.md)
*   [Kameo Documentation](https://docs.rs/kameo/latest/kameo/)
*   [Apache Arrow Rust Documentation](https://docs.rs/arrow/latest/arrow/)
*   [Polars Documentation](https://docs.rs/polars/latest/polars/)
*   [RustFFT Documentation](https://docs.rs/rustfft/latest/rustfft/)
