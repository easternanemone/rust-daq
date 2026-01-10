# PVCAM Circular Buffer Implementation Audit

**Date:** 2026-01-09
**Camera:** Prime BSI (GS2020 sensor), 2048x2048 pixels
**PVCAM Version:** 3.10.2.5 (maitai)
**SDK Version:** 3.10.2.5-1

## Executive Summary

Analysis of `/opt/pvcam/sdk/` on maitai reveals **three critical issues** in the Rust implementation that likely cause both the **Error 185 (Invalid Configuration)** with CIRC_OVERWRITE and the **85-frame stall** with CIRC_NO_OVERWRITE.

**Root Cause:** The `exp_mode` parameter passed to `pl_exp_setup_cont()` is constructed incorrectly. Modern sCMOS cameras like Prime BSI do NOT support bare `TIMED_MODE` - they require `EXT_TRIG_INTERNAL` OR'd with an expose-out mode.

---

## Critical Issue #1: exp_mode Construction (ROOT CAUSE)

### Location
`src/components/acquisition.rs` lines 1171-1173

### Current Code
```rust
// bd-3gnv: Always use TIMED_MODE (like DynExp). EXT_TRIG_INTERNAL caused error 185.
// DynExp reference: uses TIMED_MODE with CIRC_OVERWRITE successfully.
let exp_mode = TIMED_MODE;
```

### SDK Pattern (Common.cpp lines 1239-1290)
```cpp
bool SelectCameraExpMode(const CameraContext* ctx, int16& expMode,
        int16 legacyTrigMode, int16 extendedTrigMode)
{
    NVPC triggerModes;
    ReadEnumeration(ctx->hcam, &triggerModes, PARAM_EXPOSURE_MODE, ...);

    // Try to find the legacy mode first (for CCD cameras)
    for (const NVP& nvp : triggerModes) {
        if (nvp.value == legacyTrigMode) {
            expMode = legacyTrigMode;  // CCD: use bare TIMED_MODE
            return true;
        }
    }

    // Modern sCMOS: use extended mode with expose-out
    for (const NVP& nvp : triggerModes) {
        if (nvp.value == extendedTrigMode) {
            NVPC expOutModes;
            ReadEnumeration(ctx->hcam, &expOutModes, PARAM_EXPOSE_OUT_MODE, ...);
            const int16 expOutMode = static_cast<int16>(expOutModes[0].value);

            // CRITICAL: The final mode is an 'or-ed' value!
            expMode = extendedTrigMode | expOutMode;
            return true;
        }
    }
}
```

### Why This Causes Error 185

From `pvcam.h` lines 602-606:
```cpp
/** This mode is similar to the legacy #TIMED_MODE.
    This value allows the exposure mode to be "ORed" with #PL_EXPOSE_OUT_MODES */
EXT_TRIG_INTERNAL = (7 + 0) << 8,  // = 1792
```

Prime BSI is a **modern sCMOS camera** that does NOT advertise `TIMED_MODE` in its `PARAM_EXPOSURE_MODE` enumeration. When the SDK receives `TIMED_MODE (0)`, it returns error 185 because that mode literally isn't supported by the hardware.

