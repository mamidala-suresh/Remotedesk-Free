// RemoteDesk Free - GPU H.264 Hardware Encoder Implementation
// Copyright (C) 2026 Suresh Mamidala | Freeware

#include "gpu_h264_encoder.h"
#include <mfapi.h>
#include <mferror.h>
#include <wmcodecdsp.h>
#include <iostream>

static const GUID IID_ICodecAPI_Local = { 0x901db4c7, 0x31ce, 0x41a2, { 0x85, 0xdc, 0x8f, 0xa0, 0xbf, 0x41, 0xb8, 0xda } };

GpuH264Encoder::GpuH264Encoder() {}

GpuH264Encoder::~GpuH264Encoder() {
    Release();
}

bool GpuH264Encoder::Initialize(
    ID3D11Device* device,
    uint32_t width,
    uint32_t height,
    uint32_t fps,
    uint32_t bitrateKbps,
    uint32_t scalePercent,
    OnFrameEncodedFn callback
) {
    Release();

    m_device = device;
    m_device->AddRef();
    m_device->GetImmediateContext(&m_context);

    m_srcWidth = width;
    m_srcHeight = height;
    m_fps = fps > 0 ? fps : 30;
    m_bitrate = (bitrateKbps > 0 ? bitrateKbps : 2500) * 1000;
    m_scalePercent = scalePercent > 0 ? scalePercent : 100;
    m_callback = callback;

    if (m_scalePercent < 100) {
        m_targetWidth = ((width * m_scalePercent / 100) & ~1);
        m_targetHeight = ((height * m_scalePercent / 100) & ~1);
    } else {
        m_targetWidth = width;
        m_targetHeight = height;
    }

    // Startup Windows Media Foundation
    MFStartup(MF_VERSION, MFSTARTUP_NOSOCKET);

    if (!SetupEncoderMFT()) {
        std::cerr << "[Encoder] Failed to initialize Media Foundation H.264 MFT." << std::endl;
        return false;
    }

    return true;
}

bool GpuH264Encoder::SetupEncoderMFT() {
    MFT_REGISTER_TYPE_INFO outType = { MFMediaType_Video, MFVideoFormat_H264 };
    IMFActivate** activates = nullptr;
    UINT32 count = 0;

    // Search for hardware-accelerated GPU H.264 encoders (NVENC, AMF, QuickSync)
    HRESULT hr = MFTEnumEx(
        MFT_CATEGORY_VIDEO_ENCODER,
        MFT_ENUM_FLAG_HARDWARE | MFT_ENUM_FLAG_SORTANDFILTER,
        nullptr,
        &outType,
        &activates,
        &count
    );

    if (FAILED(hr) || count == 0) {
        // Fallback to all available H.264 encoders (including software/hybrid MFT)
        hr = MFTEnumEx(
            MFT_CATEGORY_VIDEO_ENCODER,
            MFT_ENUM_FLAG_ALL,
            nullptr,
            &outType,
            &activates,
            &count
        );
    }

    if (FAILED(hr) || count == 0) {
        return false;
    }

    // Activate the primary GPU hardware encoder
    hr = activates[0]->ActivateObject(__uuidof(IMFTransform), (void**)&m_encoderMft);

    // Free activation references
    for (UINT32 i = 0; i < count; ++i) {
        activates[i]->Release();
    }
    CoTaskMemFree(activates);

    if (FAILED(hr) || !m_encoderMft) {
        return false;
    }

    // Query ICodecAPI for low-latency interactive streaming
    if (SUCCEEDED(m_encoderMft->QueryInterface(IID_ICodecAPI_Local, (void**)&m_codecApi))) {
        VARIANT var;
        VariantInit(&var);

        // 1. Enable Zero-Latency Real-Time Streaming mode
        var.vt = VT_BOOL;
        var.boolVal = VARIANT_TRUE;
        m_codecApi->SetValue(&CODECAPI_AVLowLatencyMode, &var);

        // 2. Set Rate Control Mode to CBR (Constant Bitrate) for smooth networking
        var.vt = VT_UI4;
        var.ulVal = eAVEncCommonRateControlMode_CBR;
        m_codecApi->SetValue(&CODECAPI_AVEncCommonRateControlMode, &var);

        // 3. Set Mean Bitrate
        var.vt = VT_UI4;
        var.ulVal = m_bitrate;
        m_codecApi->SetValue(&CODECAPI_AVEncCommonMeanBitRate, &var);

        // 4. Set GOP Keyframe interval (1 keyframe per second)
        var.vt = VT_UI4;
        var.ulVal = m_fps;
        m_codecApi->SetValue(&CODECAPI_AVEncMPVGOPSize, &var);
    }

    return CreateMediaTypes();
}

