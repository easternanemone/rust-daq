# Phase 4: Data Plane (Weeks 7+)

## Phase 4: Data Plane Epic
type: epic
priority: P0
parent: bd-oq51
description: |
  Implement high-performance data streaming with memory-mapped ring buffer.

  OBJECTIVE: Enable zero-copy data access for real-time analysis.
  TIMELINE: Weeks 7+
  PARALLELIZABLE: Tasks J and K are sequential (K depends on J)

  SUCCESS CRITERIA:
  - Ring buffer supports 10k+ writes/sec
  - Python can attach and read via pyarrow (zero-copy)
  - HDF5 writer persists data in background without blocking
  - GUI can "scroll back" 5 minutes in data timeline
  - < 1ms latency from instrument write to ring buffer

## Task J: Memory-Mapped Ring Buffer Implementation
type: task
priority: P0
parent: bd-oq51.4
description: |
  Implement lock-free ring buffer with Apache Arrow schema.

  CREATE: src/data/ring_buffer.rs

  MEMORY LAYOUT (#[repr(C)] for cross-language compatibility):
  ```rust
  #[repr(C)]
  struct RingBufferHeader {
      magic: u64,              // 0xDA_DA_DA_DA_00_00_00_01
      capacity_bytes: u64,     // Total size of data region
      write_head: AtomicU64,   // Current write offset
      read_tail: AtomicU64,    // Oldest valid data offset
      schema_len: u32,         // Length of Arrow Schema JSON
      _padding: [u8; 116],     // Pad to 128 bytes (cache line alignment)
  }

  const HEADER_SIZE: usize = 128;
  ```

  IMPLEMENTATION:
  ```rust
  use memmap2::{MmapMut, MmapOptions};
  use std::sync::atomic::{AtomicU64, Ordering};
  use std::fs::OpenOptions;

  pub struct RingBuffer {
      mmap: MmapMut,
      header: *mut RingBufferHeader,
      data_ptr: *mut u8,
      capacity: u64,
  }

  impl RingBuffer {
      /// Create new ring buffer backed by shared memory
      pub fn create(path: &Path, capacity_mb: usize) -> Result<Self> {
          let capacity_bytes = capacity_mb * 1024 * 1024;
          let total_size = HEADER_SIZE + capacity_bytes;

          // Create memory-mapped file
          let file = OpenOptions::new()
              .read(true)
              .write(true)
              .create(true)
              .open(path)?;

          file.set_len(total_size as u64)?;

          let mut mmap = unsafe { MmapOptions::new().map_mut(&file)? };

          // Initialize header
          let header = mmap.as_mut_ptr() as *mut RingBufferHeader;
          unsafe {
              (*header).magic = 0xDA_DA_DA_DA_00_00_00_01;
              (*header).capacity_bytes = capacity_bytes as u64;
              (*header).write_head = AtomicU64::new(0);
              (*header).read_tail = AtomicU64::new(0);
              (*header).schema_len = 0;
          }

          let data_ptr = unsafe { mmap.as_mut_ptr().add(HEADER_SIZE) };

          Ok(Self {
              mmap,
              header,
              data_ptr,
              capacity: capacity_bytes as u64,
          })
      }

      /// Write data to ring buffer (lock-free)
      pub fn write(&self, data: &[u8]) -> Result<()> {
          let len = data.len() as u64;
          if len > self.capacity {
              return Err(anyhow!("Data too large for ring buffer"));
          }

          unsafe {
              let head = (*self.header).write_head.load(Ordering::Acquire);
              let write_offset = (head % self.capacity) as isize;

              // Simple circular wrap (overwrites old data)
              let dest = self.data_ptr.offset(write_offset);
              std::ptr::copy_nonoverlapping(data.as_ptr(), dest, data.len());

              (*self.header).write_head.fetch_add(len, Ordering::Release);
          }

          Ok(())
      }

      /// Read current snapshot (from tail to head)
      pub fn read_snapshot(&self) -> Vec<u8> {
          unsafe {
              let head = (*self.header).write_head.load(Ordering::Acquire);
              let tail = (*self.header).read_tail.load(Ordering::Acquire);

              let available = (head - tail).min(self.capacity);
              let read_offset = (tail % self.capacity) as isize;

              let mut buffer = vec![0u8; available as usize];
              let src = self.data_ptr.offset(read_offset);
              std::ptr::copy_nonoverlapping(src, buffer.as_mut_ptr(), available as usize);

              buffer
          }
      }
  }

  // Python access via PyArrow
  impl RingBuffer {
      /// Get memory address for external mapping (Python/C++)
      pub fn data_address(&self) -> usize {
          self.data_ptr as usize
      }

      /// Write Arrow RecordBatch in IPC format
      pub fn write_arrow_batch(&self, batch: &RecordBatch) -> Result<()> {
          let mut buffer = Vec::new();
          let mut writer = arrow::ipc::writer::FileWriter::try_new(
              &mut buffer,
              &batch.schema()
          )?;

          writer.write(batch)?;
          writer.finish()?;

          self.write(&buffer)
      }
  }
  ```

  PYTHON READER (for demonstration):
  ```python
  import mmap
  import struct
  import pyarrow as pa

  class RingBufferReader:
      def __init__(self, path):
          with open(path, 'rb') as f:
              self.mmap = mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ)

          # Read header
          magic, capacity, write_head, read_tail = struct.unpack_from('QQQQ', self.mmap, 0)
          assert magic == 0xDADADADA00000001

          self.capacity = capacity
          self.data_offset = 128

      def read_latest(self):
          # Read Arrow IPC stream from ring buffer
          write_head = struct.unpack_from('Q', self.mmap, 16)[0]
          # ... parse Arrow data
  ```

  ACCEPTANCE:
  - Ring buffer creates 100MB shared memory file
  - write() succeeds at 10k+ ops/sec
  - Python can read data via mmap
  - Header layout matches C struct alignment
  - Zero crashes under concurrent read/write

## Task K: HDF5 Background Writer
type: task
priority: P0
parent: bd-oq51.4
deps: bd-oq51.4.1
description: |
  Background task that persists ring buffer data to HDF5.

  CREATE: src/data/hdf5_writer.rs

  IMPLEMENTATION:
  ```rust
  use hdf5::{File, Group, Result};
  use tokio::time::{interval, Duration};
  use crate::data::ring_buffer::RingBuffer;

  pub struct HDF5Writer {
      file: File,
      ring_buffer: Arc<RingBuffer>,
      flush_interval: Duration,
      last_read_tail: AtomicU64,
  }

  impl HDF5Writer {
      pub fn new(output_path: &Path, ring_buffer: Arc<RingBuffer>) -> Result<Self> {
          let file = File::create(output_path)?;

          Ok(Self {
              file,
              ring_buffer,
              flush_interval: Duration::from_secs(1),
              last_read_tail: AtomicU64::new(0),
          })
      }

      /// Run background writer loop
      pub async fn run(mut self) {
          let mut interval = interval(self.flush_interval);

          loop {
              interval.tick().await;

              if let Err(e) = self.flush_to_disk() {
                  eprintln!("HDF5 flush error: {}", e);
              }
          }
      }

      fn flush_to_disk(&mut self) -> Result<()> {
          // Read new data from ring buffer
          let snapshot = self.ring_buffer.read_snapshot();

          if snapshot.is_empty() {
              return Ok(()); // No new data
          }

          // Parse Arrow RecordBatch from snapshot
          // ... (Arrow IPC deserialization)

          // Write to HDF5
          let dataset = self.file.group("measurements")?.create_group("batch_001")?;
          // ... (HDF5 write logic)

          Ok(())
      }
  }
  ```

  INTEGRATION (main.rs daemon mode):
  ```rust
  #[tokio::main]
  async fn main() -> Result<()> {
      // Initialize ring buffer
      let ring_buffer = Arc::new(RingBuffer::create(
          Path::new("/dev/shm/rust_daq_ring"),
          100 // 100 MB
      )?);

      // Start background HDF5 writer
      let writer = HDF5Writer::new(
          Path::new("experiment_data.h5"),
          ring_buffer.clone()
      )?;

      tokio::spawn(async move {
          writer.run().await;
      });

      // ... rest of daemon initialization
  }
  ```

  ACCEPTANCE:
  - HDF5 file created on first flush
  - Data written every 1 second
  - No blocking of main thread
  - HDF5 file readable by h5py/MATLAB
  - Writer survives ring buffer overruns
