# MeasurementProcessor Developer Guide

A practical guide for implementing the new `MeasurementProcessor` trait that works with structured `Measurement` data instead of scalar-only `DataPoint`.

## Quick Start

### Basic Implementation

```rust
use rust_daq::core::{MeasurementProcessor, Measurement, DataPoint};

struct MyProcessor {
    // Your state here
}

impl MeasurementProcessor for MyProcessor {
    fn process_measurements(&mut self, data: &[Measurement]) -> Vec<Measurement> {
        let mut results = Vec::new();
        
        for measurement in data {
            match measurement {
                Measurement::Scalar(datapoint) => {
                    // Process scalar data
                    if let Some(processed) = self.process_scalar(datapoint) {
                        results.push(Measurement::Scalar(processed));
                    }
                }
                Measurement::Spectrum(spectrum) => {
                    // Process spectrum data
                    if let Some(processed) = self.process_spectrum(spectrum) {
                        results.push(Measurement::Spectrum(processed));
                    }
                }
                Measurement::Image(image) => {
                    // Process image data
                    if let Some(processed) = self.process_image(image) {
                        results.push(Measurement::Image(processed));
                    }
                }
            }
        }
        
        results
    }
}
```

## Common Patterns

### 1. Type Filtering

Process only specific measurement types:

```rust
impl MeasurementProcessor for ScalarOnlyFilter {
    fn process_measurements(&mut self, data: &[Measurement]) -> Vec<Measurement> {
        data.iter()
            .filter_map(|m| match m {
                Measurement::Scalar(dp) => {
                    let filtered = self.apply_filter(dp.value);
                    Some(Measurement::Scalar(DataPoint {
                        value: filtered,
                        ..dp.clone()
                    }))
                }
                _ => None, // Skip non-scalar measurements
            })
            .collect()
    }
}
```

### 2. Type Conversion

Transform one measurement type to another:

```rust
impl MeasurementProcessor for FFTProcessor {
    fn process_measurements(&mut self, data: &[Measurement]) -> Vec<Measurement> {
        data.iter()
            .filter_map(|m| match m {
                Measurement::Scalar(dp) => {
                    // Convert scalar time-series to spectrum
                    self.compute_fft(dp).map(Measurement::Spectrum)
                }
                _ => None,
            })
            .collect()
    }
}
```

### 3. Multi-Output Processing

Generate multiple measurements from one input:

```rust
impl MeasurementProcessor for StatisticsProcessor {
    fn process_measurements(&mut self, data: &[Measurement]) -> Vec<Measurement> {
        let mut results = Vec::new();
        
        for measurement in data {
            if let Measurement::Spectrum(spectrum) = measurement {
                let timestamp = spectrum.timestamp;
                let base_channel = &spectrum.channel;
                
                // Generate multiple statistics as scalar measurements
                results.push(Measurement::Scalar(DataPoint {
                    timestamp,
                    channel: format!("{}_peak_freq", base_channel),
                    value: self.find_peak_frequency(&spectrum.bins),
                    unit: "Hz".to_string(),
                    metadata: None,
                }));
                
                results.push(Measurement::Scalar(DataPoint {
                    timestamp,
                    channel: format!("{}_total_power", base_channel),
                    value: self.calculate_total_power(&spectrum.bins),
                    unit: "dB".to_string(),
                    metadata: None,
                }));
            }
        }
        
        results
    }
}
```

### 4. Wrapping Legacy DataProcessor

Migrate existing `DataProcessor` implementations:

```rust
struct LegacyFilter {
    // existing fields
}

impl DataProcessor for LegacyFilter {
    fn process(&mut self, data: &[DataPoint]) -> Vec<DataPoint> {
        // existing implementation
    }
}

// Add MeasurementProcessor support
impl MeasurementProcessor for LegacyFilter {
    fn process_measurements(&mut self, data: &[Measurement]) -> Vec<Measurement> {
        // Extract scalar measurements
        let scalars: Vec<DataPoint> = data.iter()
            .filter_map(|m| match m {
                Measurement::Scalar(dp) => Some(dp.clone()),
                _ => None,
            })
            .collect();
        
        // Use existing DataProcessor implementation
        let processed = self.process(&scalars);
        
        // Convert back to measurements
        processed.into_iter()
            .map(Measurement::Scalar)
            .collect()
    }
}
```

## Working with Specific Measurement Types

### Spectrum Data

```rust
fn process_spectrum(&mut self, spectrum: &SpectrumData) -> Option<SpectrumData> {
    // Access frequency bins directly - no JSON parsing!
    let mut new_bins = Vec::new();
    
    for bin in &spectrum.bins {
        if bin.frequency > 1000.0 && bin.frequency < 5000.0 {
            // Apply processing to specific frequency range
            let processed_magnitude = self.apply_spectral_filter(bin.magnitude);
            new_bins.push(FrequencyBin {
                frequency: bin.frequency,
                magnitude: processed_magnitude,
            });
        }
    }
    
    if new_bins.is_empty() {
        return None;
    }
    
    Some(SpectrumData {
        timestamp: spectrum.timestamp,
        channel: format!("{}_filtered", spectrum.channel),
        unit: spectrum.unit.clone(),
        bins: new_bins,
        metadata: Some(serde_json::json!({
            "filter_type": "bandpass",
            "freq_range": [1000.0, 5000.0],
            "original_bins": spectrum.bins.len(),
            "filtered_bins": new_bins.len(),
        })),
    })
}
```

