//! Low-level FFI bindings for the Linux Comedi library.
//!
//! This crate provides raw, unsafe bindings to the comedilib C library,
//! which is the user-space interface to Comedi (Control and Measurement
//! Device Interface) kernel drivers.
//!
//! # Comedi Overview
//!
//! Comedi is a collection of Linux kernel drivers for data acquisition (DAQ)
//! hardware. It supports a wide variety of devices from manufacturers like
//! National Instruments, Measurement Computing, Advantech, and others.
//!
//! # Safety
//!
//! All functions in this crate are `unsafe` as they are direct FFI bindings.
//! For a safe wrapper, use the `daq-driver-comedi` crate instead.
//!
//! # Features
//!
//! - `comedi-sdk`: Generate bindings from system comedilib headers.
//!   Without this feature, pre-defined bindings are used for cross-compilation.
//!
//! # Example (unsafe)
//!
//! ```no_run
//! use comedi_sys::*;
//! use std::ffi::CString;
//! use std::ptr;
//!
//! unsafe {
//!     let path = CString::new("/dev/comedi0").unwrap();
//!     let dev = comedi_open(path.as_ptr());
//!     if !dev.is_null() {
//!         let n_subdevices = comedi_get_n_subdevices(dev);
//!         println!("Device has {} subdevices", n_subdevices);
//!         comedi_close(dev);
//!     }
//! }
//! ```

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(clippy::all)]

// Include the generated bindings
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        // Verify subdevice type constants are defined
        assert_eq!(COMEDI_SUBD_AI, 1);
        assert_eq!(COMEDI_SUBD_AO, 2);
        assert_eq!(COMEDI_SUBD_DIO, 5);
        assert_eq!(COMEDI_SUBD_COUNTER, 6);
    }

    #[test]
    fn test_aref_constants() {
        // Verify analog reference constants
        assert_eq!(AREF_GROUND, 0);
        assert_eq!(AREF_COMMON, 1);
        assert_eq!(AREF_DIFF, 2);
    }

    #[test]
    fn test_cr_pack() {
        // Test channel/range/aref packing
        let packed = CR_PACK(0, 1, AREF_GROUND);
        assert_eq!(CR_CHAN(packed), 0);
        assert_eq!(CR_RANGE(packed), 1);
        assert_eq!(CR_AREF(packed), AREF_GROUND);
    }

    #[test]
    fn test_dio_constants() {
        assert_eq!(COMEDI_INPUT, 0);
        assert_eq!(COMEDI_OUTPUT, 1);
    }
}