The SDK examples call `SelectCameraExpMode(ctx, expMode, TIMED_MODE, EXT_TRIG_INTERNAL)`, which:
1. First checks if `TIMED_MODE` is in the enumeration (it won't be for Prime BSI)
2. Falls back to `EXT_TRIG_INTERNAL`
3. OR's it with the first available expose-out mode

### Fix

```rust
// Query PARAM_EXPOSURE_MODE to find supported modes
fn select_camera_exp_mode(hcam: i16) -> Result<i16> {
    // Check if camera supports PARAM_EXPOSE_OUT_MODE (modern sCMOS)
    let has_expose_out = unsafe {
        let mut avail: rs_bool = 0;
        pl_get_param(hcam, PARAM_EXPOSE_OUT_MODE, ATTR_AVAIL as i16,
                     &mut avail as *mut _ as *mut _) != 0 && avail != 0
    };

    if has_expose_out {
        // Modern sCMOS: Use EXT_TRIG_INTERNAL | expose_out_mode
        // Query first available expose-out mode
        let expose_out_mode = unsafe {
            let mut mode: i32 = 0;
            if pl_get_param(hcam, PARAM_EXPOSE_OUT_MODE, ATTR_CURRENT as i16,
                           &mut mode as *mut _ as *mut _) != 0 {
                mode as i16
            } else {
                EXPOSE_OUT_FIRST_ROW as i16  // Default fallback
            }
        };
        Ok(EXT_TRIG_INTERNAL | expose_out_mode)
    } else {
        // Legacy CCD: Use bare TIMED_MODE
        Ok(TIMED_MODE)
    }
}

// In start_stream, replace line 1173:
let exp_mode = select_camera_exp_mode(h)?;
```

### Quick Test
Before full implementation, try this one-line change at line 1173:
```rust
let exp_mode = EXT_TRIG_INTERNAL;  // 1792 instead of TIMED_MODE (0)
```

If CIRC_OVERWRITE works after this change, the root cause is confirmed.

---

## Critical Issue #2: Callback Registration Order

### Location
`src/components/acquisition.rs` lines 1176-1254

### Current Order (WRONG)
```
1. pl_exp_setup_cont()     (lines 1177-1195)
2. pl_cam_register_callback_ex3()  (lines 1237-1254)
3. pl_exp_start_cont()     (lines 1288-1305)
```

### SDK Order (LiveImage.cpp)
```
1. pl_cam_register_callback_ex3()  (line 119)
2. pl_exp_setup_cont()             (line 146)
3. pl_exp_start_cont()             (line 173)
```

### Why This Matters
The SDK documentation states: "A callback can only be registered once, after opening the camera."

However, the LiveImage.cpp example registers the callback BEFORE calling `pl_exp_setup_cont()`. This ensures the callback is ready to receive frames immediately when acquisition starts.

### Fix
Move callback registration BEFORE `pl_exp_setup_cont()`:

```rust
// In start_stream(), before pl_exp_setup_cont:

// PVCAM Best Practices: Register callback BEFORE setup
let callback_ctx_ptr = &**self.callback_context as *const CallbackContext;
let use_callback = unsafe {
    let result = pl_cam_register_callback_ex3(
        h,
        PL_CALLBACK_EOF,
        pvcam_eof_callback as *mut std::ffi::c_void,
        callback_ctx_ptr as *mut std::ffi::c_void,
    );
    if result == 0 {
        tracing::warn!("Failed to register EOF callback");
        false
    } else {
        self.callback_registered.store(true, Ordering::Release);
        true
    }
};

// THEN call pl_exp_setup_cont
let mut frame_bytes: uns32 = 0;
if pl_exp_setup_cont(...) == 0 {
    // Deregister callback on failure
    if use_callback {
        pl_cam_deregister_callback(h, PL_CALLBACK_EOF);
        self.callback_registered.store(false, Ordering::Release);
    }
    return Err(...);
}
```

---

## Critical Issue #3: Frame Retrieval Location

### Location
`src/components/acquisition.rs` lines 266-285 (callback) and lines 2206-2225 (retrieval)

### Current Architecture
```
Callback → signals condvar → Frame loop wakes → calls get_oldest_frame
```

### SDK Architecture (LiveImage.cpp CustomEofHandler lines 47-57)
```cpp
void PV_DECL CustomEofHandler(FRAME_INFO* pFrameInfo, void* pContext)
{
    auto ctx = static_cast<CameraContext*>(pContext);
    ctx->eofCounter++;
    ctx->eofFrameInfo = *pFrameInfo;

    // Get frame pointer HERE in callback:
    if (PV_OK != pl_exp_get_latest_frame(ctx->hcam, &ctx->eofFrame))
    {
        ctx->eofFrame = nullptr;
    }

    // Then signal:
    ctx->eofEvent.cond.notify_all();
}
```

### SDK Best Practices Documentation
> "When EOF callback notification is executed by PVCAM, call `pl_exp_get_latest_frame_ex()` from inside the callback routine to retrieve the frame data pointer."

### Why This Matters
1. **Timing**: The frame pointer retrieved by `get_latest_frame` is only guaranteed valid at callback time
2. **Multiple callbacks**: If callbacks fire faster than the loop processes, frames may be missed
3. **SDK contract**: The documentation explicitly states to retrieve in the callback

### Fix
Modify `CallbackContext` to store the camera handle and frame pointer:

```rust
#[cfg(feature = "pvcam_hardware")]
pub struct CallbackContext {
    pub pending_frames: std::sync::atomic::AtomicU32,
    pub latest_frame_nr: AtomicI32,
    pub condvar: std::sync::Condvar,
    pub mutex: std::sync::Mutex<bool>,
    pub shutdown: AtomicBool,

    // NEW: Store camera handle for frame retrieval in callback
    pub hcam: AtomicI16,
    // NEW: Store latest frame pointer (set in callback)
    pub latest_frame_ptr: std::sync::atomic::AtomicPtr<std::ffi::c_void>,
    // NEW: Store latest frame info
    pub latest_frame_info: std::sync::Mutex<FRAME_INFO>,
}

#[cfg(feature = "pvcam_hardware")]
pub unsafe extern "system" fn pvcam_eof_callback(
    p_frame_info: *const FRAME_INFO,
    p_context: *mut std::ffi::c_void,
) {
    if p_context.is_null() {
        return;
    }
    let ctx = &*(p_context as *const CallbackContext);

    // Store frame info
    let frame_nr = if !p_frame_info.is_null() {
        if let Ok(mut info) = ctx.latest_frame_info.lock() {
            *info = *p_frame_info;
        }
        (*p_frame_info).FrameNr
    } else {
        -1
    };

    // CRITICAL: Retrieve frame pointer HERE in callback
    let hcam = ctx.hcam.load(Ordering::Acquire);
    if hcam >= 0 {
        let mut frame_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        if pl_exp_get_latest_frame(hcam, &mut frame_ptr) != 0 && !frame_ptr.is_null() {
            ctx.latest_frame_ptr.store(frame_ptr, Ordering::Release);
        }
    }

    ctx.signal_frame_ready(frame_nr);
}
```

---

## Issue #4: Buffer Mode Mismatch

### Current State
- `USE_CIRC_OVERWRITE_MODE = false` (forced due to error 185)
- Uses `get_oldest_frame` + `unlock_oldest_frame` (correct for CIRC_NO_OVERWRITE)

### SDK Documentation (pvcam.h lines 4306-4332)
- `CIRC_OVERWRITE`: Use `pl_exp_get_latest_frame` - no unlock needed
- `CIRC_NO_OVERWRITE`: Use `pl_exp_get_oldest_frame` + `pl_exp_unlock_oldest_frame`

### Current Code Assessment
The mode switching logic at lines 2206-2225 is correct. However, it will only work properly once Issue #1 is fixed and CIRC_OVERWRITE stops returning error 185.

---

## Issue #5: 85-Frame Stall Root Cause Analysis

### Observation
Camera stops producing frames after exactly 85 frames, regardless of buffer size.

### Likely Cause
With incorrect `exp_mode` (TIMED_MODE instead of EXT_TRIG_INTERNAL), the camera may accept the configuration but operate in a degraded mode where:
1. Internal buffers fill up after ~85 frames
2. The unlock mechanism doesn't properly release buffers
3. Camera stops producing new frames

### Evidence
- The stall occurs at a consistent frame count (85) regardless of buffer size
- Auto-restart temporarily works but eventually fails
- The issue doesn't occur in SDK examples using proper `SelectCameraExpMode()`

### Prediction
Fixing Issue #1 (exp_mode) should resolve the 85-frame stall because:
1. CIRC_OVERWRITE mode will work (the preferred mode)
2. Even if using CIRC_NO_OVERWRITE, the camera will properly cycle buffers

---

## Priority Matrix

| Priority | Issue | Impact | Effort | Fix Location |
|----------|-------|--------|--------|--------------|
| **P0** | #1: exp_mode construction | Blocks CIRC_OVERWRITE, likely causes 85-frame stall | Medium | Lines 1171-1173 |
| **P1** | #2: Callback order | May cause missed frames at startup | Low | Lines 1176-1254 |
| **P2** | #3: Frame retrieval in callback | Potential timing issues, SDK non-compliance | Medium | Lines 266-285, 2206-2225 |
| **P3** | #4: Verify mode after fix | Ensure CIRC_OVERWRITE works post-fix | Low | Line 92 |

---

## Recommended Fix Sequence

### Step 1: Quick Test (5 minutes)
Change line 1173 to:
```rust
let exp_mode = EXT_TRIG_INTERNAL;  // or: let exp_mode = 1792i16;
```
Set `USE_CIRC_OVERWRITE_MODE = true` at line 92.
Run test to see if error 185 is resolved.

### Step 2: Full exp_mode Fix (30 minutes)
Implement `select_camera_exp_mode()` function that queries `PARAM_EXPOSURE_MODE` and `PARAM_EXPOSE_OUT_MODE`.

### Step 3: Callback Order Fix (15 minutes)
Move callback registration before `pl_exp_setup_cont()`.

### Step 4: Frame Retrieval Refactor (1-2 hours)
Modify callback to retrieve frame pointer directly, matching SDK pattern.

### Step 5: Validation (30 minutes)
Run >10,000 frames with CIRC_OVERWRITE at various frame rates.
Verify no lost frames via FRAME_INFO.FrameNr tracking.

---

## Success Criteria

1. CIRC_OVERWRITE mode works without error 185
2. Continuous acquisition runs >10,000 frames without stalling
3. Frame rate matches camera specification (~100fps @ full resolution)
4. No lost frames (FRAME_INFO.FrameNr gaps = 0)

---

## References

- `/opt/pvcam/sdk/include/pvcam.h` - API definitions
- `/opt/pvcam/sdk/examples/code_samples/src/LiveImage/LiveImage.cpp` - Canonical example
- `/opt/pvcam/sdk/examples/code_samples/src/CommonFiles/Common.cpp` - SelectCameraExpMode()
- `/opt/pvcam/sdk/doc/PVCAM User Manual/_best_practices.xhtml` - Best practices
