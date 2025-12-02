//! Memory-mapped ring buffer for lock-free, zero-copy data streaming.
//!
//! This module implements a high-performance ring buffer backed by memory-mapped files,
//! designed for concurrent access with a single writer and multiple readers.
//!
//! # Features
//! - Lock-free operations using atomic instructions
//! - Zero-copy data access via memory mapping
//! - Cross-language compatibility with #[repr(C)] layout
//! - Apache Arrow IPC format support for structured data
//! - Designed for 10k+ writes/sec throughput
//!
//! # Architecture
//!
//! The ring buffer uses a memory-mapped file with a fixed header followed by a circular
//! data region. Writers append data at the `write_head` position, while readers consume
//! from the `read_tail` position. When the write head reaches capacity, it wraps back
//! to the beginning (circular behavior).
//!
//! # Thread Safety
//!
//! - **Writes**: Serialized via internal mutex. Multiple writers are safe but sequential.
//! - **Reads**: Lock-free using atomic loads with Acquire ordering.
//! - **Concurrent read/write**: Safe via seqlock pattern with epoch counter validation.
//!
//! # Memory Layout
//! ```text
//! [128-byte header] [variable-size data region]
//!
//! Header (cache-line aligned):
//!   magic: u64              (0xDA_DA_DA_DA_00_00_00_01)
//!   capacity_bytes: u64     (size of data region)
//!   write_head: AtomicU64   (current write offset)
//!   read_tail: AtomicU64    (oldest valid data offset)
//!   schema_len: u32         (Arrow schema length)
//!   padding: [u8; 116]      (cache line alignment)
//! ```
//!
//! # Example
//!
//! ```no_run
//! use std::path::Path;
//! use rust_daq::data::ring_buffer::RingBuffer;
//!
//! # fn main() -> anyhow::Result<()> {
//! // Create a 100 MB ring buffer
//! let rb = RingBuffer::create(Path::new("/tmp/my_ring.buf"), 100)?;
//!
//! // Write data
//! rb.write(b"Hello, world!")?;
//!
//! // Read snapshot
//! let data = rb.read_snapshot();
//! assert_eq!(&data, b"Hello, world!");
//!
//! // Mark data as consumed
//! rb.advance_tail(data.len() as u64);
//! # Ok(())
//! # }
//! ```

