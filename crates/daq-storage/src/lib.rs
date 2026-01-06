pub mod arrow_writer;
pub mod comedi_writer;
pub mod document_writer;
pub mod hdf5_writer;
pub mod ring_buffer;
pub mod ring_buffer_reader;
pub mod tap_registry;

pub use comedi_writer::{
    AcquisitionMetadata, ChannelConfig, ComediStreamWriter, ComediStreamWriterBuilder,
    CompressionType, ContinuousAcquisitionSession, StorageFormat, StreamStats,
};
pub use document_writer::DocumentWriter;
pub use hdf5_writer::HDF5Writer;
pub use ring_buffer::{AsyncRingBuffer, RingBuffer};
pub use ring_buffer_reader::{ReaderStats, RingBufferReader};

#[cfg(feature = "storage_arrow")]
pub use arrow_writer::ArrowDocumentWriter;
#[cfg(feature = "storage_parquet")]
pub use arrow_writer::ParquetDocumentWriter;
