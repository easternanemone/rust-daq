#include "Common.h"
#include <iostream>
#include <vector>
#include <chrono>

// Callback handler - kept minimal
void PV_DECL StallTestEofHandler(FRAME_INFO* pFrameInfo, void* pContext)
{
    if (!pFrameInfo || !pContext)
        return;
    auto ctx = static_cast<CameraContext*>(pContext);

    ctx->eofCounter++;
    ctx->eofFrameInfo = *pFrameInfo;

    // Get latest frame to clear buffer (crucial)
    if (PV_OK != pl_exp_get_latest_frame(ctx->hcam, &ctx->eofFrame))
    {
        // Ignore error in callback for now, just signal
    }

    // Unblock the main thread
    {
        std::lock_guard<std::mutex> lock(ctx->eofEvent.mutex);
        ctx->eofEvent.flag = true;
    }
    ctx->eofEvent.cond.notify_all();
}

int main(int argc, char* argv[])
{
    std::vector<CameraContext*> contexts;
    // Open the first available camera
    if (!InitAndOpenOneCamera(contexts, cSingleCamIndex))
    {
        std::cerr << "Failed to open camera." << std::endl;
        return 1;
    }

    CameraContext* ctx = contexts[cSingleCamIndex];
    
    // Register callback
    if (PV_OK != pl_cam_register_callback_ex3(ctx->hcam, PL_CALLBACK_EOF,
                (void*)StallTestEofHandler, ctx))
    {
        PrintErrorMessage(pl_error_code(), "pl_cam_register_callback() error");
        CloseAllCamerasAndUninit(contexts);
        return 1;
    }

    uns32 exposureBytes;
    const uns32 exposureTime = 10; // 10ms exposure
    
    // Use a large circular buffer (e.g., 20 frames)
    const uns16 circBufferFrames = 20;
    const int16 bufferMode = CIRC_OVERWRITE;
    
    int16 expMode;
    // Try to select internal trigger mode
    if (!SelectCameraExpMode(ctx, expMode, TIMED_MODE, EXT_TRIG_INTERNAL))
    {
        CloseAllCamerasAndUninit(contexts);
        return 1;
    }

    // Setup continuous acquisition
    if (PV_OK != pl_exp_setup_cont(ctx->hcam, 1, &ctx->region, expMode,
                exposureTime, &exposureBytes, bufferMode))
    {
        PrintErrorMessage(pl_error_code(), "pl_exp_setup_cont() error");
        CloseAllCamerasAndUninit(contexts);
        return 1;
    }
    
    UpdateCtxImageFormat(ctx);

    const uns32 circBufferBytes = circBufferFrames * exposureBytes;
    uns8* circBufferInMemory = new (std::nothrow) uns8[circBufferBytes];
    if (!circBufferInMemory)
    {
        std::cerr << "Allocation failed." << std::endl;
        CloseAllCamerasAndUninit(contexts);
        return 1;
    }

    std::cout << "Starting acquisition for 200 frames..." << std::endl;

    if (PV_OK != pl_exp_start_cont(ctx->hcam, circBufferInMemory, circBufferBytes))
    {
        PrintErrorMessage(pl_error_code(), "pl_exp_start_cont() error");
        delete [] circBufferInMemory;
        CloseAllCamerasAndUninit(contexts);
        return 1;
    }

    bool errorOccurred = false;
    uns32 framesAcquired = 0;
    auto lastFrameTime = std::chrono::steady_clock::now();

    while (framesAcquired < 200)
    {
        // Wait up to 2 seconds for a frame
        if (!WaitForEofEvent(ctx, 2000, errorOccurred))
        {
            std::cerr << "TIMEOUT waiting for frame " << framesAcquired + 1 << "!" << std::endl;
            std::cerr << "Potential 85-frame stall detected at frame " << framesAcquired << std::endl;
            break;
        }

        auto now = std::chrono::steady_clock::now();
        auto diff = std::chrono::duration_cast<std::chrono::milliseconds>(now - lastFrameTime).count();
        lastFrameTime = now;

        std::cout << "Frame #" << ctx->eofFrameInfo.FrameNr 
                  << " acquired. Delta: " << diff << "ms" << std::endl;

        framesAcquired++;
    }

    pl_exp_abort(ctx->hcam, CCS_HALT);
    delete [] circBufferInMemory;
    CloseAllCamerasAndUninit(contexts);

    if (framesAcquired == 200) {
        std::cout << "SUCCESS: Acquired 200 frames without stalling." << std::endl;
    } else {
        std::cout << "FAILURE: Stopped at frame " << framesAcquired << std::endl;
    }

    return 0;
}
