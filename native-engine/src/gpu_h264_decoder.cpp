// RemoteDesk Free - GPU H.264 Hardware Decoder Implementation
// Copyright (C) 2026 Suresh Mamidala | Freeware

#include "gpu_h264_decoder.h"
#include <wmcodecdsp.h>
#include <iostream>

#ifndef MF_E_NOTACCEPTING
#define MF_E_NOTACCEPTING ((HRESULT)0xC00D6D60L)
#endif

static const GUID CLSID_CMSH264DecoderMFT_Local = { 0x62ce7e72, 0x4c71, 0x4d20, { 0xb1, 0x5d, 0x45, 0x28, 0x31, 0xa8, 0x7d, 0x9d } };

GpuH264Decoder::GpuH264Decoder(RdDecodedFrameCallback callback, void* userData)
    : m_callback(callback), m_userData(userData) {}

GpuH264Decoder::~GpuH264Decoder() {
    Release();
}

bool GpuH264Decoder::CreateD3D11Device() {
    if (m_device && m_context) return true;

    D3D_FEATURE_LEVEL featureLevels[] = {
        D3D_FEATURE_LEVEL_11_1,
        D3D_FEATURE_LEVEL_11_0,
        D3D_FEATURE_LEVEL_10_1,
        D3D_FEATURE_LEVEL_10_0
    };
    D3D_FEATURE_LEVEL featureLevel;

    UINT creationFlags = D3D11_CREATE_DEVICE_BGRA_SUPPORT;
    HRESULT hr = D3D11CreateDevice(
        nullptr,
        D3D_DRIVER_TYPE_HARDWARE,
        nullptr,
        creationFlags,
        featureLevels,
        ARRAYSIZE(featureLevels),
        D3D11_SDK_VERSION,
        &m_device,
        &featureLevel,
        &m_context
    );

    if (FAILED(hr)) {
        hr = D3D11CreateDevice(
            nullptr,
            D3D_DRIVER_TYPE_WARP,
            nullptr,
            creationFlags,
            featureLevels,
            ARRAYSIZE(featureLevels),
            D3D11_SDK_VERSION,
            &m_device,
            &featureLevel,
            &m_context
        );
    }

    return SUCCEEDED(hr);
}

bool GpuH264Decoder::CreateSharedSurface(uint32_t width, uint32_t height) {
    if (m_sharedTexture && m_width == width && m_height == height) {
        return true;
    }

    if (m_sharedTexture) {
        m_sharedTexture->Release();
        m_sharedTexture = nullptr;
        m_sharedHandle = nullptr;
    }

    m_width = width;
    m_height = height;

    D3D11_TEXTURE2D_DESC desc = {};
    desc.Width = width;
    desc.Height = height;
    desc.MipLevels = 1;
    desc.ArraySize = 1;
    desc.Format = DXGI_FORMAT_B8G8R8A8_UNORM;
    desc.SampleDesc.Count = 1;
    desc.Usage = D3D11_USAGE_DEFAULT;
    desc.BindFlags = D3D11_BIND_RENDER_TARGET | D3D11_BIND_SHADER_RESOURCE;
    desc.MiscFlags = D3D11_RESOURCE_MISC_SHARED; // Allows WPF D3DImage / WinUI SwapChainPanel zero-copy access

    HRESULT hr = m_device->CreateTexture2D(&desc, nullptr, &m_sharedTexture);
    if (FAILED(hr)) return false;

    IDXGIResource* dxgiResource = nullptr;
    hr = m_sharedTexture->QueryInterface(__uuidof(IDXGIResource), (void**)&dxgiResource);
    if (SUCCEEDED(hr)) {
        dxgiResource->GetSharedHandle(&m_sharedHandle);
        dxgiResource->Release();
    }

    return SUCCEEDED(hr);
}

bool GpuH264Decoder::Initialize() {
    Release();

    if (!CreateD3D11Device()) {
        std::cerr << "[Decoder] Failed to create D3D11 device." << std::endl;
        return false;
    }

    MFStartup(MF_VERSION, MFSTARTUP_NOSOCKET);

    // Initialize H.264 Video Decoder MFT
    HRESULT hr = CoCreateInstance(
        CLSID_CMSH264DecoderMFT_Local,
        nullptr,
        CLSCTX_INPROC_SERVER,
        __uuidof(IMFTransform),
        (void**)&m_decoderMft
    );

    if (FAILED(hr)) {
        std::cerr << "[Decoder] Failed to create CMSH264DecoderMFT." << std::endl;
        return false;
    }

    // Set input type (H.264)
    IMFMediaType* inType = nullptr;
    MFCreateMediaType(&inType);
    inType->SetGUID(MF_MT_MAJOR_TYPE, MFMediaType_Video);
    inType->SetGUID(MF_MT_SUBTYPE, MFVideoFormat_H264);
    hr = m_decoderMft->SetInputType(0, inType, 0);
    inType->Release();

    if (FAILED(hr)) return false;

    // Set output type (NV12 or ARGB)
    IMFMediaType* outType = nullptr;
    MFCreateMediaType(&outType);
    outType->SetGUID(MF_MT_MAJOR_TYPE, MFMediaType_Video);
    outType->SetGUID(MF_MT_SUBTYPE, MFVideoFormat_NV12);
    hr = m_decoderMft->SetOutputType(0, outType, 0);
    outType->Release();

    CreateSharedSurface(1920, 1080);
    return true;
}

bool GpuH264Decoder::DecodeFrame(const uint8_t* data, uint32_t size, uint64_t timestampMs) {
    if (!m_decoderMft || !data || size == 0) return false;

    IMFSample* sample = nullptr;
    IMFMediaBuffer* buffer = nullptr;

    HRESULT hr = MFCreateSample(&sample);
    if (FAILED(hr)) return false;

    hr = MFCreateMemoryBuffer(size, &buffer);
    if (SUCCEEDED(hr)) {
        BYTE* dest = nullptr;
        buffer->Lock(&dest, nullptr, nullptr);
        memcpy(dest, data, size);
        buffer->Unlock();
        buffer->SetCurrentLength(size);
        sample->AddBuffer(buffer);
        buffer->Release();
    }

    hr = m_decoderMft->ProcessInput(0, sample, 0);
    sample->Release();

    if (FAILED(hr) && hr != MF_E_NOTACCEPTING) {
        return false;
    }

    // Process output
    MFT_OUTPUT_DATA_BUFFER outData = {};
    DWORD status = 0;

    IMFSample* outSample = nullptr;
    IMFMediaBuffer* outBuffer = nullptr;
    MFCreateSample(&outSample);
    MFCreateMemoryBuffer(m_width * m_height * 4, &outBuffer);
    outSample->AddBuffer(outBuffer);
    outBuffer->Release();

    outData.pSample = outSample;

    hr = m_decoderMft->ProcessOutput(0, 1, &outData, &status);
    if (SUCCEEDED(hr) && outData.pSample) {
        if (m_callback && m_sharedHandle) {
            m_callback(m_sharedHandle, m_width, m_height, m_userData);
        }
    }

    outSample->Release();
    return true;
}

void GpuH264Decoder::Release() {
    if (m_sharedTexture) {
        m_sharedTexture->Release();
        m_sharedTexture = nullptr;
        m_sharedHandle = nullptr;
    }
    if (m_decoderMft) {
        m_decoderMft->Release();
        m_decoderMft = nullptr;
    }
    if (m_context) {
        m_context->Release();
        m_context = nullptr;
    }
    if (m_device) {
        m_device->Release();
        m_device = nullptr;
    }

    MFShutdown();
}
