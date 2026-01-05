use std::collections::HashMap;
use std::sync::Arc;

/// Extended metadata for frames (bd-183h).
///
/// Stored as `Option<Box<FrameMetadata>>` to avoid allocation overhead
/// for frames that don't need extended metadata.
#[derive(Debug, Clone, Default)]
pub struct FrameMetadata {
    /// Sensor temperature at capture time (Celsius)
    pub temperature_c: Option<f64>,

    /// Gain mode name (e.g., "HDR", "High Sensitivity")
    pub gain_mode: Option<String>,

    /// Readout speed (e.g., "100 MHz")
    pub readout_speed: Option<String>,

    /// Binning (x, y)
    pub binning: Option<(u16, u16)>,

    /// Trigger mode (e.g., "Timed", "Trigger First")
    pub trigger_mode: Option<String>,

    /// Extensible key-value metadata for driver-specific properties
    pub extra: HashMap<String, String>,
}

/// Represents a single image frame.
///
/// Designed to be flexible for FFI (C-compatible memory layout) and efficient storage.
///
/// # Storage
/// Data is stored as a raw byte vector (`Vec<u8>`).
/// - 8-bit images: 1 byte per pixel.
/// - 12/16-bit images: 2 bytes per pixel, Little Endian.
///
/// Use `as_u16_slice()` to access 16-bit data safely.
///
/// # Metadata (bd-183h)
/// Frames include timing and acquisition metadata for end-to-end traceability:
/// - `frame_number`: Driver-provided sequence number
/// - `timestamp_ns`: Capture timestamp from driver (nanoseconds since epoch)
/// - `exposure_ms`: Exposure time used for this frame
/// - `roi_x`, `roi_y`: ROI origin offset in sensor coordinates
/// - `metadata`: Optional extended metadata (temperature, gain, etc.)
#[derive(Debug, Clone)]
pub struct Frame {
    /// Width in pixels (of the captured ROI, not full sensor)
    pub width: u32,

    /// Height in pixels (of the captured ROI, not full sensor)
    pub height: u32,

    /// Bits per pixel (e.g., 8, 12, 16)
    pub bit_depth: u32,

    /// Raw pixel data
    pub data: Vec<u8>,

    // === Timing & Identification (bd-183h) ===
    /// Driver-provided frame sequence number (monotonically increasing)
    pub frame_number: u64,

    /// Capture timestamp from driver (nanoseconds since UNIX epoch)
    ///
    /// This is the time when the frame was captured by the camera hardware,
    /// not when it was received by the server. This enables accurate timing
    /// analysis without network jitter artifacts.
    pub timestamp_ns: u64,

    // === Acquisition Parameters (bd-183h) ===
    /// Exposure time used for this frame (milliseconds)
    pub exposure_ms: Option<f64>,

    /// ROI X offset in sensor coordinates (0 = left edge of sensor)
    pub roi_x: u32,

    /// ROI Y offset in sensor coordinates (0 = top edge of sensor)
    pub roi_y: u32,

    // === Extended Metadata (bd-183h) ===
    /// Optional extended metadata (temperature, gain, etc.)
    ///
    /// Boxed to avoid allocation overhead for frames without metadata.
    /// Use `with_metadata()` builder to set.
    pub metadata: Option<Box<FrameMetadata>>,
}

impl Frame {
    /// Create a new frame from 16-bit pixel data.
    ///
    /// Copies the data into a byte vector.
    /// Frame metadata fields default to zero/None; use builder methods to set.
    pub fn from_u16(width: u32, height: u32, pixels: &[u16]) -> Self {
        // Convert u16 pixels to u8 bytes (Little Endian)
        let mut data = Vec::with_capacity(pixels.len() * 2);
        for pixel in pixels {
            data.extend_from_slice(&pixel.to_le_bytes());
        }

        Self {
            width,
            height,
            bit_depth: 16,
            data,
            frame_number: 0,
            timestamp_ns: 0,
            exposure_ms: None,
            roi_x: 0,
            roi_y: 0,
            metadata: None,
        }
    }

    /// Create a new frame from 8-bit pixel data.
    ///
    /// Frame metadata fields default to zero/None; use builder methods to set.
    pub fn from_u8(width: u32, height: u32, data: Vec<u8>) -> Self {
        Self {
            width,
            height,
            bit_depth: 8,
            data,
            frame_number: 0,
            timestamp_ns: 0,
            exposure_ms: None,
            roi_x: 0,
            roi_y: 0,
            metadata: None,
        }
    }

    /// Create a frame from raw byte data with explicit bit depth.
    ///
    /// The caller must ensure the buffer length matches the expected size for the bit depth.
    /// Frame metadata fields default to zero/None; use builder methods to set.
    pub fn from_bytes(width: u32, height: u32, bit_depth: u32, data: Vec<u8>) -> Self {
        Self {
            width,
            height,
            bit_depth,
            data,
            frame_number: 0,
            timestamp_ns: 0,
            exposure_ms: None,
            roi_x: 0,
            roi_y: 0,
            metadata: None,
        }
    }

