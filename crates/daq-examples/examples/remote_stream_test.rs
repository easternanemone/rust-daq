//! Remote gRPC streaming test
//!
//! Tests frame streaming from a remote daemon to verify:
//! - Tailscale direct connectivity (no SSH tunnel needed)
//! - LZ4 compression working correctly
//! - Compression ratio metrics
//!
//! Usage:
//! ```bash
//! # Connect to remote daemon (replace IP with your Tailscale address)
//! DAQ_DAEMON_URL=http://100.117.5.12:50051 cargo run --example remote_stream_test
//! ```

use daq_proto::daq::{
    hardware_service_client::HardwareServiceClient, ListDevicesRequest, StartStreamRequest,
    StopStreamRequest, StreamFramesRequest,
};
use futures::StreamExt;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let endpoint = env::var("DAQ_DAEMON_URL").unwrap_or_else(|_| "http://127.0.0.1:50051".into());
    println!("Connecting to daemon at {}", endpoint);

    let channel = tonic::transport::Channel::from_shared(endpoint.clone())?
        .connect_timeout(std::time::Duration::from_secs(10))
        .connect()
        .await?;

    // Increase max message size for camera frames (100MB limit matches server)
    let mut client = HardwareServiceClient::new(channel)
        .max_decoding_message_size(100 * 1024 * 1024)
        .max_encoding_message_size(100 * 1024 * 1024);

    // List devices first
    let devices = client
        .list_devices(ListDevicesRequest {
            capability_filter: None,
        })
        .await?;
    println!("\nAvailable devices:");
    for d in &devices.get_ref().devices {
        println!("  - {} ({}) [frame_producer={}]", d.id, d.driver_type, d.is_frame_producer);
    }

    // Find the camera (is_frame_producer = true)
    let camera_id = devices
        .get_ref()
        .devices
        .iter()
        .find(|d| d.is_frame_producer || d.id == "camera")
        .map(|d| d.id.clone());

    let Some(cam) = camera_id else {
        println!("\nNo camera found. Available devices listed above.");
        return Ok(());
    };

    println!("\nStarting stream from {}...", cam);

    // Start stream (None for frame_count = continuous)
    client
        .start_stream(StartStreamRequest {
            device_id: cam.clone(),
            frame_count: None,
        })
        .await?;

    // Stream a few frames
    let request = StreamFramesRequest {
        device_id: cam.clone(),
        max_fps: 30, // Rate limit for GUI rendering
    };
    let mut stream = client.stream_frames(request).await?.into_inner();

    let mut count = 0;
    let mut total_uncompressed = 0usize;
    let mut total_compressed = 0usize;

    println!("\nReceiving frames (press Ctrl+C to stop):");
    println!("{}", "─".repeat(70));

    while let Some(frame_result) = stream.next().await {
        if count >= 10 {
            break;
        }
        match frame_result {
            Ok(frame) => {
                let compression_type = frame.compression;
                let compressed_size = frame.data.len();
                let uncompressed_size = frame.uncompressed_size as usize;
                let ratio = if compressed_size > 0 {
                    uncompressed_size as f64 / compressed_size as f64
                } else {
                    1.0
                };

                total_uncompressed += uncompressed_size;
                total_compressed += compressed_size;

                // compression=1 means LZ4, compression=0 means none
                let comp_str = if compression_type == 1 { "LZ4" } else { "none" };

                println!(
                    "Frame {:4}: {:4}x{:4}  {} {:6} KB → {:6} KB  ({:.1}x)",
                    frame.frame_number,
                    frame.width,
                    frame.height,
                    comp_str,
                    uncompressed_size / 1024,
                    compressed_size / 1024,
                    ratio
                );
                count += 1;
            }
            Err(e) => {
                println!("Stream error: {}", e);
                break;
            }
        }
    }

    println!("{}", "─".repeat(70));

    // Print summary
    if total_compressed > 0 {
        let overall_ratio = total_uncompressed as f64 / total_compressed as f64;
        println!(
            "\nSummary: {} frames, {} MB → {} MB ({:.1}x compression)",
            count,
            total_uncompressed / 1024 / 1024,
            total_compressed / 1024 / 1024,
            overall_ratio
        );
    }

    // Stop stream
    client
        .stop_stream(StopStreamRequest { device_id: cam })
        .await?;
    println!("Stream stopped");

    Ok(())
}
