# PVCAM Setup and Troubleshooting

This guide details the setup and troubleshooting steps for the Photometrics PVCAM driver on Linux, specifically for high-bandwidth cameras like the Prime BSI.

## Prerequisites

- **OS**: Arch Linux (verified), Ubuntu/Debian (supported by PVCAM SDK)
- **SDK**: PVCAM SDK 3.10.2.5 or later
- **Hardware**: Photometrics Camera (e.g., Prime BSI, Prime 95B) via USB 3.0 or PCIe

## Installation

1. **Install PVCAM SDK**: Follow the official instructions to install the SDK.
2. **Kernel Module**: Ensure the `pvcam` kernel module is loaded:

    ```bash
    sudo modprobe pvcam
    lsmod | grep pvcam
    ```

3. **Permissions**: Ensure your user is in the `video` or `users` group (depending on udev rules).

## Troubleshooting

### "Failed to start continuous acquisition"

**Symptoms:**

- Camera is detected.
- Metadata (serial, firmware) can be read.
- `acquire_frame()` fails immediately with an error indicating acquisition start failure.

**Cause:**
This is often caused by insufficient USB memory buffer allocation in the Linux kernel. The default `usbfs_memory_mb` is typically 16MB or 200MB, which is too small for high-bandwidth cameras like the Prime BSI (which requires ~16MB *per frame* and allocates multiple buffers).

**Solution:**
Increase the `usbfs_memory_mb` limit to at least 1000MB.

**Temporary Fix:**

```bash
echo 1000 | sudo tee /sys/module/usbcore/parameters/usbfs_memory_mb
```

**Permanent Fix (Systemd):**
Create a systemd service to apply this setting on boot:

`/etc/systemd/system/pvcam-usb-buffer.service`:

```ini
[Unit]
Description=Set USBFS memory limit for PVCAM
After=network.target

[Service]
Type=oneshot
ExecStart=/bin/sh -c "echo 1000 > /sys/module/usbcore/parameters/usbfs_memory_mb"
RemainAfterExit=yes

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable pvcam-usb-buffer
sudo systemctl start pvcam-usb-buffer
```

### "Failed to open camera"

**Symptoms:**

- Application panics or returns error during `PvcamDriver::new()`.

**Checklist:**

1. **Kernel Module**: Run `lsmod | grep pvcam`. If missing, reinstall drivers or run `sudo depmod -a && sudo modprobe pvcam`.
2. **USB Connection**: Run `lsusb`. Look for Photometrics device (ID `1f12:xxxx`).
3. **Environment Variables**: Ensure `PVCAM_SDK_DIR` etc. are set correctly (see crate README).
4. **Lockfile**: Check for stale lockfiles if a previous process crashed.

### 16-bit Pixel Depth

**Note**: Most scientific cameras (Prime BSI included) return 16-bit data (`u16`).

- **Buffer Size**: When verifying frame data manually, remember `buffer_size_bytes = width * height * 2`.
- **Binning**: Some cameras support flexible binning (3x3, 5x5) even if not officially advertised in simple spec sheets. The driver typically queries the hardware for validity.

## Verification

Run the hardware validation suite:

```bash
cargo test --test hardware_pvcam_validation --features "pvcam_hardware,hardware_tests" -- --test-threads=1
```

(Note: `--test-threads=1` is recommended to avoid resource contention on the USB bus during tests).