    // === Builder Methods (bd-183h) ===

    /// Set frame number (builder pattern).
    #[must_use]
    pub fn with_frame_number(mut self, frame_number: u64) -> Self {
        self.frame_number = frame_number;
        self
    }

    /// Set capture timestamp (builder pattern).
    #[must_use]
    pub fn with_timestamp(mut self, timestamp_ns: u64) -> Self {
        self.timestamp_ns = timestamp_ns;
        self
    }

    /// Set exposure time in milliseconds (builder pattern).
    #[must_use]
    pub fn with_exposure(mut self, exposure_ms: f64) -> Self {
        self.exposure_ms = Some(exposure_ms);
        self
    }

    /// Set ROI origin offset (builder pattern).
    #[must_use]
    pub fn with_roi_offset(mut self, roi_x: u32, roi_y: u32) -> Self {
        self.roi_x = roi_x;
        self.roi_y = roi_y;
        self
    }

    /// Set extended metadata (builder pattern).
    #[must_use]
    pub fn with_metadata(mut self, metadata: FrameMetadata) -> Self {
        self.metadata = Some(Box::new(metadata));
        self
    }

    /// Create timestamp from current system time.
    ///
    /// Utility for drivers that don't have hardware timestamps.
    pub fn timestamp_now() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0)
    }

    /// Get pixel value at (x, y) as u32 (handling bit depth conversion).
    pub fn get(&self, x: u32, y: u32) -> Option<u32> {
        if x >= self.width || y >= self.height {
            return None;
        }

        let idx = (y * self.width + x) as usize;

        match self.bit_depth {
            8 => self.data.get(idx).map(|&v| v as u32),
            12 | 16 => {
                let start = idx * 2;
                if start + 1 < self.data.len() {
                    let bytes = [self.data[start], self.data[start + 1]];
                    Some(u16::from_le_bytes(bytes) as u32)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Access data as u16 slice (if applicable).
    ///
    /// This uses `bytemuck` style casting which is safe given alignment,
    /// but for simplicity without deps: this requires the machine is Little Endian (standard for standard x86/ARM).
    ///
    /// Returns None if bit_depth is 8 or data length is invalid.
    pub fn as_u16_slice(&self) -> Option<&[u16]> {
        if self.bit_depth <= 8 {
            return None;
        }
        if !self.data.len().is_multiple_of(2) {
            return None;
        }

        // SAFETY: Casting [u8] to [u16] is valid if alignment is respected.
        // Vec<u8> is not guaranteed to be u16 aligned, so we rely on `align_to`.
        // Ideally we would use `bytemuck::cast_slice`, but we want to avoid deps if possible.
        // For now, we will perform a check-and-cast.
        let (prefix, mid, suffix) = unsafe { self.data.align_to::<u16>() };

        if !prefix.is_empty() || !suffix.is_empty() {
            // Alignment mismatch (bd-hnim). Log this so we know if it happens in production.
            // In practice, allocators usually return aligned memory enough for u16,
            // so this should be rare. If it happens often, change storage to Vec<u16>.
            tracing::warn!(
                prefix_len = prefix.len(),
                suffix_len = suffix.len(),
                data_len = self.data.len(),
                "Frame::as_u16_slice alignment mismatch - returning None (bd-hnim)"
            );
            return None;
        }

        Some(mid)
    }

    /// Calculate mean pixel value.
    pub fn mean(&self) -> f64 {
        match self.bit_depth {
            8 => {
                if self.data.is_empty() {
                    return 0.0;
                }
                let sum: u64 = self.data.iter().map(|&v| v as u64).sum();
                sum as f64 / self.data.len() as f64
            }
            16 => {
                let slice = self.as_u16_slice().unwrap_or(&[]);
                if slice.is_empty() {
                    return 0.0;
                }
                let sum: u64 = slice.iter().map(|&v| v as u64).sum();
                sum as f64 / slice.len() as f64
            }
            _ => 0.0,
        }
    }
}

/// Thread-safe frame reference for zero-copy sharing.
#[derive(Debug, Clone)]
pub struct FrameRef {
    pub width: u32,
    pub height: u32,
    pub stride: usize,
    data: Arc<[u8]>,
}

impl FrameRef {
    pub fn new(width: u32, height: u32, data: Vec<u8>, stride: usize) -> Self {
        Self {
            width,
            height,
            stride,
            data: data.into(),
        }
    }

    pub fn from_arc(width: u32, height: u32, data: Arc<[u8]>, stride: usize) -> Self {
        Self {
            width,
            height,
            stride,
            data,
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    pub fn data_arc(&self) -> Arc<[u8]> {
        Arc::clone(&self.data)
    }
}
