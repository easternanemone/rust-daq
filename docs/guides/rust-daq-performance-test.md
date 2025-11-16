# Performance Optimization and Testing Guide (V4 Architecture)

## Overview

This guide covers performance optimization strategies, benchmarking, profiling, and comprehensive testing approaches for the Rust DAQ application under the V4 architecture. The focus is on achieving optimal real-time performance and reliability using Kameo actors and Apache Arrow data.

## 1. Performance Optimization Strategies

### Zero-Copy Data Handling with Apache Arrow

The V4 architecture leverages `apache/arrow-rs` for in-memory data representation, which inherently supports zero-copy operations. This minimizes data movement and serialization overhead.

```rust
use arrow::record_batch::RecordBatch;
use arrow::array::{Float64Array, UInt64Array};
use arrow::datatypes::{Schema, Field, DataType};
use std::sync::Arc;

// When passing data between actors or components, use Arc<RecordBatch>
// This allows multiple consumers to access the same data without copying.
pub async fn process_shared_record_batch(batch: Arc<RecordBatch>) {
    // Access data directly from the Arc'd RecordBatch
    let values = batch.column_by_name("value")
        .and_then(|col| col.as_any().downcast_ref::<Float64Array>())
        .expect("Expected 'value' column");

    for i in 0..values.len() {
        // Process values
        let _value = values.value(i);
    }
    // No data copying occurred here
}

// Example of creating a RecordBatch (often done by instrument actors)
pub fn create_example_record_batch() -> RecordBatch {
    let schema = Arc::new(Schema::new(vec![
        Field::new("timestamp_ns", DataType::UInt64, false),
        Field::new("value", DataType::Float64, false),
    ]));
    let timestamps = Arc::new(UInt64Array::from(vec![1, 2, 3]));
    let values = Arc::new(Float64Array::from(vec![10.0, 20.0, 30.0]));
    RecordBatch::try_new(schema, vec![timestamps, values]).unwrap()
}
```

### SIMD Optimization for Numerical Processing

For computationally intensive signal processing tasks, SIMD (Single Instruction, Multiple Data) instructions can be utilized. Libraries like `wide` or direct `std::arch` can be used, often in conjunction with `ndarray` for array operations.

```rust
use ndarray::{Array1, ArrayView1};
use wide::f64x4; // For portable SIMD operations

pub fn apply_simd_filter(input: ArrayView1<f64>, coefficients: &[f64]) -> Array1<f64> {
    let mut output = Array1::zeros(input.len());
    
    // Process 4 elements at a time using SIMD
    let chunks = input.exact_chunks(4);
    let remainder = chunks.remainder();
    
    for (i, chunk) in chunks.into_iter().enumerate() {
        let data = f64x4::from(chunk.as_slice().unwrap());
        let mut result = f64x4::splat(0.0);
        
        for &coeff in coefficients {
            result += data * f64x4::splat(coeff);
        }
        output.slice_mut(s![i*4..(i+1)*4]).copy_from_slice(&result.to_array());
    }
    
    // Handle remainder (scalar processing)
    for (i, &value) in remainder.iter().enumerate() {
        let mut filtered_value = 0.0;
        for &coeff in coefficients {
            filtered_value += value * coeff;
        }
        output[input.len() - remainder.len() + i] = filtered_value;
    }
    
    output
}
```

### Asynchronous Performance with Kameo Actors

Kameo actors provide a robust framework for concurrent and asynchronous processing, preventing blocking operations and maximizing CPU utilization.

```rust
use kameo::{Actor, Context, Message, ActorRef};
use tokio::time::Duration;
use arrow::record_batch::RecordBatch;

// Message for data processing
#[derive(Message)]
pub struct ProcessBatch(pub RecordBatch);

pub struct DataProcessorActor {
    // ... processor state
}

impl Actor for DataProcessorActor {
    type State = Self;
    type Supervision = kameo::SupervisionStrategy;

    fn new() -> Self::State {
        DataProcessorActor { /* ... */ }
    }
}

impl Message<ProcessBatch> for DataProcessorActor {
    type Result = RecordBatch; // Return processed batch

    async fn handle(&mut self, message: ProcessBatch, _ctx: &mut Context<Self>) -> Self::Result {
        let input_batch = message.0;
        // Perform CPU-bound processing here
        // Example: simple transformation
        let processed_batch = input_batch.project(&[0, 1]).unwrap(); // Project first two columns
        processed_batch
    }
}

// Example of sending data to a processor actor
pub async fn send_data_to_processor(processor_ref: ActorRef<DataProcessorActor>, data: RecordBatch) -> RecordBatch {
    processor_ref.send(ProcessBatch(data)).await.expect("Processor actor failed")
}
```

## 2. Benchmarking and Profiling

