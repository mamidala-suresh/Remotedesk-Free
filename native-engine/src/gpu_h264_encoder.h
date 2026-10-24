// RemoteDesk Free - GPU H.264 Hardware Encoder
// Copyright (C) 2026 Suresh Mamidala | Freeware

#pragma once

#include <windows.h>
#include <strmif.h>
#include <d3d11.h>
#include <mfapi.h>
#include <mfidl.h>
#include <mftransform.h>
#include <codecapi.h>
#include <stdint.h>
#include <vector>
#include <functional>

typedef std::function<void(const uint8_t* data, uint32_t size, uint32_t width, uint32_t height, bool is_keyframe, uint64_t timestamp_ms)> OnFrameEncodedFn;

class GpuH264Encoder {
public:
    GpuH264Encoder();
    ~GpuH264Encoder();

    bool Initialize(
        ID3D11Device* device,
        uint32_t width,
        uint32_t height,
        uint32_t fps,
        uint32_t bitrateKbps,
        uint32_t scalePercent,
        OnFrameEncodedFn callback
    );

    void Release();

    // Encodes a D3D11 frame texture to H.264 NAL units
    bool EncodeFrame(ID3D11Texture2D* texture, uint64_t timestampMs);

    // Forces immediate IDR Keyframe on next encode tick
    void ForceKeyframe();

    // Dynamic bitrate & framerate adjustment
    void SetBitrateAndFps(uint32_t bitrateKbps, uint32_t fps);

private:
    bool SetupEncoderMFT();
    bool CreateMediaTypes();

    ID3D11Device*        m_device = nullptr;
    ID3D11DeviceContext* m_context = nullptr;
    IMFTransform*        m_encoderMft = nullptr;
    ICodecAPI*           m_codecApi = nullptr;

    uint32_t m_srcWidth = 1920;
    uint32_t m_srcHeight = 1080;
    uint32_t m_targetWidth = 1920;
    uint32_t m_targetHeight = 1080;
    uint32_t m_fps = 30;
    uint32_t m_bitrate = 2500000; // 2.5 Mbps
    uint32_t m_scalePercent = 100;
    bool     m_forceKeyframe = true;
    uint64_t m_sampleIndex = 0;

    ID3D11Texture2D*          m_scaledTexture = nullptr;
    ID3D11ShaderResourceView* m_srv = nullptr;
    ID3D11RenderTargetView*   m_rtv = nullptr;

    OnFrameEncodedFn m_callback = nullptr;
};
