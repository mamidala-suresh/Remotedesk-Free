// RemoteDesk Free - GPU H.264 Hardware Decoder & Direct3D 11 Renderer Bridge
// Copyright (C) 2026 Suresh Mamidala | Freeware

#pragma once

#include <windows.h>
#include <d3d11.h>
#include <mfapi.h>
#include <mfidl.h>
#include <mftransform.h>
#include <stdint.h>
#include "remotedesk_native_api.h"

class GpuH264Decoder {
public:
    GpuH264Decoder(RdDecodedFrameCallback callback, void* userData);
    ~GpuH264Decoder();

    bool Initialize();
    void Release();

    // Decodes H.264 NAL buffer and blits to shared D3D11 surface
    bool DecodeFrame(const uint8_t* data, uint32_t size, uint64_t timestampMs);

    // Returns the shared texture handle for zero-copy WPF D3DImage presentation
    void* GetSharedSurfaceHandle() const { return m_sharedHandle; }

    uint32_t GetWidth() const { return m_width; }
    uint32_t GetHeight() const { return m_height; }

private:
    bool CreateD3D11Device();
    bool CreateSharedSurface(uint32_t width, uint32_t height);

    ID3D11Device*        m_device = nullptr;
    ID3D11DeviceContext* m_context = nullptr;
    IMFTransform*        m_decoderMft = nullptr;

    ID3D11Texture2D*     m_sharedTexture = nullptr;
    HANDLE               m_sharedHandle = nullptr;

    uint32_t m_width = 1920;
    uint32_t m_height = 1080;

    RdDecodedFrameCallback m_callback = nullptr;
    void* m_userData = nullptr;
};