### Benchmarking with Criterion

Criterion is a powerful benchmarking harness for Rust. It can be used to measure the performance of individual components or end-to-end data pipelines.

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use tokio::runtime::Runtime;
use arrow::record_batch::RecordBatch;
use arrow::array::{Float64Array, UInt64Array};
use arrow::datatypes::{Schema, Field, DataType};
use std::sync::Arc;

// Helper to create a dummy RecordBatch
fn create_dummy_record_batch(size: usize) -> RecordBatch {
    let schema = Arc::new(Schema::new(vec![
        Field::new("timestamp_ns", DataType::UInt64, false),
        Field::new("value", DataType::Float64, false),
    ]));
    let timestamps = Arc::new(UInt64Array::from_iter_values(0..size as u64));
    let values = Arc::new(Float64Array::from_iter_fn(size, |i| (i as f64 * 0.001).sin()));
    RecordBatch::try_new(schema, vec![timestamps, values]).unwrap()
}

fn benchmark_arrow_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("arrow_data_processing");
    
    for size in [1000, 10000, 100000].iter() {
        let batch = create_dummy_record_batch(*size);
        
        group.bench_with_input(
            BenchmarkId::new("project_columns", size),
            &batch,
            |b, batch| {
                b.iter(|| {
                    black_box(batch.project(&[0, 1]).unwrap()) // Project two columns
                })
            },
        );
    }
    group.finish();
}

fn benchmark_kameo_actor_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("kameo_actor_throughput");
    
    for size in [100, 1000, 10000].iter() {
        let batch = create_dummy_record_batch(*size);
        
        group.bench_with_input(
            BenchmarkId::new("send_and_receive_batch", size),
            &batch,
            |b, batch| {
                b.to_async(&rt).iter(|| async {
                    let processor_ref = DataProcessorActor::spawn().await;
                    let _ = processor_ref.send(ProcessBatch(batch.clone())).await;
                    // In a real scenario, you'd measure round-trip time or processed data
                })
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    benchmark_arrow_processing,
    benchmark_kameo_actor_throughput
);
criterion_main!(benches);
```

### Memory Usage Profiling with `tracing-subscriber` and custom allocators

`tracing-subscriber` can be configured to collect memory allocation events, and custom global allocators can provide detailed memory usage statistics.

```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tracing_memory_allocator::MemoryTrackingLayer; // Example crate for memory tracking

pub fn init_profiling_logging() {
    let memory_layer = MemoryTrackingLayer::new();
    tracing_subscriber::registry()
        .with(memory_layer)
        .init();
    
    tracing::info!("Memory profiling enabled.");
}

// Then, in your code, you can log memory usage at points of interest
// tracing::info!(target: "memory", "Current memory usage: {} bytes", get_current_memory_usage());
```

## 3. Comprehensive Testing Strategy

### Unit Testing with Mocked Hardware

Instrument actors can be unit tested by mocking the underlying `InstrumentHardware` trait.

```rust
use mockall::{automock, predicate::*};
use kameo::{Actor, Context, Message, ActorRef};
use tokio::runtime::Runtime;
use arrow::record_batch::RecordBatch;

// Mock the hardware trait
#[automock]
#[async_trait::async_trait]
pub trait MockableInstrumentHardware: Send + Sync + 'static {
    async fn connect(&mut self, connection_string: &str) -> anyhow::Result<()>;
    async fn send_command(&mut self, command: &str) -> anyhow::Result<String>;
    async fn read_raw_data(&mut self) -> anyhow::Result<Vec<u8>>;
}

// Our Instrument Actor, now generic over the hardware
pub struct TestInstrumentActor<H: MockableInstrumentHardware> {
    id: String,
    hardware: H,
    // ... other fields
}

impl<H: MockableInstrumentHardware + 'static> Actor for TestInstrumentActor<H> {
    type State = Self;
    type Supervision = kameo::SupervisionStrategy;
    fn new() -> Self::State { unimplemented!() } // For testing, we'll create it directly
}

// Implement message handlers for TestInstrumentActor (similar to GenericInstrumentActor)
impl<H: MockableInstrumentHardware + 'static> Message<Connect> for TestInstrumentActor<H> {
    type Result = anyhow::Result<()>;
    async fn handle(&mut self, message: Connect, _ctx: &mut Context<Self>) -> Self::Result {
        self.hardware.connect(&message.0).await
    }
}
impl<H: MockableInstrumentHardware + 'static> Message<SendCommand> for TestInstrumentActor<H> {
    type Result = anyhow::Result<String>;
    async fn handle(&mut self, message: SendCommand, _ctx: &mut Context<Self>) -> Self::Result {
        self.hardware.send_command(&message.0).await
    }
}
impl<H: MockableInstrumentHardware + 'static> Message<ReadData> for TestInstrumentActor<H> {
    type Result = RecordBatch;
    async fn handle(&mut self, _message: ReadData, _ctx: &mut Context<Self>) -> Self::Result {
        let raw_data = self.hardware.read_raw_data().await.expect("Mock read failed");
        // Convert raw_data to Arrow RecordBatch (mock this too if complex)
        RecordBatch::new_empty(Arc::new(Schema::new(vec![])))
    }
}


