use anyhow::Result;
use async_trait::async_trait;

use tokio::sync::broadcast;

/// A source of measurement data (e.g., Camera, Sensor).
///
/// Produces data items of type `Output`.
#[async_trait]
pub trait MeasurementSource: Send + Sync {
    type Output: Clone + Send + Sync + 'static;

    /// Subscribe to the data stream.
    ///
    /// Returns a broadcast receiver that will receive the data.
    async fn subscribe(&self) -> Result<broadcast::Receiver<Self::Output>>;
}

/// A processor that transforms measurement data.
///
/// Consumes `Input` and produces `Output`.
/// Examples: Background subtraction, FFT, Peak Finding.
#[async_trait]
pub trait MeasurementProcessor: Send + Sync {
    type Input: Clone + Send + Sync + 'static;
    type Output: Clone + Send + Sync + 'static;

    /// Process a single input item.
    ///
    /// This is typically called by a runner loop that subscribes to a source
    /// and feeds the processor.
    async fn process(&mut self, input: Self::Input) -> Result<Self::Output>;
}

/// A sink that consumes measurement data.
///
/// Examples: HDF5Writer, NetworkSender, GuiPlot.
#[async_trait]
pub trait MeasurementSink: Send + Sync {
    type Input: Clone + Send + Sync + 'static;

    /// Consume a data item.
    async fn send(&mut self, input: Self::Input) -> Result<()>;
}

/// A pipeline node that connects a Source to a Sink or another Processor.
///
/// This struct helps manage the lifecycle of a processing task.
pub struct PipelineNode {
    // Implementation details for running the connecting loop
}
