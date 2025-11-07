use crate::core::DataPoint;
use crate::measurement::Measure;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::mpsc;

// DataPoint implements Measure trait as a stub for backward compatibility with V1 modules
#[async_trait]
impl Measure for DataPoint {
    type Data = DataPoint;

    async fn measure(&mut self) -> Result<Self::Data> {
        Ok(self.clone())
    }

    async fn data_stream(&self) -> Result<mpsc::Receiver<Arc<Self::Data>>> {
        let (tx, rx) = mpsc::channel(1);
        tx.send(Arc::new(self.clone())).await.ok();
        Ok(rx)
    }
}