#[tokio::test]
async fn test_instrument_actor_connection() {
    let mut mock_hardware = MockMockableInstrumentHardware::new();
    mock_hardware.expect_connect()
        .with(eq("mock_address"))
        .times(1)
        .returning(|_| Ok(()));
    mock_hardware.expect_send_command()
        .with(eq("*IDN?"))
        .times(1)
        .returning(|_| Ok("MOCK_INSTRUMENT".to_string()));

    let instrument_actor = TestInstrumentActor {
        id: "mock_inst_1".to_string(),
        hardware: mock_hardware,
        config: serde_json::Value::Null, // Placeholder
        data_publisher: ActorRef::null(), // Placeholder
    };
    let instrument_ref = instrument_actor.spawn().await;

    instrument_ref.send(Connect("mock_address".to_string())).await.unwrap().unwrap();
    let idn_response = instrument_ref.send(SendCommand("*IDN?".to_string())).await.unwrap().unwrap();
    assert_eq!(idn_response, "MOCK_INSTRUMENT");
}
```

### Integration Testing

Integration tests verify the end-to-end data flow and interaction between multiple Kameo actors (e.g., InstrumentManager, Instrument Actors, Storage Actors).

```rust
use tempfile::TempDir;
use tokio::time::Duration;
use kameo::{Actor, Context, Message, ActorRef};
use arrow::record_batch::RecordBatch;
use arrow::array::{Float64Array, UInt64Array, StringArray};
use arrow::datatypes::{Schema, Field, DataType};
use std::sync::Arc;

// Mock Storage Actor for integration testing
#[derive(Message)]
pub struct StoreData(pub RecordBatch);

pub struct MockStorageActor {
    pub received_batches: Vec<RecordBatch>,
}

impl Actor for MockStorageActor {
    type State = Self;
    type Supervision = kameo::SupervisionStrategy;
    fn new() -> Self::State { MockStorageActor { received_batches: Vec::new() } }
}

impl Message<StoreData> for MockStorageActor {
    type Result = ();
    async fn handle(&mut self, message: StoreData, _ctx: &mut Context<Self>) -> Self::Result {
        self.received_batches.push(message.0);
    }
}

#[tokio::test]
async fn test_end_to_end_data_pipeline() {
    let temp_dir = TempDir::new().unwrap();
    // In a real scenario, you'd configure HDF5 storage actor here

    // Spawn InstrumentManagerActor
    let instrument_manager_actor = InstrumentManagerActor::spawn().await;

    // Spawn a mock instrument actor
    let mut mock_hardware = MockMockableInstrumentHardware::new();
    mock_hardware.expect_connect().returning(|_| Ok(()));
    mock_hardware.expect_read_raw_data()
        .times(3)
        .returning(|| Ok(vec![1, 2, 3, 4, 5, 6, 7, 8])); // Dummy raw data
    
    let instrument_actor = TestInstrumentActor {
        id: "mock_inst_1".to_string(),
        hardware: mock_hardware,
        config: serde_json::Value::Null,
        data_publisher: instrument_manager_actor.clone(), // InstrumentManager is the publisher
    };
    let instrument_ref = instrument_actor.spawn().await;

    // Register instrument with manager
    instrument_manager_actor.send(InstrumentManagerMessage::RegisterInstrument(
        "mock_inst_1".to_string(),
        instrument_ref.clone(),
    )).await.unwrap();

    // Spawn MockStorageActor and subscribe it to InstrumentManager
    let storage_actor = MockStorageActor::spawn().await;
    instrument_manager_actor.send(InstrumentManagerMessage::AddDataSubscriber(
        storage_actor.clone().into(), // Convert to generic ActorRef
    )).await.unwrap();

    // Connect and start acquisition
    instrument_manager_actor.send(InstrumentManagerMessage::ConnectInstrument(
        "mock_inst_1".to_string(),
        "mock_address".to_string(),
    )).await.unwrap();
    instrument_manager_actor.send(InstrumentManagerMessage::StartAcquisition(
        "mock_inst_1".to_string(),
    )).await.unwrap();

    tokio::time::sleep(Duration::from_millis(500)).await; // Allow some data to flow

    // Verify data was received by storage actor
    let received_batches = storage_actor.send(GetReceivedBatches).await.unwrap(); // Assuming GetReceivedBatches message
    assert!(!received_batches.is_empty());
    assert!(received_batches.len() >= 3); // At least 3 batches from 3 reads
}
```

### Performance and Load Testing

Performance and load tests simulate real-world conditions to identify bottlenecks and ensure the system meets throughput and latency requirements.

```rust
use tokio::time::Duration;
use kameo::{Actor, Context, Message, ActorRef};
use arrow::record_batch::RecordBatch;
use arrow::array::{Float64Array, UInt64Array};
use arrow::datatypes::{Schema, Field, DataType};
use std::sync::Arc;
use std::time::Instant;

