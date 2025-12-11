# Feature Matrix (Draft)

**Status:** Draft — owned by Phase 5 (bd-37tw.5)  
**Purpose:** Single source of truth for build profiles, feature groups, and CI matrix once crate splits land.

## Defaults
- **Default build:** headless + transport + mock hardware + CSV storage
  - Enabled: `transport`, `mock-hw`, `storage-csv`
  - Disabled by default: `gui`, `drivers-serial`, `drivers-pvcam`, `storage-hdf5`, `storage-arrow`, `scripting-python`

## Feature Groups (proposed)
- `transport` — tonic/tonic-web server + proto mappings
- `gui` — egui/eframe desktop GUI
- `drivers-serial` — serialport + tokio-serial drivers (elliptec, esp300, newport 1830C, maitai)
- `drivers-pvcam` — PVCAM SDK + daq-driver-pvcam crate
- `storage-csv` — CSV writer (default)
- `storage-arrow` — Arrow IPC writer
- `storage-hdf5` — HDF5 writer (requires system libhdf5)
- `storage-netcdf` — NetCDF writer
- `scripting-python` — PyO3 backend for ScriptEngine

## Recommended Profiles
- **Headless minimal (fast dev):** `--no-default-features --features transport,storage-csv`
- **Server + scripting:** `--no-default-features --features transport,storage-csv,scripting-python`
- **Full hardware (no GUI):** `--no-default-features --features transport,drivers-serial,drivers-pvcam,storage-arrow`
- **GUI operator build:** `--no-default-features --features transport,gui,drivers-serial,storage-arrow`
- **Data-heavy lab build:** `--no-default-features --features transport,drivers-serial,drivers-pvcam,storage-hdf5,storage-arrow`

## CI Matrix (suggested)
- **fast:** `transport,storage-csv` (unit + doc tests)
- **hardware-lite:** `transport,drivers-serial,storage-csv` (mocked)
- **storage:** `transport,storage-arrow` and `transport,storage-hdf5` (if libhdf5 present)
- **gui smoke (optional):** `transport,gui` headless egui smoke test

## Open Questions
- Do we need a separate `plugins_hot_reload` flag in the new daq-hardware crate?
- Should `mock-hw` stay a feature or always-on for tests?
- Is `transport` required for GUI, or should GUI be able to run against a local engine without gRPC?

## Next Steps
- Align feature definitions in each new crate’s `Cargo.toml`.
- Update README and examples to use these profiles.
- Mirror matrix in CI once crate splits merge. 
