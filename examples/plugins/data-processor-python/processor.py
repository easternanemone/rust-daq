"""
Python FFT Data Processor Module

A script-based module demonstrating Python integration via PyO3 for real-time
FFT analysis and signal processing of acquired data streams.

This module implements the rust_daq ScriptModule interface, providing:
- Real-time FFT analysis using NumPy
- Configurable windowing functions (Hanning, Hamming, Blackman, Kaiser)
- Overlapping FFT windows for continuous analysis
- Multiple output types (magnitude, power, PSD, phase)

Usage:
    The module is loaded by ScriptPluginLoader when the 'python' feature is enabled.
    Configure FFT parameters via the module configuration interface.
"""

import numpy as np
from typing import Dict, List, Any, Optional

# =============================================================================
# Module Configuration
# =============================================================================

# Default configuration values
config = {
    "fft_size": 1024,
    "window_type": "hanning",
    "overlap_percent": 50.0,
    "sample_rate_hz": 10000.0,
    "output_type": "magnitude",
}

# Module state
is_running = False
is_paused = False
sample_buffer: List[float] = []
fft_count = 0

# Precomputed window function
window: Optional[np.ndarray] = None


# =============================================================================
# Module Interface Functions (required by ScriptModule)
# =============================================================================


def module_type_info() -> Dict[str, Any]:
    """
    Return module metadata for registration with the module system.

    This function is called once when the module is loaded to extract
    type information for the module registry.

    Returns:
        Dictionary containing module metadata including type_id, parameters,
        roles, events, and data types.
    """
    return {
        "type_id": "python_fft_processor",
        "display_name": "Python FFT Processor",
        "description": "Real-time FFT analysis using NumPy for frequency domain analysis",
        "version": "1.0.0",
        "parameters": [
            {
                "param_id": "fft_size",
                "display_name": "FFT Size",
                "description": "Number of samples per FFT window (power of 2)",
                "param_type": "integer",
                "default_value": "1024",
                "min_value": "64",
                "max_value": "16384",
            },
            {
                "param_id": "window_type",
                "display_name": "Window Function",
                "description": "Windowing function applied before FFT",
                "param_type": "enum",
                "default_value": "hanning",
                "enum_values": ["none", "hanning", "hamming", "blackman", "kaiser"],
            },
            {
                "param_id": "overlap_percent",
                "display_name": "Overlap Percentage",
                "description": "Percentage overlap between FFT windows",
                "param_type": "float",
                "default_value": "50.0",
                "min_value": "0.0",
                "max_value": "90.0",
                "units": "%",
            },
            {
                "param_id": "sample_rate_hz",
                "display_name": "Sample Rate",
                "description": "Input signal sample rate",
                "param_type": "float",
                "default_value": "10000.0",
                "min_value": "1.0",
                "max_value": "1000000.0",
                "units": "Hz",
                "required": True,
            },
            {
                "param_id": "output_type",
                "display_name": "Output Type",
                "description": "Type of FFT output",
                "param_type": "enum",
                "default_value": "magnitude",
                "enum_values": ["magnitude", "power", "psd", "phase"],
            },
        ],
        "required_roles": [
            {
                "role_id": "data_source",
                "display_name": "Data Source",
                "description": "Device providing time-domain data",
                "required_capability": "readable",
                "allows_multiple": False,
            }
        ],
        "optional_roles": [],
        "event_types": ["fft_started", "fft_completed", "peak_detected", "error"],
        "data_types": ["fft_result", "frequency_bins", "peak_frequencies"],
    }