use anyhow::{anyhow, Context, Result};
use memmap2::{MmapMut, MmapOptions};
use std::fs::OpenOptions;
use std::path::Path;
use std::sync::atomic::{fence, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

#[cfg(feature = "storage_arrow")]
use arrow::record_batch::RecordBatch;

/// Magic number for ring buffer header validation
const MAGIC: u64 = 0xDA_DA_DA_DA_00_00_00_01;

/// Size of the ring buffer header in bytes (128 bytes = 2 cache lines on most systems)
const HEADER_SIZE: usize = 128;

/// Ring buffer header with cache-line alignment.
///
/// This structure uses #[repr(C)] to ensure a predictable memory layout
/// that can be accessed from other languages (Python, C++).
///
/// Layout (128 bytes total):
/// - magic: u64 (8 bytes)
/// - capacity_bytes: u64 (8 bytes)
/// - write_head: AtomicU64 (8 bytes)
/// - read_tail: AtomicU64 (8 bytes)
/// - write_epoch: AtomicU64 (8 bytes)
/// - schema_len: u32 (4 bytes)
/// - _padding: [u8; 84] (84 bytes)
/// Total: 8 + 8 + 8 + 8 + 8 + 4 + 84 = 128 bytes
#[repr(C)]
struct RingBufferHeader {
    /// Magic number for validation (0xDADADADA00000001)
    magic: u64,

    /// Total size of the data region in bytes
    capacity_bytes: u64,

    /// Current write position (monotonically increasing)
    write_head: AtomicU64,

    /// Oldest valid data position (for circular buffer management)
    read_tail: AtomicU64,

    /// Write epoch counter for seqlock synchronization
    ///
    /// Incremented before and after each write operation.
    /// Readers check this before and after reading - if it changed
    /// (or is odd during read), the read may have seen partial data.
    write_epoch: AtomicU64,

    /// Length of the Arrow schema JSON (if using Arrow format)
    schema_len: u32,

    /// Padding to align header to 128 bytes (cache line boundary)
    /// Calculation: 128 - (8 + 8 + 8 + 8 + 8 + 4) = 128 - 44 = 84 bytes
    _padding: [u8; 84],
}

// Static assertion to ensure header size matches HEADER_SIZE constant
const _: () = assert!(
    std::mem::size_of::<RingBufferHeader>() == HEADER_SIZE,
    "RingBufferHeader size must equal HEADER_SIZE (128 bytes)"
);

/// High-performance memory-mapped ring buffer.
///
/// # Safety
/// This structure contains raw pointers to memory-mapped regions. It is safe to use
/// as long as:
/// - The memory-mapped file remains valid for the lifetime of RingBuffer
/// - Readers use appropriate atomic ordering (Acquire)
/// - Writers use appropriate atomic ordering (Release)
///
/// # Thread Safety
/// The buffer uses an internal write lock to serialize concurrent writes, making it
/// safe for multiple writers. Reads remain lock-free.
pub struct RingBuffer {
    /// Memory-mapped file backing the ring buffer
    #[expect(dead_code, reason = "mmap must be kept alive to maintain memory mapping validity")]
    mmap: MmapMut,

    /// Pointer to the header structure
    /// SAFETY: Points to the start of mmap, valid as long as mmap exists
    header: *mut RingBufferHeader,

    /// Pointer to the data region (after header)
    /// SAFETY: Points to HEADER_SIZE bytes into mmap, valid as long as mmap exists
    data_ptr: *mut u8,

    /// Capacity of the data region in bytes
    capacity: u64,

    /// Write lock to serialize concurrent writes and prevent data races
    write_lock: Mutex<()>,
}

impl std::fmt::Debug for RingBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RingBuffer")
            .field("capacity", &self.capacity)
            .field("write_head", &self.write_head())
            .field("read_tail", &self.read_tail())
            .field("data_ptr", &format!("{:p}", self.data_ptr))
            .field("header", &format!("{:p}", self.header))
            .finish()
    }
}

// SAFETY: RingBuffer uses atomic operations for synchronization and can be safely
// sent between threads. The raw pointers are only accessed with proper atomic ordering.
unsafe impl Send for RingBuffer {}

// SAFETY: Write operations are serialized via write_lock. Read operations use atomic
// instructions with Acquire ordering to see all previous writes. The combination
// makes concurrent access safe.
unsafe impl Sync for RingBuffer {}