bool GpuH264Encoder::CreateMediaTypes() {
    if (!m_encoderMft) return false;

    // 1. Configure Output Type (H.264)
    IMFMediaType* outType = nullptr;
    HRESULT hr = MFCreateMediaType(&outType);
    if (FAILED(hr)) return false;

    hr = outType->SetGUID(MF_MT_MAJOR_TYPE, MFMediaType_Video);
    hr = outType->SetGUID(MF_MT_SUBTYPE, MFVideoFormat_H264);
    hr = outType->SetUINT32(MF_MT_AVG_BITRATE, m_bitrate);
    hr = MFSetAttributeSize(outType, MF_MT_FRAME_SIZE, m_targetWidth, m_targetHeight);
    hr = MFSetAttributeRatio(outType, MF_MT_FRAME_RATE, m_fps, 1);
    hr = MFSetAttributeRatio(outType, MF_MT_PIXEL_ASPECT_RATIO, 1, 1);
    hr = outType->SetUINT32(MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive);
    hr = outType->SetUINT32(MF_MT_MPEG2_PROFILE, eAVEncH264VProfile_Main);

    hr = m_encoderMft->SetOutputType(0, outType, 0);
    outType->Release();
    if (FAILED(hr)) return false;

    // 2. Configure Input Type (NV12 or ARGB)
    IMFMediaType* inType = nullptr;
    hr = MFCreateMediaType(&inType);
    if (FAILED(hr)) return false;

    hr = inType->SetGUID(MF_MT_MAJOR_TYPE, MFMediaType_Video);
    hr = inType->SetGUID(MF_MT_SUBTYPE, MFVideoFormat_NV12);
    hr = MFSetAttributeSize(inType, MF_MT_FRAME_SIZE, m_targetWidth, m_targetHeight);
    hr = MFSetAttributeRatio(inType, MF_MT_FRAME_RATE, m_fps, 1);
    hr = MFSetAttributeRatio(inType, MF_MT_PIXEL_ASPECT_RATIO, 1, 1);
    hr = inType->SetUINT32(MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive);

    hr = m_encoderMft->SetInputType(0, inType, 0);
    if (FAILED(hr)) {
        // Try RGB32 fallback if encoder prefers RGB
        inType->SetGUID(MF_MT_SUBTYPE, MFVideoFormat_RGB32);
        hr = m_encoderMft->SetInputType(0, inType, 0);
    }
    inType->Release();
    if (FAILED(hr)) return false;

    // Notify MFT to begin streaming
    m_encoderMft->ProcessMessage(MFT_MESSAGE_NOTIFY_BEGIN_STREAMING, 0);
    m_encoderMft->ProcessMessage(MFT_MESSAGE_NOTIFY_START_OF_STREAM, 0);

    return true;
}

void GpuH264Encoder::ForceKeyframe() {
    m_forceKeyframe = true;
    if (m_codecApi) {
        VARIANT var;
        VariantInit(&var);
        var.vt = VT_UI4;
        var.ulVal = 1;
        m_codecApi->SetValue(&CODECAPI_AVEncVideoForceKeyFrame, &var);
    }
}

void GpuH264Encoder::SetBitrateAndFps(uint32_t bitrateKbps, uint32_t fps) {
    if (bitrateKbps > 0) m_bitrate = bitrateKbps * 1000;
    if (fps > 0) m_fps = fps;

    if (m_codecApi) {
        VARIANT var;
        VariantInit(&var);
        var.vt = VT_UI4;
        var.ulVal = m_bitrate;
        m_codecApi->SetValue(&CODECAPI_AVEncCommonMeanBitRate, &var);
    }
}

