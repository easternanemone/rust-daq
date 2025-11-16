# GUI Documentation

This document provides an overview of the graphical user interface (GUI) for the Rust DAQ application. The GUI is built using `egui` and will leverage `egui_plot` for all real-time data visualization in the V4 architecture.

## Event Log Panel

The Event Log panel is located at the bottom of the main window and displays log messages from the application. It provides several features to help with debugging and monitoring.

### Features

- **Log Display**: Shows a time-stamped, color-coded, and scrollable list of log entries.
- **Level Filtering**: A dropdown allows you to filter logs by their severity (Error, Warn, Info, Debug, Trace).
- **Text Filtering**: A text input field allows you to filter logs by their message content or target.
- **Auto-Scrolling**: A toggle to automatically scroll to the latest log message.
- **Clear Button**: A button to clear all captured log messages.
- **Consolidate Logs**: A toggle to group identical log messages and display an occurrence count. This is useful for reducing noise from repetitive errors.

### Enabling Debug Logging

To enable debug logging, set the `RUST_LOG` environment variable to `debug` before launching the application. For example:

```bash
RUST_LOG=debug cargo run
```

### Log Filtering Syntax

The text filter supports simple substring matching. For more advanced filtering, you can use regular expressions. For example, to find all logs from the `maitai` instrument, you can enter `maitai` in the filter box.

## Plotting Capabilities (V4)

The V4 GUI will utilize `egui_plot` for all real-time data visualization. This includes:

- **Real-time Traces**: Displaying live data streams from instruments.
- **Multi-axis Plots**: Supporting multiple Y-axes for different units or scales.
- **Interactive Features**: Zooming, panning, and cursor readouts.
- **Customizable Plots**: Allowing users to configure plot appearance and data sources.

Data for plotting will be received from the `InstrumentManager` as `apache/arrow-rs` compatible measurements.