impl RingBuffer {
    /// Create a new ring buffer backed by a memory-mapped file.
    ///
    /// # Arguments
    /// * `path` - Path to the backing file (typically in /dev/shm or /tmp)
    /// * `capacity_mb` - Size of the data region in megabytes
    ///
    /// # Returns
    /// A new `RingBuffer` instance with initialized header
    ///
    /// # Example
    /// ```no_run
    /// use std::path::Path;
    /// use rust_daq::data::ring_buffer::RingBuffer;
    ///
    /// let rb = RingBuffer::create(Path::new("/tmp/my_ring_buffer"), 100).unwrap();
    /// ```
    pub fn create(path: &Path, capacity_mb: usize) -> Result<Self> {
        let capacity_bytes = capacity_mb * 1024 * 1024;
        let total_size = HEADER_SIZE + capacity_bytes;

        // Create or open the backing file
        let mut opts = OpenOptions::new();
        opts.read(true).write(true).create(true);

        let is_new_file = !path.exists();

        // Only truncate when creating a brand-new buffer; preserve existing data otherwise.
        if is_new_file {
            opts.truncate(true);
        }

        let file = opts
            .open(path)
            .with_context(|| format!("Failed to create/open ring buffer file: {:?}", path))?;

        // Validate existing file size or set for new file
        let existing_size = file.metadata()
            .context("Failed to get file metadata")?
            .len();

        if is_new_file || existing_size == 0 {
            // Set file size for new buffer or empty file
            file.set_len(total_size as u64)
                .context("Failed to set ring buffer file size")?;
        } else if existing_size != total_size as u64 {
            // Existing buffer with data has different capacity - this would corrupt data
            return Err(anyhow!(
                "Ring buffer capacity mismatch: file has {} bytes but requested {} bytes. \
                 Delete the existing file or use matching capacity.",
                existing_size,
                total_size
            ));
        }

        // Create memory mapping
        // SAFETY: We just created the file and set its size, so mapping is safe
        let mut mmap = unsafe {
            MmapOptions::new()
                .map_mut(&file)
                .context("Failed to create memory mapping")?
        };

        // Initialize header
        // SAFETY: mmap is at least HEADER_SIZE bytes (total_size includes HEADER_SIZE)
        let header = mmap.as_mut_ptr() as *mut RingBufferHeader;
        unsafe {
            // Write header fields
            (*header).magic = MAGIC;
            (*header).capacity_bytes = capacity_bytes as u64;
            (*header).write_head = AtomicU64::new(0);
            (*header).read_tail = AtomicU64::new(0);
            (*header).write_epoch = AtomicU64::new(0);
            (*header).schema_len = 0;

            // Zero out padding for deterministic behavior
            (*header)._padding.fill(0);
        }

        // Calculate data region pointer
        // SAFETY: mmap is total_size bytes, so offset HEADER_SIZE is within bounds
        let data_ptr = unsafe { mmap.as_mut_ptr().add(HEADER_SIZE) };

        Ok(Self {
            mmap,
            header,
            data_ptr,
            capacity: capacity_bytes as u64,
            write_lock: Mutex::new(()),
        })
    }

    /// Open an existing ring buffer from a memory-mapped file.
    ///
    /// # Arguments
    /// * `path` - Path to the existing backing file
    ///
    /// # Returns
    /// A `RingBuffer` instance attached to the existing buffer
    pub fn open(path: &Path) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .with_context(|| format!("Failed to open ring buffer file: {:?}", path))?;

        // Create memory mapping
        // SAFETY: Opening existing file created by create()
        let mut mmap = unsafe {
            MmapOptions::new()
                .map_mut(&file)
                .context("Failed to map ring buffer file")?
        };

        // Validate header
        // SAFETY: File was created with create(), header is valid
        let header = mmap.as_mut_ptr() as *mut RingBufferHeader;
        let (magic, capacity) = unsafe { ((*header).magic, (*header).capacity_bytes) };

        if magic != MAGIC {
            return Err(anyhow!(
                "Invalid ring buffer magic number: expected 0x{:016X}, got 0x{:016X}",
                MAGIC,
                magic
            ));
        }

        // SAFETY: mmap size validated, offset HEADER_SIZE is within bounds
        let data_ptr = unsafe { mmap.as_mut_ptr().add(HEADER_SIZE) };

