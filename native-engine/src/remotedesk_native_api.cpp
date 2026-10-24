// RemoteDesk Free - Native C-ABI Engine Entrypoint
// Copyright (C) 2026 Suresh Mamidala | Freeware

#define REMOTEDESK_NATIVE_EXPORTS
#include "remotedesk_native_api.h"
#include "dxgi_duplicator.h"
#include "gpu_h264_encoder.h"
#include "gpu_h264_decoder.h"

#include <thread>
#include <atomic>
#include <chrono>
#include <iostream>

struct RdEngineContext {
    std::unique_ptr<DxgiDuplicator> duplicator;
    std::unique_ptr<GpuH264Encoder> encoder;

    uint32_t monitorIndex;
    uint32_t targetFps;
    uint32_t bitrateKbps;
    uint32_t scalePercent;

    RdEncodedFrameCallback callback;
    void* userData;

    std::thread captureThread;
    std::atomic<bool> isRunning{false};
    std::atomic<bool> forceKeyframe{true};
};

extern "C" {

int32_t rd_get_monitors(RdMonitorInfo* monitors, int32_t max_count) {
    if (!monitors || max_count <= 0) return 0;
    auto list = DxgiDuplicator::EnumerateMonitors();
    int32_t toCopy = (int32_t)list.size() < max_count ? (int32_t)list.size() : max_count;
    for (int32_t i = 0; i < toCopy; ++i) {
        monitors[i] = list[i];
    }
    return toCopy;
}

RdEngineHandle rd_engine_create(
    uint32_t monitor_index,
    uint32_t target_fps,
    uint32_t target_bitrate_kbps,
    uint32_t scale_percent,
    RdEncodedFrameCallback callback,
    void* user_data
) {
    auto ctx = new RdEngineContext();
    ctx->monitorIndex = monitor_index;
    ctx->targetFps = target_fps > 0 ? target_fps : 60; // 60 FPS default for native engine
    ctx->bitrateKbps = target_bitrate_kbps > 0 ? target_bitrate_kbps : 3500;
    ctx->scalePercent = scale_percent > 0 ? scale_percent : 100;
    ctx->callback = callback;
    ctx->userData = user_data;

    ctx->duplicator = std::make_unique<DxgiDuplicator>();
    ctx->encoder = std::make_unique<GpuH264Encoder>();

    return (RdEngineHandle)ctx;
}

static void CaptureWorkerLoop(RdEngineContext* ctx) {
    if (!ctx->duplicator->Initialize(ctx->monitorIndex)) {
        std::cerr << "[Engine] Failed to initialize DXGI Duplicator on monitor " << ctx->monitorIndex << std::endl;
        ctx->isRunning = false;
        return;
    }

    uint32_t width = ctx->duplicator->GetWidth();
    uint32_t height = ctx->duplicator->GetHeight();

    auto onEncoded = [ctx](const uint8_t* data, uint32_t size, uint32_t w, uint32_t h, bool isKeyframe, uint64_t ts) {
        if (ctx->callback) {
            ctx->callback(data, size, w, h, isKeyframe, ts, ctx->userData);
        }
    };

    if (!ctx->encoder->Initialize(
        ctx->duplicator->GetDevice(),
        width,
        height,
        ctx->targetFps,
        ctx->bitrateKbps,
        ctx->scalePercent,
        onEncoded
    )) {
        std::cerr << "[Engine] Failed to initialize GPU H.264 Encoder." << std::endl;
        ctx->isRunning = false;
        return;
    }

    std::cout << "[Engine] DXGI + GPU H.264 streaming engine active at " << ctx->targetFps << " FPS (" << width << "x" << height << ")" << std::endl;

    auto frameDuration = std::chrono::milliseconds(1000 / ctx->targetFps);

    while (ctx->isRunning.load()) {
        auto start = std::chrono::steady_clock::now();

        if (ctx->forceKeyframe.exchange(false)) {
            ctx->encoder->ForceKeyframe();
        }

        ID3D11Texture2D* frameTexture = nullptr;
        bool cursorUpdated = false;
        POINT cursorPos = {};
        bool cursorVisible = false;

        // Acquire with ultra-low latency timeout (5ms)
        if (ctx->duplicator->AcquireFrame(&frameTexture, 5, cursorUpdated, cursorPos, cursorVisible)) {
            uint64_t nowMs = std::chrono::duration_cast<std::chrono::milliseconds>(
                std::chrono::system_clock::now().time_since_epoch()
            ).count();

            ctx->encoder->EncodeFrame(frameTexture, nowMs);
            frameTexture->Release();
        }

        auto elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::steady_clock::now() - start);
        if (elapsed < frameDuration) {
            std::this_thread::sleep_for(frameDuration - elapsed);
        }
    }

    ctx->encoder->Release();
    ctx->duplicator->Release();
    std::cout << "[Engine] Screen capture worker stopped." << std::endl;
}

bool rd_engine_start(RdEngineHandle engine) {
    if (!engine) return false;
    auto ctx = (RdEngineContext*)engine;

    if (ctx->isRunning.exchange(true)) {
        return true; // Already running
    }

    ctx->captureThread = std::thread(CaptureWorkerLoop, ctx);
    return true;
}

bool rd_engine_stop(RdEngineHandle engine) {
    if (!engine) return false;
    auto ctx = (RdEngineContext*)engine;

    if (ctx->isRunning.exchange(false)) {
        if (ctx->captureThread.joinable()) {
            ctx->captureThread.join();
        }
    }
    return true;
}

void rd_engine_request_keyframe(RdEngineHandle engine) {
    if (!engine) return;
    auto ctx = (RdEngineContext*)engine;
    ctx->forceKeyframe.store(true);
}

void rd_engine_set_quality(RdEngineHandle engine, uint32_t bitrate_kbps, uint32_t target_fps, uint32_t scale_percent) {
    if (!engine) return;
    auto ctx = (RdEngineContext*)engine;
    ctx->bitrateKbps = bitrate_kbps;
    if (target_fps > 0) ctx->targetFps = target_fps;
    if (scale_percent > 0) ctx->scalePercent = scale_percent;

    if (ctx->encoder) {
        ctx->encoder->SetBitrateAndFps(bitrate_kbps, target_fps);
    }
}

void rd_engine_destroy(RdEngineHandle engine) {
    if (!engine) return;
    auto ctx = (RdEngineContext*)engine;
    rd_engine_stop(engine);
    delete ctx;
}

// Decoder API
RdDecoderHandle rd_decoder_create(RdDecodedFrameCallback callback, void* user_data) {
    auto dec = new GpuH264Decoder(callback, user_data);
    if (!dec->Initialize()) {
        delete dec;
        return nullptr;
    }
    return (RdDecoderHandle)dec;
}

bool rd_decoder_decode(RdDecoderHandle decoder, const uint8_t* h264_data, uint32_t size, uint64_t timestamp_ms) {
    if (!decoder) return false;
    auto dec = (GpuH264Decoder*)decoder;
    return dec->DecodeFrame(h264_data, size, timestamp_ms);
}

void* rd_decoder_get_shared_handle(RdDecoderHandle decoder) {
    if (!decoder) return nullptr;
    auto dec = (GpuH264Decoder*)decoder;
    return dec->GetSharedSurfaceHandle();
}

void rd_decoder_destroy(RdDecoderHandle decoder) {
    if (!decoder) return;
    auto dec = (GpuH264Decoder*)decoder;
    delete dec;
}

} // extern "C"
