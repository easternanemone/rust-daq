//! Frame timing probe to understand get_oldest_frame vs get_latest_frame semantics
//!
//! Tests whether "oldest" and "latest" refer to:
//! - Chronological capture time (oldest = captured first)
//! - Stack/buffer position (oldest = first slot, latest = last slot)
//!
//! Run with:
//! ```bash
//! ssh maitai@100.117.5.12 'source /etc/profile.d/pvcam.sh && \
//!   export PVCAM_SDK_DIR=/opt/pvcam/sdk && \
//!   export LIBRARY_PATH=/opt/pvcam/library/x86_64:$LIBRARY_PATH && \
//!   export LD_LIBRARY_PATH=/opt/pvcam/library/x86_64:$LD_LIBRARY_PATH && \
//!   cd ~/rust-daq && git pull && \
//!   cargo test --release -p daq-driver-pvcam --features pvcam_hardware \
//!     --test frame_timing_probe -- --nocapture --test-threads=1'
//! ```

#![cfg(not(target_arch = "wasm32"))]
#![cfg(feature = "pvcam_hardware")]
#![allow(clippy::unwrap_used, clippy::expect_used, unused_imports, dead_code)]

use pvcam_sys::*;
use std::ffi::{c_void, CStr};
use std::time::{Duration, Instant};

// Buffer mode constants
const CIRC_NO_OVERWRITE: i16 = 1;
const CCS_HALT: i16 = 1;

// Exposure mode
const EXT_TRIG_INTERNAL: i16 = 1792;
const EXPOSE_OUT_FIRST_ROW: i16 = 0;

/// Get PVCAM error message
fn get_error_message() -> String {
    let mut msg = [0i8; 256];
    unsafe {
        let code = pl_error_code();
        pl_error_message(code, msg.as_mut_ptr());
        CStr::from_ptr(msg.as_ptr()).to_string_lossy().into_owned()
    }
}

/// Frame metadata for analysis
#[derive(Debug, Clone)]
struct FrameMetadata {
    frame_nr: i32,
    timestamp: u64,       // TimeStamp field (100ns resolution typically)
    timestamp_bof: u64,   // Beginning of frame timestamp
    readout_time: i32,    // Readout time in microseconds
    retrieval_method: &'static str,
    wall_clock_ms: u128,  // Wall clock time since test start
}