### Image Data

```rust
fn process_image(&mut self, image: &ImageData) -> Option<ImageData> {
    // Process 2D pixel data
    let processed_pixels: Vec<f64> = image.pixels.iter()
        .map(|&pixel| self.apply_image_filter(pixel))
        .collect();
    
    Some(ImageData {
        timestamp: image.timestamp,
        channel: format!("{}_processed", image.channel),
        width: image.width,
        height: image.height,
        pixels: processed_pixels,
        unit: image.unit.clone(),
        metadata: Some(serde_json::json!({
            "processing": "gaussian_blur",
            "kernel_size": 3,
        })),
    })
}
```

## Error Handling

### Robust Processing

```rust
impl MeasurementProcessor for RobustProcessor {
    fn process_measurements(&mut self, data: &[Measurement]) -> Vec<Measurement> {
        data.iter()
            .filter_map(|measurement| {
                match self.try_process_measurement(measurement) {
                    Ok(Some(result)) => Some(result),
                    Ok(None) => None, // Intentionally filtered out
                    Err(e) => {
                        log::warn!("Failed to process measurement from channel '{}': {}", 
                                 measurement.channel(), e);
                        None // Skip failed measurements
                    }
                }
            })
            .collect()
    }
    
    fn try_process_measurement(&mut self, measurement: &Measurement) -> Result<Option<Measurement>, ProcessingError> {
        match measurement {
            Measurement::Scalar(dp) => self.try_process_scalar(dp),
            Measurement::Spectrum(spec) => self.try_process_spectrum(spec),
            Measurement::Image(img) => self.try_process_image(img),
        }
    }
}
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use rust_daq::core::{SpectrumData, FrequencyBin};
    
    #[test]
    fn test_spectral_processor() {
        let mut processor = SpectralProcessor::new();
        
        let input = vec![
            Measurement::Spectrum(SpectrumData {
                timestamp: Utc::now(),
                channel: "test_signal_fft".to_string(),
                unit: "dB".to_string(),
                bins: vec![
                    FrequencyBin { frequency: 500.0, magnitude: -30.0 },
                    FrequencyBin { frequency: 1500.0, magnitude: -10.0 }, // Should pass filter
                    FrequencyBin { frequency: 6000.0, magnitude: -20.0 },
                ],
                metadata: None,
            })
        ];
        
        let results = processor.process_measurements(&input);
        
        assert_eq!(results.len(), 1);
        match &results[0] {
            Measurement::Spectrum(spec) => {
                assert_eq!(spec.bins.len(), 1); // Only 1500 Hz bin passes
                assert_eq!(spec.bins[0].frequency, 1500.0);
                assert!(spec.channel.ends_with("_filtered"));
            }
            _ => panic!("Expected spectrum output"),
        }
    }
    
    #[test]
    fn test_mixed_input_types() {
        let mut processor = MixedProcessor::new();
        
        let input = vec![
            Measurement::Scalar(DataPoint {
                timestamp: Utc::now(),
                channel: "temp".to_string(),
                value: 23.5,
                unit: "°C".to_string(),
                metadata: None,
            }),
            Measurement::Spectrum(create_test_spectrum()),
        ];
        
        let results = processor.process_measurements(&input);
        
        // Verify processor handles mixed types correctly
        assert!(!results.is_empty());
    }
}
```

## Performance Considerations

### Memory Efficiency

```rust
impl MeasurementProcessor for EfficientProcessor {
    fn process_measurements(&mut self, data: &[Measurement]) -> Vec<Measurement> {
        // Pre-allocate with estimated capacity
        let mut results = Vec::with_capacity(data.len());
        
        for measurement in data {
            match measurement {
                Measurement::Spectrum(spectrum) => {
                    // Reuse existing spectrum structure when possible
                    if let Some(processed) = self.process_spectrum_in_place(spectrum) {
                        results.push(Measurement::Spectrum(processed));
                    }
                }
                _ => {} // Handle other types
            }
        }
        
        results
    }
    
    fn process_spectrum_in_place(&mut self, spectrum: &SpectrumData) -> Option<SpectrumData> {
        // Modify bins in place to avoid allocations
        let mut new_spectrum = spectrum.clone();
        for bin in &mut new_spectrum.bins {
            bin.magnitude = self.process_magnitude(bin.magnitude);
        }
        Some(new_spectrum)
    }
}
```

## Migration Checklist

- [ ] Identify processors that could benefit from structured data
- [ ] Implement `MeasurementProcessor` alongside existing `DataProcessor`
- [ ] Add comprehensive tests for new measurement types
- [ ] Update documentation with specific measurement type handling
- [ ] Consider performance implications of data copying vs. in-place processing
- [ ] Plan for future measurement types your domain might need

## See Also

- [Measurement Enum Migration Guide](../MEASUREMENT_ENUM_MIGRATION.md) - Complete architectural overview
- [FFT Processor Example](../src/data/fft.rs) - Real implementation example
- [Integration Tests](../tests/measurement_enum_test.rs) - Usage demonstrations
- [Core Types](../src/core.rs) - Full type definitions