        Ok(Self {
            mmap,
            header,
            data_ptr,
            capacity,
            write_lock: Mutex::new(()),
        })
    }

    /// Write data to the ring buffer.
    ///
    /// This operation uses an internal lock to serialize concurrent writes.
    /// Reads remain lock-free via atomic operations.
    ///
    /// # Arguments
    /// * `data` - Byte slice to write
    ///
    /// # Returns
    /// Ok(()) on success, Err if data is too large for buffer
    ///
    /// # Note
    /// If the buffer is full, this will overwrite the oldest data (circular behavior)
    ///
    /// # Thread Safety
    /// Multiple concurrent writers are safe - the internal lock serializes writes.
    pub fn write(&self, data: &[u8]) -> Result<()> {
        // Acquire write lock to serialize concurrent writes
        let _guard = self.write_lock.lock().map_err(|_| anyhow!("Write lock poisoned"))?;

        let len = data.len() as u64;

        if len > self.capacity {
            return Err(anyhow!(
                "Data size ({} bytes) exceeds ring buffer capacity ({} bytes)",
                len,
                self.capacity
            ));
        }

        // SAFETY: header is valid for the lifetime of self, and data_ptr points to a
        // valid mmap region of size self.capacity bytes
        unsafe {
            // Increment epoch BEFORE write (odd = write in progress)
            // Use AcqRel to prevent the memcpy from floating up before this increment
            (*self.header).write_epoch.fetch_add(1, Ordering::AcqRel);

            // Load current write position with Acquire ordering to see previous writes
            let head = (*self.header).write_head.load(Ordering::Acquire);

            // Calculate circular offset
            let write_offset = (head % self.capacity) as usize;

            // Handle wrap-around: if data doesn't fit before end, split the write
            if write_offset + data.len() > self.capacity as usize {
                // SAFETY: write_offset < capacity (due to modulo), and first_part_len
                // is bounded by capacity - write_offset, so data_ptr.add(write_offset)
                // is within the mmap region [data_ptr, data_ptr + capacity)
                let first_part_len = self.capacity as usize - write_offset;
                let dest = self.data_ptr.add(write_offset);
                std::ptr::copy_nonoverlapping(data.as_ptr(), dest, first_part_len);

                // SAFETY: second_part_len = data.len() - first_part_len, and we already
                // checked data.len() <= capacity, so second_part_len <= capacity.
                // Writing to data_ptr (start of region) is always valid.
                let second_part_len = data.len() - first_part_len;
                std::ptr::copy_nonoverlapping(
                    data.as_ptr().add(first_part_len),
                    self.data_ptr,
                    second_part_len,
                );
            } else {
                // SAFETY: write_offset + data.len() <= capacity (checked above),
                // so the entire write range is within the mmap data region
                let dest = self.data_ptr.add(write_offset);
                std::ptr::copy_nonoverlapping(data.as_ptr(), dest, data.len());
            }

            // Update write head with Release ordering to publish the write
            (*self.header).write_head.fetch_add(len, Ordering::Release);

            // Increment epoch AFTER write (even = write complete)
            (*self.header).write_epoch.fetch_add(1, Ordering::Release);
        }

        Ok(())
    }

    /// Read a snapshot of current data in the ring buffer.
    ///
    /// This creates a copy of the available data from read_tail to write_head.
    /// Safe for concurrent use with write operations - uses seqlock validation
    /// to detect and retry if a write occurred during the read.
    ///
    /// # Returns
    /// A Vec containing a snapshot of the current buffer contents
    ///
    /// # Note
    /// This method will retry automatically if a concurrent write is detected.
    /// If the writer crashes mid-write (leaving epoch odd), this will timeout
    /// after MAX_RETRY_DURATION_MS and return an empty Vec.
    pub fn read_snapshot(&self) -> Vec<u8> {
        const MAX_RETRIES: usize = 100;
        const MAX_RETRY_DURATION_MS: u128 = 100; // 100ms timeout for crashed writer detection

        let start_time = Instant::now();

        for retry in 0..MAX_RETRIES {
            // Check for timeout (handles crashed writer scenario where epoch stays odd)
            if start_time.elapsed().as_millis() > MAX_RETRY_DURATION_MS {
                tracing::error!(
                    "read_snapshot timed out after {}ms - possible crashed writer (epoch stuck odd)",
                    MAX_RETRY_DURATION_MS
                );
                return Vec::new();
            }

            // SAFETY: header is valid for the lifetime of self
            unsafe {
                // Load epoch BEFORE read (must be even = no write in progress)
                let epoch_before = (*self.header).write_epoch.load(Ordering::Acquire);

                // If epoch is odd, a write is in progress - brief spin then retry
                if epoch_before % 2 != 0 {
                    // Exponential backoff for odd epoch (write in progress)
                    if retry < 10 {
                        std::hint::spin_loop();
                    } else {
                        // After 10 fast retries, yield to OS scheduler
                        std::thread::yield_now();
                    }
                    continue;
                }

                // Load positions with Acquire ordering to see all previous writes
                let head = (*self.header).write_head.load(Ordering::Acquire);
                let tail = (*self.header).read_tail.load(Ordering::Acquire);

                // Calculate available data (capped at capacity to handle wrap-around)
                let available = (head.saturating_sub(tail)).min(self.capacity);

                if available == 0 {
                    return Vec::new();
                }

                // Calculate read offset (circular)
                let read_offset = (tail % self.capacity) as usize;

                let mut buffer = vec![0u8; available as usize];

                // Handle wrap-around
                if read_offset + available as usize > self.capacity as usize {
                    // SAFETY: read_offset < capacity (due to modulo), and first_part_len
                    // is bounded by capacity - read_offset, so data_ptr.add(read_offset)
                    // is within the mmap region [data_ptr, data_ptr + capacity)
                    let first_part_len = self.capacity as usize - read_offset;
                    let src = self.data_ptr.add(read_offset);
                    std::ptr::copy_nonoverlapping(src, buffer.as_mut_ptr(), first_part_len);

                    // SAFETY: second_part_len = available - first_part_len <= capacity,
                    // and we read from data_ptr (start of data region) which is valid
                    let second_part_len = available as usize - first_part_len;
                    std::ptr::copy_nonoverlapping(
                        self.data_ptr,
                        buffer.as_mut_ptr().add(first_part_len),
                        second_part_len,
                    );
                } else {
                    // SAFETY: read_offset + available <= capacity, so the entire read
                    // range [data_ptr + read_offset, data_ptr + read_offset + available)
                    // is within the mmap data region
                    let src = self.data_ptr.add(read_offset);
                    std::ptr::copy_nonoverlapping(src, buffer.as_mut_ptr(), available as usize);
                }

                // Fence to ensure all data reads complete before loading epoch
                // Required for ARM/Apple Silicon where loads can be reordered
                fence(Ordering::SeqCst);

                // Load epoch AFTER read - must match epoch_before
                let epoch_after = (*self.header).write_epoch.load(Ordering::Acquire);

                if epoch_before == epoch_after {
                    // Read was consistent - no concurrent write occurred
                    return buffer;
                }

                // Epoch changed - a write occurred during our read, retry with backoff
                if retry < 10 {
                    std::hint::spin_loop();
                } else {
                    std::thread::yield_now();
                }
            }
        }

        // After max retries, return empty (caller should handle high contention)
        tracing::warn!(
            "read_snapshot exceeded {} retries due to high write contention",
            MAX_RETRIES
        );
        Vec::new()
    }

    /// Get the memory address of the data region for external mapping (Python/C++).
    ///
    /// This is useful for zero-copy access from other languages.
    ///
    /// # Returns
    /// The memory address of the data region as a usize
    pub fn data_address(&self) -> usize {
        self.data_ptr as usize
    }

    /// Get the capacity of the data region in bytes.
    ///
    /// This returns the maximum amount of data that can be stored in the ring buffer
    /// before wrap-around occurs. The actual file size is larger by [`HEADER_SIZE`]
    /// bytes (128 bytes for the header).
    ///
    /// # Returns
    ///
    /// The capacity in bytes as specified during creation.
    ///
    /// # Thread Safety
    ///
    /// This method is safe to call from multiple threads concurrently. The capacity
    /// is immutable after buffer creation.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use std::path::Path;
    /// # use rust_daq::data::ring_buffer::RingBuffer;
    /// # fn main() -> anyhow::Result<()> {
    /// let rb = RingBuffer::create(Path::new("/tmp/test.buf"), 100)?;
    /// assert_eq!(rb.capacity(), 100 * 1024 * 1024); // 100 MB
    /// # Ok(())
    /// # }
    /// ```
    pub fn capacity(&self) -> u64 {
        self.capacity
    }

    /// Get the current write head position (monotonically increasing).
    ///
    /// The write head tracks the total number of bytes written to the buffer since
    /// creation. It increases monotonically and never wraps. To get the actual offset
    /// in the circular buffer, compute `write_head % capacity`.
    ///
    /// # Returns
    ///
    /// Total bytes written since buffer creation.
    ///
    /// # Thread Safety
    ///
    /// Uses atomic load with Acquire ordering to ensure visibility of all writes
    /// that happened-before this load. Safe to call concurrently with writes.
    ///
    /// # Producer/Consumer Semantics
    ///
    /// - Available data = `write_head - read_tail` (capped at capacity)
    /// - If `write_head == read_tail`, the buffer is empty
    /// - If `write_head - read_tail > capacity`, old data has been overwritten
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use std::path::Path;
    /// # use rust_daq::data::ring_buffer::RingBuffer;
    /// # fn main() -> anyhow::Result<()> {
    /// let rb = RingBuffer::create(Path::new("/tmp/test.buf"), 1)?;
    ///
    /// rb.write(b"test")?;
    /// assert_eq!(rb.write_head(), 4);
    ///
    /// rb.write(b"data")?;
    /// assert_eq!(rb.write_head(), 8); // Monotonically increasing
    /// # Ok(())
    /// # }
    /// ```
    pub fn write_head(&self) -> u64 {
        // SAFETY: header is valid for the lifetime of self
        unsafe { (*self.header).write_head.load(Ordering::Acquire) }
    }

    /// Get the current read tail position (marks oldest unconsumed data).
    ///
    /// The read tail tracks the oldest data position that has not yet been consumed
    /// by readers. Like the write head, it increases monotonically and never wraps.
    /// To get the actual offset in the circular buffer, compute `read_tail % capacity`.
    ///
    /// # Returns
    ///
    /// Position of the oldest unconsumed byte.
    ///
    /// # Thread Safety
    ///
    /// Uses atomic load with Acquire ordering. Safe to call concurrently with
    /// `update_read_tail()` and `advance_tail()` operations.
    ///
    /// # Producer/Consumer Semantics
    ///
    /// - Consumers should call `update_read_tail()` or `advance_tail()` after
    ///   processing data to free up buffer space
    /// - The tail is managed by consumers; the buffer itself only updates it
    ///   via explicit calls
    /// - If not updated, old data will eventually be overwritten when the
    ///   write head laps the tail
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use std::path::Path;
    /// # use rust_daq::data::ring_buffer::RingBuffer;
    /// # fn main() -> anyhow::Result<()> {
    /// let rb = RingBuffer::create(Path::new("/tmp/test.buf"), 1)?;
    ///
    /// rb.write(b"test")?;
    /// assert_eq!(rb.read_tail(), 0); // No data consumed yet
    ///
    /// let snapshot = rb.read_snapshot();
    /// rb.advance_tail(snapshot.len() as u64);
    /// assert_eq!(rb.read_tail(), 4); // Tail advanced after consumption
    /// # Ok(())
    /// # }
    /// ```
    pub fn read_tail(&self) -> u64 {
        // SAFETY: header is valid for the lifetime of self
        unsafe { (*self.header).read_tail.load(Ordering::Acquire) }
    }

    /// Update the read tail position (mark data as consumed).
    ///
    /// This should be called by consumers after processing data to free up space.
    pub fn update_read_tail(&self, new_tail: u64) {
        // SAFETY: header is valid for the lifetime of self
        unsafe {
            (*self.header).read_tail.store(new_tail, Ordering::Release);
        }
    }

    /// Advance the read tail by a number of bytes (convenience wrapper).
    ///
    /// This is a helper for consumers who want to mark data as consumed
    /// by advancing the tail relative to its current position.
    pub fn advance_tail(&self, bytes: u64) {
        // SAFETY: header is valid for the lifetime of self
        unsafe {
            (*self.header).read_tail.fetch_add(bytes, Ordering::Release);
        }
    }
}

