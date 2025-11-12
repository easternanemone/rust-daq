# Recommended Rust Libraries for DAQ Architecture V4

This document summarizes the findings of a research pass to identify mature Rust libraries that can simplify the codebase, improve robustness, and accelerate the V4 refactoring effort.

## 1. Actor Framework

The V4 architecture should be built on a structured actor framework to manage instrument state and concurrency, eliminating the `Arc<Mutex>` patterns that have caused issues.

*   **Primary Recommendation:** **`actix`** ([https://github.com/actix/actix](https://github.com/actix/actix))
    *   **Justification:** As the most mature, popular, and battle-tested actor framework in Rust, `actix` offers stability and a wealth of community resources. It provides a robust foundation for building resilient, concurrent applications, which is critical for this project's long-term health.

*   **Alternative:** **`kameo`** ([https://github.com/jprochazk/kameo](https://github.com/jprochazk/kameo))
    *   **Justification:** A modern, lightweight, and `tokio`-native actor framework. Its design may be simpler to adopt than `actix` if the team prefers a less-verbose API.

**Decision:** The team should evaluate both and standardize on one for the V4 architecture. `actix` is the safer, more established choice.

## 2. Data Handling and Storage

The current mix of custom data types and CSV output is inefficient for large scientific datasets.

*   **Recommendation:** **`apache/arrow-rs`** ([https://github.com/apache/arrow-rs](https://github.com/apache/arrow-rs))
    *   **Justification:** `arrow-rs` is the official Rust implementation of Apache Arrow, the industry standard for high-performance, in-memory columnar data.
    *   **Benefits:**
        *   **Performance:** Zero-copy reads and SIMD-optimized operations provide massive performance gains.
        *   **Interoperability:** Seamlessly share data with Python (Pandas, NumPy) and other data science tools using the Feather or Parquet formats.
        *   **Ecosystem:** Comes with the `parquet` crate for highly efficient, compressed, columnar storage, which should replace CSV for all large datasets.

## 3. GUI and Plotting

Real-time data visualization is a core requirement. The plotting library should integrate natively with the existing `egui` framework.

*   **Recommendation:** **`egui_plot`** ([https://github.com/emilk/egui_plot](https://github.com/emilk/egui_plot))
    *   **Justification:** Developed by the author of `egui`, this library is designed from the ground up for `egui`. It follows the same immediate-mode principles, making it simple to integrate for displaying real-time data streams with minimal boilerplate.

## 4. Instrument Control

Manually implementing protocols for every instrument is error-prone. Using standard libraries for VISA and SCPI is more robust.

*   **Primary Recommendation:** **`visa-rs`** ([https://github.com/TsuITOAR/visa-rs](https://github.com/TsuITOAR/visa-rs))
    *   **Justification:** This library provides high-level, safe Rust bindings for the NI-VISA standard. Any instrument that supports VISA should be controlled through this library, abstracting away the complexities of the underlying communication (Serial, USB, GPIB, Ethernet). This is the most robust path for broad instrument compatibility.

*   **Secondary Recommendation:** **`easy-scpi`** ([https://github.com/bicarlsen/easy-scpi](https://github.com/bicarlsen/easy-scpi))
    *   **Justification:** For simple instruments that only speak SCPI over a raw TCP or serial stream (and do not require a full VISA stack), this library can help parse SCPI responses and build commands.

## 5. Numerical & Signal Processing

Scientific applications often require specialized numerical algorithms like FFT.

*   **Recommendations:**
    1.  **`ndarray`** ([https://github.com/rust-ndarray/ndarray](https://github.com/rust-ndarray/ndarray)): Continue using `ndarray` as the fundamental building block for N-dimensional arrays. It is the standard in the Rust scientific computing ecosystem.
    2.  **`rustfft`** ([https://github.com/ejmahler/rustfft](https://github.com/ejmahler/rustfft)): For any Fast Fourier Transform operations, `rustfft` is the go-to, high-performance library.
    *   **Justification:** The scientific Rust ecosystem is composed of a set of focused, high-quality libraries rather than a single monolithic framework. Combining `ndarray` with specialized crates like `rustfft` is the standard and recommended approach.
