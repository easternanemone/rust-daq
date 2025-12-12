use std::sync::Arc;

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
#[derive(Debug, Clone)]
pub struct Frame {
    /// Width in pixels
    pub width: u32,

    /// Height in pixels
    pub height: u32,

    /// Bits per pixel (e.g., 8, 12, 16)
    pub bit_depth: u32,

    /// Raw pixel data
    pub data: Vec<u8>,
}

impl Frame {
    /// Create a new frame from 16-bit pixel data.
    ///
    /// Copies the data into a byte vector.
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
        }
    }

    /// Create a new frame from 8-bit pixel data.
    pub fn from_u8(width: u32, height: u32, data: Vec<u8>) -> Self {
        Self {
            width,
            height,
            bit_depth: 8,
            data,
        }
    }

    /// Create a frame from raw byte data with explicit bit depth.
    ///
    /// The caller must ensure the buffer length matches the expected size for the bit depth.
    pub fn from_bytes(width: u32, height: u32, bit_depth: u32, data: Vec<u8>) -> Self {
        Self {
            width,
            height,
            bit_depth,
            data,
        }
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
        if self.data.len() % 2 != 0 {
            return None;
        }

        // SAFETY: Casting [u8] to [u16] is valid if alignment is respected.
        // Vec<u8> is not guaranteed to be u16 aligned, so we rely on `align_to`.
        // Ideally we would use `bytemuck::cast_slice`, but we want to avoid deps if possible.
        // For now, we will perform a check-and-cast.
        let (prefix, mid, suffix) = unsafe { self.data.align_to::<u16>() };

        if !prefix.is_empty() || !suffix.is_empty() {
            // Alignment mismatch. If this happens often, we should change storage to `Vec<u16>` or use `Bytes`.
            // For now, return None.
            // In practice, allocators usually return aligned memory enough for u16.
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