#[tokio::test]
async fn test_frame_timing_semantics() {
    println!("\n=== FRAME TIMING PROBE: get_oldest_frame vs get_latest_frame ===\n");
    println!("Goal: Determine if 'oldest'/'latest' refers to capture time or buffer position\n");

    // Initialize PVCAM
    unsafe {
        if pl_pvcam_init() == 0 {
            panic!("Failed to initialize PVCAM: {}", get_error_message());
        }
    }
    println!("[OK] PVCAM initialized");

    // Get camera count and open
    let mut cam_count: i16 = 0;
    unsafe {
        if pl_cam_get_total(&mut cam_count) == 0 || cam_count == 0 {
            pl_pvcam_uninit();
            panic!("No cameras found");
        }
    }

    let mut cam_name = [0i8; 32];
    unsafe { pl_cam_get_name(0, cam_name.as_mut_ptr()); }
    let name = unsafe { CStr::from_ptr(cam_name.as_ptr()) }
        .to_string_lossy()
        .into_owned();
    println!("[OK] Camera: {}", name);

    let mut hcam: i16 = 0;
    unsafe {
        if pl_cam_open(cam_name.as_mut_ptr(), &mut hcam, 0) == 0 {
            pl_pvcam_uninit();
            panic!("Failed to open camera: {}", get_error_message());
        }
    }
    println!("[OK] Camera opened, handle = {}\n", hcam);

    // Use small ROI for speed but long exposure to separate frames in time
    let region = rgn_type {
        s1: 0, s2: 255, sbin: 1,
        p1: 0, p2: 255, pbin: 1,
    };

    // LONG exposure to make timing differences clear
    let exposure_ms: uns32 = 500;  // 500ms exposure = 2 FPS max
    let mut frame_bytes: uns32 = 0;
    let exp_mode = EXT_TRIG_INTERNAL | EXPOSE_OUT_FIRST_ROW;
    let buffer_frames = 10u32;  // Room for several frames

    println!("Configuration:");
    println!("  Exposure: {}ms (long, for clear timing separation)", exposure_ms);
    println!("  Buffer frames: {}", buffer_frames);
    println!("  ROI: 256x256\n");

    // Setup continuous acquisition
    let setup_ok = unsafe {
        pl_exp_setup_cont(
            hcam, 1, &region as *const _, exp_mode,
            exposure_ms, &mut frame_bytes, CIRC_NO_OVERWRITE,
        ) != 0
    };

    if !setup_ok {
        println!("[FAIL] setup_cont failed: {}", get_error_message());
        unsafe { pl_cam_close(hcam); pl_pvcam_uninit(); }
        return;
    }
    println!("[OK] setup_cont succeeded, frame_bytes = {}", frame_bytes);

    // Allocate buffer
    let buffer_size = (frame_bytes as usize) * (buffer_frames as usize);
    let mut buffer = vec![0u8; buffer_size];

    // Start acquisition
    let start_ok = unsafe {
        pl_exp_start_cont(hcam, buffer.as_mut_ptr() as *mut c_void, buffer_size as uns32) != 0
    };

    if !start_ok {
        println!("[FAIL] start_cont failed: {}", get_error_message());
        unsafe { pl_cam_close(hcam); pl_pvcam_uninit(); }
        return;
    }
    println!("[OK] start_cont succeeded\n");

    let test_start = Instant::now();
    let mut oldest_frames: Vec<FrameMetadata> = Vec::new();
    let mut latest_frames: Vec<FrameMetadata> = Vec::new();

    println!("=== Phase 1: Let buffer fill with frames (waiting 3 seconds) ===\n");

    // Wait for buffer to fill with several frames
    std::thread::sleep(Duration::from_secs(3));

    println!("=== Phase 2: Retrieve frames using BOTH methods ===\n");

    // Try get_oldest_frame multiple times
    println!("--- Testing pl_exp_get_oldest_frame_ex ---");
    for i in 0..5 {
        let mut address: *mut c_void = std::ptr::null_mut();
        let mut fi: FRAME_INFO = unsafe { std::mem::zeroed() };

        let result = unsafe { pl_exp_get_oldest_frame_ex(hcam, &mut address, &mut fi) };

        if result != 0 && !address.is_null() {
            let meta = FrameMetadata {
                frame_nr: fi.FrameNr,
                timestamp: fi.TimeStamp as u64,
                timestamp_bof: fi.TimeStampBOF as u64,
                readout_time: fi.ReadoutTime,
                retrieval_method: "get_oldest_frame",
                wall_clock_ms: test_start.elapsed().as_millis(),
            };
            println!("  [{}] FrameNr={}, TimeStamp={}, TimeStampBOF={}, ReadoutTime={}",
                     i, meta.frame_nr, meta.timestamp, meta.timestamp_bof, meta.readout_time);
            oldest_frames.push(meta);

            // Unlock so we can get next frame
            unsafe { pl_exp_unlock_oldest_frame(hcam); }
        } else {
            println!("  [{}] No frame available", i);
            break;
        }

        std::thread::sleep(Duration::from_millis(100));
    }

    println!("\n--- Testing pl_exp_get_latest_frame_ex ---");
    for i in 0..5 {
        let mut address: *mut c_void = std::ptr::null_mut();
        let mut fi: FRAME_INFO = unsafe { std::mem::zeroed() };

        let result = unsafe { pl_exp_get_latest_frame_ex(hcam, &mut address, &mut fi) };

        if result != 0 && !address.is_null() {
            let meta = FrameMetadata {
                frame_nr: fi.FrameNr,
                timestamp: fi.TimeStamp as u64,
                timestamp_bof: fi.TimeStampBOF as u64,
                readout_time: fi.ReadoutTime,
                retrieval_method: "get_latest_frame",
                wall_clock_ms: test_start.elapsed().as_millis(),
            };
            println!("  [{}] FrameNr={}, TimeStamp={}, TimeStampBOF={}, ReadoutTime={}",
                     i, meta.frame_nr, meta.timestamp, meta.timestamp_bof, meta.readout_time);
            latest_frames.push(meta);
            // Note: get_latest_frame doesn't need unlock
        } else {
            println!("  [{}] No frame available", i);
            break;
        }

        std::thread::sleep(Duration::from_millis(100));
    }

    // Stop acquisition
    unsafe { pl_exp_abort(hcam, CCS_HALT); }

    // Analysis
    println!("\n=== ANALYSIS ===\n");

    if !oldest_frames.is_empty() && !latest_frames.is_empty() {
        let oldest_first = &oldest_frames[0];
        let latest_first = &latest_frames[0];

        println!("First frame from get_oldest_frame:");
        println!("  FrameNr: {}, TimeStamp: {}", oldest_first.frame_nr, oldest_first.timestamp);

        println!("\nFirst frame from get_latest_frame:");
        println!("  FrameNr: {}, TimeStamp: {}", latest_first.frame_nr, latest_first.timestamp);

        println!("\n--- INTERPRETATION ---");

        if oldest_first.frame_nr < latest_first.frame_nr {
            println!("✓ get_oldest_frame returns LOWER FrameNr ({} < {})",
                     oldest_first.frame_nr, latest_first.frame_nr);
            println!("  → 'oldest' = chronologically older (captured earlier)");
            println!("  → 'latest' = chronologically newer (captured later)");
            println!("\n  NAMING IS CHRONOLOGICAL (as expected)");
        } else if oldest_first.frame_nr > latest_first.frame_nr {
            println!("✗ get_oldest_frame returns HIGHER FrameNr ({} > {})",
                     oldest_first.frame_nr, latest_first.frame_nr);
            println!("  → 'oldest' = newest in capture time!");
            println!("  → 'latest' = oldest in capture time!");
            println!("\n  *** NAMING IS INVERTED (stack position, not time) ***");
        } else {
            println!("? Same FrameNr - inconclusive (only one frame in buffer?)");
        }

        if oldest_first.timestamp != latest_first.timestamp {
            println!("\nTimestamp comparison:");
            if oldest_first.timestamp < latest_first.timestamp {
                println!("  oldest.TimeStamp ({}) < latest.TimeStamp ({})",
                         oldest_first.timestamp, latest_first.timestamp);
                println!("  → Confirms: 'oldest' = earlier capture time");
            } else {
                println!("  oldest.TimeStamp ({}) > latest.TimeStamp ({})",
                         oldest_first.timestamp, latest_first.timestamp);
                println!("  → Confirms: NAMING IS INVERTED!");
            }
        }
    } else {
        println!("Insufficient data for comparison.");
        println!("oldest_frames collected: {}", oldest_frames.len());
        println!("latest_frames collected: {}", latest_frames.len());
    }

    // Print all frame numbers for debugging
    println!("\n--- All collected frame numbers ---");
    println!("get_oldest_frame: {:?}", oldest_frames.iter().map(|f| f.frame_nr).collect::<Vec<_>>());
    println!("get_latest_frame: {:?}", latest_frames.iter().map(|f| f.frame_nr).collect::<Vec<_>>());

    // Cleanup
    println!("\n--- Cleanup ---");
    unsafe {
        pl_cam_close(hcam);
        pl_pvcam_uninit();
    }
    println!("[OK] Done\n");
}