bool GpuH264Encoder::EncodeFrame(ID3D11Texture2D* texture, uint64_t timestampMs) {
    if (!m_encoderMft || !texture) return false;

    // Check if keyframe was requested
    if (m_forceKeyframe) {
        m_forceKeyframe = false;
        if (m_codecApi) {
            VARIANT var;
            VariantInit(&var);
            var.vt = VT_UI4;
            var.ulVal = 1;
            m_codecApi->SetValue(&CODECAPI_AVEncVideoForceKeyFrame, &var);
        }
    }

    // 1. Map texture or copy to staging buffer for MFT feeding
    D3D11_TEXTURE2D_DESC desc;
    texture->GetDesc(&desc);

    D3D11_TEXTURE2D_DESC stagingDesc = desc;
    stagingDesc.Usage = D3D11_USAGE_STAGING;
    stagingDesc.BindFlags = 0;
    stagingDesc.CPUAccessFlags = D3D11_CPU_ACCESS_READ;
    stagingDesc.MiscFlags = 0;

    ID3D11Texture2D* stagingTex = nullptr;
    if (FAILED(m_device->CreateTexture2D(&stagingDesc, nullptr, &stagingTex))) {
        return false;
    }

    m_context->CopyResource(stagingTex, texture);

    D3D11_MAPPED_SUBRESOURCE mapped;
    if (FAILED(m_context->Map(stagingTex, 0, D3D11_MAP_READ, 0, &mapped))) {
        stagingTex->Release();
        return false;
    }

    // Convert BGRA to NV12 for standard hardware encoder compatibility
    uint32_t nv12Size = m_targetWidth * m_targetHeight * 3 / 2;
    std::vector<uint8_t> nv12Buffer(nv12Size);

    uint8_t* yPlane = nv12Buffer.data();
    uint8_t* uvPlane = yPlane + (m_targetWidth * m_targetHeight);

    const uint8_t* bgraSrc = (const uint8_t*)mapped.pData;
    uint32_t srcPitch = mapped.RowPitch;

    // Fast SIMD/Integer BT.601 RGB -> YUV conversion
    for (uint32_t y = 0; y < m_targetHeight; ++y) {
        const uint8_t* srcRow = bgraSrc + (y * srcPitch);
        uint8_t* yRow = yPlane + (y * m_targetWidth);

        for (uint32_t x = 0; x < m_targetWidth; ++x) {
            uint8_t b = srcRow[x * 4 + 0];
            uint8_t g = srcRow[x * 4 + 1];
            uint8_t r = srcRow[x * 4 + 2];

            // Y = 0.299R + 0.587G + 0.114B
            yRow[x] = (uint8_t)((66 * r + 129 * g + 25 * b + 128) >> 8) + 16;

            if ((y % 2 == 0) && (x % 2 == 0)) {
                uint32_t uvIdx = (y / 2) * m_targetWidth + x;
                // U = -0.169R - 0.331G + 0.500B
                uvPlane[uvIdx + 0] = (uint8_t)((-38 * r - 74 * g + 112 * b + 128) >> 8) + 128;
                // V = 0.500R - 0.419G - 0.081B
                uvPlane[uvIdx + 1] = (uint8_t)((112 * r - 94 * g - 18 * b + 128) >> 8) + 128;
            }
        }
    }

    m_context->Unmap(stagingTex, 0);
    stagingTex->Release();

    // 2. Wrap into Media Foundation Sample
    IMFSample* sample = nullptr;
    IMFMediaBuffer* buffer = nullptr;

    HRESULT hr = MFCreateSample(&sample);
    if (FAILED(hr)) return false;

    hr = MFCreateMemoryBuffer(nv12Size, &buffer);
    if (SUCCEEDED(hr)) {
        BYTE* dest = nullptr;
        buffer->Lock(&dest, nullptr, nullptr);
        memcpy(dest, nv12Buffer.data(), nv12Size);
        buffer->Unlock();
        buffer->SetCurrentLength(nv12Size);
        sample->AddBuffer(buffer);
        buffer->Release();
    }

    LONGLONG sampleTime = (LONGLONG)(m_sampleIndex++ * (10000000 / m_fps));
    sample->SetSampleTime(sampleTime);
    sample->SetSampleDuration(10000000 / m_fps);

    // 3. Feed input to MFT
    hr = m_encoderMft->ProcessInput(0, sample, 0);
    sample->Release();

    if (FAILED(hr)) return false;

    // 4. Drain encoded H.264 NAL units from MFT output
    MFT_OUTPUT_DATA_BUFFER outputData = {};
    DWORD status = 0;

    IMFSample* outSample = nullptr;
    IMFMediaBuffer* outBuffer = nullptr;
    MFCreateSample(&outSample);
    MFCreateMemoryBuffer(1024 * 1024, &outBuffer);
    outSample->AddBuffer(outBuffer);
    outBuffer->Release();

    outputData.pSample = outSample;

    hr = m_encoderMft->ProcessOutput(0, 1, &outputData, &status);
    if (SUCCEEDED(hr) && outputData.pSample) {
        IMFMediaBuffer* resBuffer = nullptr;
        if (SUCCEEDED(outputData.pSample->ConvertToContiguousBuffer(&resBuffer))) {
            BYTE* h264Data = nullptr;
            DWORD h264Len = 0;
            resBuffer->Lock(&h264Data, nullptr, &h264Len);

            UINT32 isKeyframeVal = 0;
            outputData.pSample->GetUINT32(MFSampleExtension_CleanPoint, &isKeyframeVal);
            bool isKeyframe = (isKeyframeVal != 0);

            if (m_callback && h264Len > 0) {
                m_callback(h264Data, h264Len, m_targetWidth, m_targetHeight, isKeyframe, timestampMs);
            }

            resBuffer->Unlock();
            resBuffer->Release();
        }
    }

    outSample->Release();
    return true;
}

void GpuH264Encoder::Release() {
    if (m_encoderMft) {
        m_encoderMft->ProcessMessage(MFT_MESSAGE_NOTIFY_END_OF_STREAM, 0);
        m_encoderMft->Release();
        m_encoderMft = nullptr;
    }
    if (m_codecApi) {
        m_codecApi->Release();
        m_codecApi = nullptr;
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