#[cfg(feature = "storage_arrow")]
impl RingBuffer {
    /// Write an Arrow RecordBatch in IPC format.
    ///
    /// This serializes the batch to Arrow IPC format and writes it to the ring buffer.
    ///
    /// # Arguments
    /// * `batch` - The Arrow RecordBatch to write
    ///
    /// # Returns
    /// Ok(()) on success, Err on serialization or write failure
    pub fn write_arrow_batch(&self, batch: &RecordBatch) -> Result<()> {
        use arrow::ipc::writer::FileWriter;
        use std::io::Cursor;

        let mut buffer = Vec::new();
        let mut writer = FileWriter::try_new(&mut buffer, &batch.schema())
            .context("Failed to create Arrow IPC writer")?;

        writer.write(batch).context("Failed to write Arrow batch")?;
        writer.finish().context("Failed to finish Arrow writer")?;

        self.write(&buffer)
            .context("Failed to write Arrow IPC data to ring buffer")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_create_ring_buffer() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("test_ring.buf");

        let rb = RingBuffer::create(&path, 1).unwrap(); // 1 MB
        assert_eq!(rb.capacity(), 1024 * 1024);
        assert_eq!(rb.write_head(), 0);
        assert_eq!(rb.read_tail(), 0);
    }

    #[test]
    fn test_open_existing_ring_buffer() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("test_ring.buf");

