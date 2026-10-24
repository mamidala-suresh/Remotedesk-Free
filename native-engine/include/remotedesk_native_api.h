// RemoteDesk Free - Native C++ Remote Desktop Engine
// High Performance DXGI Desktop Duplication & Hardware GPU H.264 Engine
// Copyright (C) 2026 Suresh Mamidala | Freeware

#pragma once

#include <stdint.h>
#include <stdbool.h>

#ifdef _WIN32
  #ifdef REMOTEDESK_NATIVE_EXPORTS
    #define RD_API __declspec(dllexport)
  #else
    #define RD_API __declspec(dllimport)
  #endif
#else
  #define RD_API
#endif

#ifdef __cplusplus
extern "C" {
#endif

// Engine handle type
typedef void* RdEngineHandle;
typedef void* RdDecoderHandle;

// Monitor descriptor
typedef struct {
    uint32_t index;
    uint32_t width;
    uint32_t height;
    int32_t  left;
    int32_t  top;
    bool     is_primary;
    char     device_name[64];
} RdMonitorInfo;

// Mouse button flags
typedef enum {
    RD_MOUSE_LEFT = 1,
    RD_MOUSE_RIGHT = 2,
    RD_MOUSE_MIDDLE = 3,
    RD_MOUSE_X1 = 4,
    RD_MOUSE_X2 = 5
} RdMouseButton;

// Frame encoded callback: invokes C# or native receiver with H.264 NAL stream
typedef void (*RdEncodedFrameCallback)(
    const uint8_t* data,
    uint32_t size,
    uint32_t width,
    uint32_t height,
    bool is_keyframe,
    uint64_t timestamp_ms,
    void* user_data
);

// Decoded frame ready callback for swapchain presentation
typedef void (*RdDecodedFrameCallback)(
    void* d3d11_texture_handle,
    uint32_t width,
    uint32_t height,
    void* user_data
);

// ==========================================
// 1. Capture & Encoder API (Host Computer)
// ==========================================

// Enumerate connected physical & virtual displays
RD_API int32_t rd_get_monitors(RdMonitorInfo* monitors, int32_t max_count);

// Initialize DXGI Duplication & Hardware H.264 Encoder (NVENC, AMF, QuickSync, MFT)
RD_API RdEngineHandle rd_engine_create(
    uint32_t monitor_index,
    uint32_t target_fps,
    uint32_t target_bitrate_kbps,
    uint32_t scale_percent,
    RdEncodedFrameCallback callback,
    void* user_data
);

// Start capture worker thread
RD_API bool rd_engine_start(RdEngineHandle engine);

// Stop capture worker thread
RD_API bool rd_engine_stop(RdEngineHandle engine);

// Force instantaneous IDR Keyframe generation (e.g. upon minimize/restore or new client)
RD_API void rd_engine_request_keyframe(RdEngineHandle engine);

// Dynamically tune stream bitrate and framerate
RD_API void rd_engine_set_quality(RdEngineHandle engine, uint32_t bitrate_kbps, uint32_t target_fps, uint32_t scale_percent);

// Destroy capture engine and release COM/DirectX resources
RD_API void rd_engine_destroy(RdEngineHandle engine);

// ==========================================
// 2. Hardware Decoder & Renderer (Viewer)
// ==========================================

// Initialize Hardware GPU H.264 Video Decoder (DXVA2 / Direct3D 11)
RD_API RdDecoderHandle rd_decoder_create(
    RdDecodedFrameCallback callback,
    void* user_data
);

// Decode raw H.264 NAL packet directly to GPU surface
RD_API bool rd_decoder_decode(
    RdDecoderHandle decoder,
    const uint8_t* h264_data,
    uint32_t size,
    uint64_t timestamp_ms
);

// Obtain DirectX shared handle for zero-copy WPF D3DImage / WinUI SwapChainPanel rendering
RD_API void* rd_decoder_get_shared_handle(RdDecoderHandle decoder);

// Destroy hardware video decoder
RD_API void rd_decoder_destroy(RdDecoderHandle decoder);

// ==========================================
// 3. Windows Input Injection (Host Computer)
// ==========================================

RD_API bool rd_input_mouse_move(int32_t x, int32_t y, uint32_t screen_w, uint32_t screen_h);
RD_API bool rd_input_mouse_down(RdMouseButton button, int32_t x, int32_t y, uint32_t screen_w, uint32_t screen_h);
RD_API bool rd_input_mouse_up(RdMouseButton button, int32_t x, int32_t y, uint32_t screen_w, uint32_t screen_h);
RD_API bool rd_input_mouse_wheel(int32_t delta_x, int32_t delta_y);
RD_API bool rd_input_key_down(uint32_t vk_code, uint32_t scan_code, bool extended);
RD_API bool rd_input_key_up(uint32_t vk_code, uint32_t scan_code, bool extended);
RD_API bool rd_input_text_utf16(const wchar_t* text);

#ifdef __cplusplus
}
#endif
