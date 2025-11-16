# Data Management and Storage Guide (V4 Architecture)

This guide outlines the data management and storage strategies for the Rust DAQ application under the new V4 architecture. The core principles are high-performance, interoperability, and structured storage, leveraging industry-standard libraries.

## 1. Core Data Representation: Apache Arrow

In the V4 architecture, all in-memory measurement data is represented using **[Apache Arrow](https://arrow.apache.org/)** (`arrow-rs`). This provides a columnar, high-performance, and language-agnostic data format.

### Key Concepts:

*   **`RecordBatch`**: The primary unit of data transfer. A `RecordBatch` consists of a `Schema` and a set of `Array`s, where each `Array` represents a column of data.
*   **`Schema`**: Defines the columns (fields) and their data types within a `RecordBatch`.
*   **`Array`**: Columnar data structures optimized for analytical processing.

### Example `RecordBatch` for Scalar Measurements:

```rust
use arrow::record_batch::RecordBatch;
use arrow::array::{Float64Array, UInt64Array, StringArray};
use arrow::datatypes::{Schema, Field, DataType};
use std::sync::Arc;

// Define the schema for scalar measurements
let schema = Arc::new(Schema::new(vec![
    Field::new("timestamp_ns", DataType::UInt64, false), // Nanoseconds since epoch
    Field::new("channel", DataType::Utf8, false),
    Field::new("value", DataType::Float64, false),
    Field::new("unit", DataType::Utf8, true), // Unit might be optional
]));

// Example data
let timestamps = Arc::new(UInt64Array::from(vec![1_678_886_400_000_000_000, 1_678_886_401_000_000_000]));
let channels = Arc::new(StringArray::from(vec!["temp_sensor_1", "temp_sensor_1"]));
let values = Arc::new(Float64Array::from(vec![23.5, 23.6]));
let units = Arc::new(StringArray::from(vec!["degC", "degC"]));

let record_batch = RecordBatch::try_new(
    schema.clone(),
    vec![timestamps, channels, values, units],
).unwrap();

println!("Scalar RecordBatch:\n{:?}", record_batch);
```

## 2. Data Flow within Kameo Actors

Data flows through the system as `RecordBatch`es, passed between Kameo actors via messages.

```mermaid
graph LR
    subgraph "Kameo Actor System"
        InstrumentActor[Instrument Actor<br/>(e.g., Newport)] -- RecordBatch --> InstrumentManager[InstrumentManager Actor]
        InstrumentManager -- RecordBatch --> ProcessorActor[Processor Actor<br/>(e.g., FFT, Filter)]
        ProcessorActor -- RecordBatch --> StorageActor[Storage Actor<br/>(HDF5 Writer)]
        InstrumentManager -- RecordBatch --> GuiActor[GUI Actor]
    end
```

*   **Instrument Actors:** Acquire raw data from hardware, convert it into `RecordBatch`es, and send them to the `InstrumentManager`.
*   **InstrumentManager Actor:** Acts as a central hub, receiving `RecordBatch`es from instruments and distributing them to interested subscribers (other actors).
*   **Processor Actors:** Receive `RecordBatch`es, perform transformations (e.g., `rustfft` for FFT, `ndarray` for numerical operations), and output new `RecordBatch`es.
*   **Storage Actor:** Receives `RecordBatch`es and persists them to disk using `hdf5-rust`.
*   **GUI Actor:** Receives `RecordBatch`es for real-time visualization using `egui_plot`.

## 3. Data Persistence: HDF5

For structured, hierarchical data storage, the V4 architecture will use **HDF5** via the `hdf5-rust` crate.

### Key Features:

*   **Hierarchical Structure:** Organize data into groups and datasets, mirroring the logical structure of experiments.
*   **Metadata Support:** Store rich metadata alongside data.
*   **Interoperability:** HDF5 files are widely supported across scientific computing ecosystems (Python, MATLAB, etc.).

### Example HDF5 Storage of `RecordBatch`:

```rust
use hdf5::File;
use arrow::ipc::writer::FileWriter;
use arrow::ipc::reader::FileReader;
use std::io::Cursor;

// Assuming you have a RecordBatch `record_batch`
// ... (from example above) ...

// Save RecordBatch to HDF5
fn save_record_batch_to_hdf5(file_path: &str, dataset_name: &str, batch: &RecordBatch) -> hdf5::Result<()> {
    let file = File::create(file_path)?;
    let group = file.create_group("arrow_data")?;

    // Write RecordBatch to an in-memory buffer using Arrow IPC format
    let mut buffer = Vec::new();
    let mut writer = FileWriter::try_new(&mut buffer, batch.schema().as_ref())?;
    writer.write(batch)?;
    writer.finish()?;

    // Store the IPC-formatted Arrow data as a HDF5 dataset
    group.new_dataset::<u8>()
        .with_data(&buffer)
        .create(dataset_name)?;

    Ok(())
}

// Load RecordBatch from HDF5
fn load_record_batch_from_hdf5(file_path: &str, dataset_name: &str) -> hdf5::Result<RecordBatch> {
    let file = File::open(file_path)?;
    let group = file.group("arrow_data")?;
    let dataset = group.dataset(dataset_name)?;
    let buffer: Vec<u8> = dataset.read_raw()?;

    let mut reader = FileReader::try_new(Cursor::new(buffer), None)?;
    reader.next().transpose().map_err(|e| hdf5::Error::Internal(e.to_string()))
}
```

## 4. Data Manipulation: Polars

**[Polars](https://github.com/pola-rs/polars)** is a high-performance DataFrame library that can be used for efficient data manipulation and analysis within the Rust application.

*   **Integration:** Polars can directly consume and produce Apache Arrow `RecordBatch`es, making it a natural fit for the V4 data pipeline.
*   **Use Cases:** Filtering, aggregation, joining, and complex transformations of tabular data.

### Example Polars Usage with Arrow:

```rust
use polars::prelude::*;
use arrow::record_batch::RecordBatch;
use std::sync::Arc;

// Assuming you have an Arrow RecordBatch `record_batch`
// ...

// Convert Arrow RecordBatch to Polars DataFrame
let df = DataFrame::try_from(record_batch.clone()).unwrap();

// Perform some operations with Polars
let processed_df = df.lazy()
    .filter(col("value").gt(lit(20.0)))
    .select(&[col("timestamp_ns"), col("value").alias("filtered_value")])
    .collect()
    .unwrap();

println!("Processed DataFrame:\n{:?}", processed_df);

// Convert back to Arrow RecordBatch if needed
let processed_arrow_batch = RecordBatch::try_from(processed_df).unwrap();
```

## 5. Real-time Buffering

Real-time buffering will be managed by individual Kameo actors or specialized buffering actors. This ensures that each data stream has its own dedicated buffer, preventing global bottlenecks.

## 6. Error Handling

Data processing errors will be handled using Rust's `Result` type and the `thiserror` crate for custom error types, integrated with the `tracing` logging system for detailed diagnostics.