def configure(params: Dict[str, str]) -> List[str]:
    """
    Configure the module with the given parameters.

    Args:
        params: Dictionary of parameter names to string values.

    Returns:
        List of warning messages (empty if configuration succeeded without warnings).
    """
    global config, window
    warnings = []

    # FFT size (must be power of 2)
    if "fft_size" in params:
        try:
            size = int(params["fft_size"])
            if size < 64 or size > 16384:
                warnings.append(f"fft_size {size} clamped to valid range [64, 16384]")
                size = max(64, min(16384, size))
            # Round to nearest power of 2
            if size & (size - 1) != 0:
                next_pow2 = 1 << (size - 1).bit_length()
                warnings.append(f"fft_size rounded to power of 2: {next_pow2}")
                size = next_pow2
            config["fft_size"] = size
        except ValueError:
            warnings.append(f"Invalid fft_size '{params['fft_size']}', using default")

    # Window type
    if "window_type" in params:
        wtype = params["window_type"].lower()
        valid_windows = ["none", "hanning", "hamming", "blackman", "kaiser"]
        if wtype in valid_windows:
            config["window_type"] = wtype
        else:
            warnings.append(f"Invalid window_type '{wtype}', using 'hanning'")
            config["window_type"] = "hanning"

    # Overlap percentage
    if "overlap_percent" in params:
        try:
            overlap = float(params["overlap_percent"])
            if overlap < 0.0 or overlap > 90.0:
                warnings.append(f"overlap_percent {overlap} clamped to [0, 90]")
                overlap = max(0.0, min(90.0, overlap))
            config["overlap_percent"] = overlap
        except ValueError:
            warnings.append(f"Invalid overlap_percent, using default")

    # Sample rate
    if "sample_rate_hz" in params:
        try:
            rate = float(params["sample_rate_hz"])
            if rate < 1.0 or rate > 1000000.0:
                warnings.append(f"sample_rate_hz {rate} clamped to [1, 1000000]")
                rate = max(1.0, min(1000000.0, rate))
            config["sample_rate_hz"] = rate
        except ValueError:
            warnings.append(f"Invalid sample_rate_hz, using default")

    # Output type
    if "output_type" in params:
        otype = params["output_type"].lower()
        valid_outputs = ["magnitude", "power", "psd", "phase"]
        if otype in valid_outputs:
            config["output_type"] = otype
        else:
            warnings.append(f"Invalid output_type '{otype}', using 'magnitude'")

    # Precompute window function
    window = _create_window(config["fft_size"], config["window_type"])

    return warnings


def get_config() -> Dict[str, str]:
    """
    Return the current module configuration.

    Returns:
        Dictionary of parameter names to string values.
    """
    return {
        "fft_size": str(config["fft_size"]),
        "window_type": config["window_type"],
        "overlap_percent": str(config["overlap_percent"]),
        "sample_rate_hz": str(config["sample_rate_hz"]),
        "output_type": config["output_type"],
    }


def stage(ctx: Dict[str, Any]) -> None:
    """
    Prepare resources before starting the module.

    This initializes the sample buffer and precomputes the window function.

    Args:
        ctx: Module context containing module_id and other runtime info.
    """
    global sample_buffer, fft_count, window

    module_id = ctx.get("module_id", "unknown")
    print(f"[{module_id}] Staging Python FFT processor...")

    # Initialize buffer
    sample_buffer = []
    fft_count = 0

    # Precompute window if not done during configure
    if window is None:
        window = _create_window(config["fft_size"], config["window_type"])

    print(f"[{module_id}] FFT size: {config['fft_size']}, Window: {config['window_type']}")


def start(ctx: Dict[str, Any]) -> None:
    """
    Start the module execution.

    In a real implementation, this would receive data from the data_source role
    and perform continuous FFT analysis.

    Args:
        ctx: Module context containing module_id and runtime flags.
    """
    global is_running, fft_count

    module_id = ctx.get("module_id", "unknown")
    is_running = True

    print(f"[{module_id}] Starting Python FFT processor")
    print(f"[{module_id}] Sample rate: {config['sample_rate_hz']} Hz")
    print(f"[{module_id}] Output type: {config['output_type']}")

    # Demo: Generate synthetic test signal and process it
    # In production, data would come from the data_source role
    demo_signal = _generate_test_signal()
    fft_result = process_fft(demo_signal)

    if fft_result is not None:
        fft_count += 1
        freqs, magnitudes = fft_result

        # Find peak frequency
        peak_idx = np.argmax(magnitudes)
        peak_freq = freqs[peak_idx]
        peak_mag = magnitudes[peak_idx]

        print(f"[{module_id}] FFT #{fft_count} completed")
        print(f"[{module_id}] Peak frequency: {peak_freq:.2f} Hz, Magnitude: {peak_mag:.4f}")
        print(f"[{module_id}] Frequency resolution: {config['sample_rate_hz'] / config['fft_size']:.2f} Hz")


def pause() -> None:
    """Pause the module execution."""
    global is_paused
    is_paused = True
    print("Python FFT processor paused")


def resume() -> None:
    """Resume the module execution."""
    global is_paused
    is_paused = False
    print("Python FFT processor resumed")


def stop() -> None:
    """Stop the module execution."""
    global is_running
    is_running = False
    print(f"Python FFT processor stopped. Total FFTs computed: {fft_count}")