// Message to simulate data acquisition
#[derive(Message)]
pub struct SimulateAcquisition(pub usize); // Number of samples

pub struct HighThroughputInstrumentActor {
    id: String,
    data_publisher: ActorRef<InstrumentManagerActor>,
    sample_rate_hz: f64,
}

impl Actor for HighThroughputInstrumentActor {
    type State = Self;
    type Supervision = kameo::SupervisionStrategy;
    fn new() -> Self::State { unimplemented!() }
}

impl Message<SimulateAcquisition> for HighThroughputInstrumentActor {
    type Result = ();
    async fn handle(&mut self, message: SimulateAcquisition, _ctx: &mut Context<Self>) -> Self::Result {
        let num_samples = message.0;
        let schema = Arc::new(Schema::new(vec![
            Field::new("timestamp_ns", DataType::UInt64, false),
            Field::new("value", DataType::Float64, false),
        ]));

        for i in 0..num_samples {
            let timestamp = (i as u64 * (1_000_000_000 / self.sample_rate_hz as u64));
            let value = (i as f64 * 0.001).sin();
            
            let timestamps = Arc::new(UInt64Array::from(vec![timestamp]));
            let values = Arc::new(Float64Array::from(vec![value]));
            let batch = RecordBatch::try_new(schema.clone(), vec![timestamps, values]).unwrap();

            // Send data to manager
            let _ = self.data_publisher.send(InstrumentManagerMessage::NewData(self.id.clone(), batch)).await;
            tokio::time::sleep(Duration::from_nanos((1_000_000_000.0 / self.sample_rate_hz) as u64)).await;
        }
    }
}

#[tokio::test]
async fn test_high_throughput_data_pipeline() {
    let num_instruments = 5;
    let samples_per_instrument = 1000;
    let target_sample_rate_hz = 1000.0; // Each instrument produces 1000 samples/sec

    let instrument_manager_actor = InstrumentManagerActor::spawn().await;
    let storage_actor = MockStorageActor::spawn().await; // To count received data

    instrument_manager_actor.send(InstrumentManagerMessage::AddDataSubscriber(
        storage_actor.clone().into(),
    )).await.unwrap();

    let mut instrument_refs = Vec::new();
    for i in 0..num_instruments {
        let id = format!("inst_{}", i);
        let instrument_actor = HighThroughputInstrumentActor {
            id: id.clone(),
            data_publisher: instrument_manager_actor.clone(),
            sample_rate_hz: target_sample_rate_hz,
        };
        let instrument_ref = instrument_actor.spawn().await;
        instrument_manager_actor.send(InstrumentManagerMessage::RegisterInstrument(
            id.clone(),
            instrument_ref.clone().into(),
        )).await.unwrap();
        instrument_refs.push(instrument_ref);
    }

    let start_time = Instant::now();
    let mut tasks = Vec::new();
    for instrument_ref in &instrument_refs {
        let task = tokio::spawn(instrument_ref.send(SimulateAcquisition(samples_per_instrument)));
        tasks.push(task);
    }

    for task in tasks {
        task.await.unwrap().unwrap(); // Wait for all simulations to complete
    }

    tokio::time::sleep(Duration::from_millis(500)).await; // Allow data to propagate

    let total_expected_samples = num_instruments * samples_per_instrument;
    let received_batches = storage_actor.send(GetReceivedBatches).await.unwrap();
    let total_received_samples: usize = received_batches.iter().map(|b| b.num_rows()).sum();

    let elapsed = start_time.elapsed();
    let throughput = total_received_samples as f64 / elapsed.as_secs_f64();

    tracing::info!("Total expected samples: {}", total_expected_samples);
    tracing::info!("Total received samples: {}", total_received_samples);
    tracing::info!("Throughput: {:.2} samples/sec", throughput);

    assert_eq!(total_received_samples, total_expected_samples, "Data loss detected!");
    assert!(throughput > (num_instruments as f64 * target_sample_rate_hz * 0.9), "Throughput is too low!");
}
```

This performance and testing guide provides comprehensive strategies for optimizing and validating your scientific data acquisition application, ensuring it meets the demanding requirements of real-time scientific instrumentation.