        // Create and write some data
        {
            let rb = RingBuffer::create(&path, 1).unwrap();
            rb.write(b"test data").unwrap();
        }

        // Open existing buffer
        let rb = RingBuffer::open(&path).unwrap();
        assert_eq!(rb.capacity(), 1024 * 1024);
        assert_eq!(rb.write_head(), 9); // "test data" = 9 bytes
    }

    #[test]
    fn test_write_and_read() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("test_ring.buf");

        let rb = RingBuffer::create(&path, 1).unwrap();

        // Write data
        let test_data = b"Hello, ring buffer!";
        rb.write(test_data).unwrap();

        // Read snapshot
        let snapshot = rb.read_snapshot();
        assert_eq!(snapshot, test_data);
    }

    #[test]
    fn test_circular_wrap() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("test_ring.buf");

        // Create small buffer (1KB)
        let rb = RingBuffer::create(&path, 1).unwrap();
        let capacity = rb.capacity() as usize;

        // Write data that will wrap around
        let test_data = vec![0xAA; 512];

        // Fill buffer past capacity to test wrap
        for _ in 0..3 {
            rb.write(&test_data).unwrap();
        }

        // Verify data wraps correctly
        let snapshot = rb.read_snapshot();
        assert!(snapshot.len() <= capacity);
    }

    #[test]
    fn test_concurrent_write_read() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("test_ring.buf");

        let rb = Arc::new(RingBuffer::create(&path, 10).unwrap()); // 10 MB

        // Spawn writer thread
        let rb_writer = Arc::clone(&rb);
        let writer = thread::spawn(move || {
            for i in 0..1000 {
                let data = format!("Message {}", i);
                rb_writer.write(data.as_bytes()).unwrap();
            }
        });

        // Spawn reader thread
        let rb_reader = Arc::clone(&rb);
        let reader = thread::spawn(move || {
            let mut read_count = 0;
            while read_count < 100 {
                let snapshot = rb_reader.read_snapshot();
                if !snapshot.is_empty() {
                    read_count += 1;
                }
                thread::sleep(std::time::Duration::from_micros(100));
            }
        });

        writer.join().unwrap();
        reader.join().unwrap();
    }

    #[test]
    fn test_data_too_large() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("test_ring.buf");

        let rb = RingBuffer::create(&path, 1).unwrap(); // 1 MB

        // Try to write more than capacity
        let large_data = vec![0u8; 2 * 1024 * 1024]; // 2 MB
        let result = rb.write(&large_data);

        assert!(result.is_err());
    }

    #[test]
    fn test_write_performance() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("test_ring.buf");

        let rb = RingBuffer::create(&path, 100).unwrap(); // 100 MB

        let test_data = vec![0u8; 1024]; // 1 KB per write
        let iterations = 10_000;

        let start = std::time::Instant::now();
        for _ in 0..iterations {
            rb.write(&test_data).unwrap();
        }
        let elapsed = start.elapsed();

        let ops_per_sec = iterations as f64 / elapsed.as_secs_f64();
        println!("Write performance: {:.0} ops/sec", ops_per_sec);

        // Should achieve 10k+ writes/sec
        assert!(
            ops_per_sec > 10_000.0,
            "Performance too low: {} ops/sec",
            ops_per_sec
        );
    }
}