def unstage(ctx: Dict[str, Any]) -> None:
    """
    Release resources after stopping.

    Args:
        ctx: Module context.
    """
    global sample_buffer, window

    module_id = ctx.get("module_id", "unknown")
    print(f"[{module_id}] Unstaging Python FFT processor...")

    # Clear buffers
    sample_buffer = []
    window = None


# =============================================================================
# FFT Processing Functions
# =============================================================================


def process_fft(data: np.ndarray) -> Optional[tuple]:
    """
    Perform FFT on the input data.

    Args:
        data: Input time-domain samples (1D numpy array).

    Returns:
        Tuple of (frequencies, output_values) or None if insufficient data.
    """
    global window

    fft_size = config["fft_size"]
    sample_rate = config["sample_rate_hz"]
    output_type = config["output_type"]

    # Check data length
    if len(data) < fft_size:
        return None

    # Take the last fft_size samples
    segment = data[-fft_size:]

    # Apply window function
    if window is not None and config["window_type"] != "none":
        segment = segment * window

    # Compute FFT
    fft_result = np.fft.rfft(segment)

    # Compute frequency bins
    freqs = np.fft.rfftfreq(fft_size, d=1.0 / sample_rate)

    # Compute output based on type
    if output_type == "magnitude":
        output = np.abs(fft_result) / fft_size
    elif output_type == "power":
        output = np.abs(fft_result) ** 2 / fft_size
    elif output_type == "psd":
        # Power spectral density (normalize by sample rate and window)
        if window is not None and config["window_type"] != "none":
            window_power = np.sum(window**2)
        else:
            window_power = fft_size
        output = np.abs(fft_result) ** 2 / (sample_rate * window_power)
    elif output_type == "phase":
        output = np.angle(fft_result)
    else:
        output = np.abs(fft_result) / fft_size

    return freqs, output


def add_samples(samples: List[float]) -> List[tuple]:
    """
    Add samples to the buffer and process complete FFT windows.

    Args:
        samples: New samples to add.

    Returns:
        List of FFT results for any complete windows.
    """
    global sample_buffer, fft_count

    sample_buffer.extend(samples)
    results = []

    fft_size = config["fft_size"]
    overlap = config["overlap_percent"] / 100.0
    hop_size = int(fft_size * (1.0 - overlap))

    # Process complete windows
    while len(sample_buffer) >= fft_size:
        segment = np.array(sample_buffer[:fft_size])
        result = process_fft(segment)
        if result is not None:
            results.append(result)
            fft_count += 1

        # Advance by hop size
        sample_buffer = sample_buffer[hop_size:]

    return results


# =============================================================================
# Helper Functions
# =============================================================================


def _create_window(size: int, window_type: str) -> Optional[np.ndarray]:
    """
    Create a window function of the specified type and size.

    Args:
        size: Window length.
        window_type: Type of window ('none', 'hanning', 'hamming', 'blackman', 'kaiser').

    Returns:
        Window array or None for no windowing.
    """
    if window_type == "none":
        return None
    elif window_type == "hanning":
        return np.hanning(size)
    elif window_type == "hamming":
        return np.hamming(size)
    elif window_type == "blackman":
        return np.blackman(size)
    elif window_type == "kaiser":
        # Beta=8.6 approximates a Hanning window
        return np.kaiser(size, 8.6)
    else:
        return np.hanning(size)


def _generate_test_signal() -> np.ndarray:
    """
    Generate a synthetic test signal for demonstration.

    Creates a signal with components at 100 Hz, 250 Hz, and 500 Hz.

    Returns:
        Numpy array of test samples.
    """
    fft_size = config["fft_size"]
    sample_rate = config["sample_rate_hz"]
    t = np.arange(fft_size) / sample_rate

    # Multi-tone signal: 100 Hz, 250 Hz, 500 Hz with different amplitudes
    signal = (
        1.0 * np.sin(2 * np.pi * 100 * t)
        + 0.5 * np.sin(2 * np.pi * 250 * t)
        + 0.25 * np.sin(2 * np.pi * 500 * t)
    )

    # Add some noise
    noise = np.random.normal(0, 0.1, fft_size)
    signal = signal + noise

    return signal


# =============================================================================
# Module Result (for PyO3Engine return value)
# =============================================================================

# This variable can be set to return a result from the script execution
result = module_type_info()
