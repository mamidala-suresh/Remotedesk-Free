// RemoteDesk Free - DXGI Desktop Duplicator Implementation
// Copyright (C) 2026 Suresh Mamidala | Freeware

#include "dxgi_duplicator.h"
#include <iostream>

DxgiDuplicator::DxgiDuplicator() {}

DxgiDuplicator::~DxgiDuplicator() {
    Release();
}

std::vector<RdMonitorInfo> DxgiDuplicator::EnumerateMonitors() {
    std::vector<RdMonitorInfo> result;

    IDXGIFactory1* factory = nullptr;
    if (FAILED(CreateDXGIFactory1(__uuidof(IDXGIFactory1), (void**)&factory))) {
        return result;
    }

    IDXGIAdapter1* adapter = nullptr;
    uint32_t adapterIdx = 0;
    uint32_t totalMonitors = 0;

    while (factory->EnumAdapters1(adapterIdx++, &adapter) != DXGI_ERROR_NOT_FOUND) {
        IDXGIOutput* output = nullptr;
        uint32_t outputIdx = 0;

        while (adapter->EnumOutputs(outputIdx++, &output) != DXGI_ERROR_NOT_FOUND) {
            DXGI_OUTPUT_DESC desc;
            if (SUCCEEDED(output->GetDesc(&desc))) {
                RdMonitorInfo info = {};
                info.index = totalMonitors++;
                info.width = desc.DesktopCoordinates.right - desc.DesktopCoordinates.left;
                info.height = desc.DesktopCoordinates.bottom - desc.DesktopCoordinates.top;
                info.left = desc.DesktopCoordinates.left;
                info.top = desc.DesktopCoordinates.top;
                info.is_primary = (info.left == 0 && info.top == 0);

                // Convert wchar device name
                WideCharToMultiByte(CP_UTF8, 0, desc.DeviceName, -1, info.device_name, sizeof(info.device_name) - 1, nullptr, nullptr);
                result.push_back(info);
            }
            output->Release();
        }
        adapter->Release();
    }

    factory->Release();
    return result;
}

bool DxgiDuplicator::InitializeD3D11() {
    if (m_d3d11Device && m_d3d11Context) return true;

    D3D_FEATURE_LEVEL featureLevels[] = {
        D3D_FEATURE_LEVEL_11_1,
        D3D_FEATURE_LEVEL_11_0,
        D3D_FEATURE_LEVEL_10_1,
        D3D_FEATURE_LEVEL_10_0
    };
    D3D_FEATURE_LEVEL featureLevel;

    UINT creationFlags = D3D11_CREATE_DEVICE_BGRA_SUPPORT;
#if defined(_DEBUG)
    // creationFlags |= D3D11_CREATE_DEVICE_DEBUG;
#endif

    HRESULT hr = D3D11CreateDevice(
        nullptr,
        D3D_DRIVER_TYPE_HARDWARE,
        nullptr,
        creationFlags,
        featureLevels,
        ARRAYSIZE(featureLevels),
        D3D11_SDK_VERSION,
        &m_d3d11Device,
        &featureLevel,
        &m_d3d11Context
    );

    if (FAILED(hr)) {
        // Fallback to WARP (High performance software rasterizer) if hardware adapter is restricted
        hr = D3D11CreateDevice(
            nullptr,
            D3D_DRIVER_TYPE_WARP,
            nullptr,
            creationFlags,
            featureLevels,
            ARRAYSIZE(featureLevels),
            D3D11_SDK_VERSION,
            &m_d3d11Device,
            &featureLevel,
            &m_d3d11Context
        );
    }

    return SUCCEEDED(hr);
}

bool DxgiDuplicator::InitializeDuplication(uint32_t monitorIndex) {
    if (!m_d3d11Device) return false;

    IDXGIDevice* dxgiDevice = nullptr;
    if (FAILED(m_d3d11Device->QueryInterface(__uuidof(IDXGIDevice), (void**)&dxgiDevice))) {
        return false;
    }

    IDXGIAdapter* adapter = nullptr;
    if (FAILED(dxgiDevice->GetAdapter(&adapter))) {
        dxgiDevice->Release();
        return false;
    }

    IDXGIOutput* output = nullptr;
    if (FAILED(adapter->EnumOutputs(monitorIndex, &output))) {
        adapter->Release();
        dxgiDevice->Release();
        return false;
    }

    DXGI_OUTPUT_DESC desc;
    output->GetDesc(&desc);
    m_width = desc.DesktopCoordinates.right - desc.DesktopCoordinates.left;
    m_height = desc.DesktopCoordinates.bottom - desc.DesktopCoordinates.top;

    IDXGIOutput1* output1 = nullptr;
    if (FAILED(output->QueryInterface(__uuidof(IDXGIOutput1), (void**)&output1))) {
        output->Release();
        adapter->Release();
        dxgiDevice->Release();
        return false;
    }

    HRESULT hr = output1->DuplicateOutput(m_d3d11Device, &m_duplication);

    output1->Release();
    output->Release();
    adapter->Release();
    dxgiDevice->Release();

    return SUCCEEDED(hr);
}

bool DxgiDuplicator::Initialize(uint32_t monitorIndex) {
    Release();
    m_monitorIndex = monitorIndex;

    if (!InitializeD3D11()) {
        std::cerr << "[DXGI] Failed to initialize Direct3D 11 device." << std::endl;
        return false;
    }

    if (!InitializeDuplication(monitorIndex)) {
        std::cerr << "[DXGI] Failed to duplicate output for monitor " << monitorIndex << std::endl;
        return false;
    }

    return true;
}

void DxgiDuplicator::ReleaseFrame() {
    if (m_hasAcquiredFrame && m_duplication) {
        m_duplication->ReleaseFrame();
        m_hasAcquiredFrame = false;
    }
    if (m_acquiredTexture) {
        m_acquiredTexture->Release();
        m_acquiredTexture = nullptr;
    }
}

bool DxgiDuplicator::AcquireFrame(
    ID3D11Texture2D** outTexture,
    uint32_t timeoutMs,
    bool& outCursorUpdated,
    POINT& outCursorPos,
    bool& outCursorVisible
) {
    outCursorUpdated = false;
    if (!m_duplication) {
        // Attempt automatic re-initialization on mode change or access lost
        if (!InitializeDuplication(m_monitorIndex)) {
            return false;
        }
    }

    ReleaseFrame();

    DXGI_OUTDUPL_FRAME_INFO frameInfo;
    IDXGIResource* desktopResource = nullptr;

    HRESULT hr = m_duplication->AcquireNextFrame(timeoutMs, &frameInfo, &desktopResource);

    if (hr == DXGI_ERROR_WAIT_TIMEOUT) {
        return false; // No new screen changes within timeout window
    }

    if (hr == DXGI_ERROR_ACCESS_LOST || hr == DXGI_ERROR_DEVICE_RESET || hr == DXGI_ERROR_INVALID_CALL) {
        // Desktop switch (e.g. UAC, login screen, resolution change)
        if (m_duplication) {
            m_duplication->Release();
            m_duplication = nullptr;
        }
        return false;
    }

    if (FAILED(hr) || !desktopResource) {
        return false;
    }

    m_hasAcquiredFrame = true;

    // Check pointer visibility and coordinates
    if (frameInfo.PointerPosition.Visible) {
        outCursorVisible = true;
        outCursorPos.x = frameInfo.PointerPosition.Position.x;
        outCursorPos.y = frameInfo.PointerPosition.Position.y;
        outCursorUpdated = true;
    } else {
        outCursorVisible = false;
    }

    ID3D11Texture2D* acquiredTex = nullptr;
    hr = desktopResource->QueryInterface(__uuidof(ID3D11Texture2D), (void**)&acquiredTex);
    desktopResource->Release();

    if (FAILED(hr)) {
        ReleaseFrame();
        return false;
    }

    // Create or reuse internal staging/copy texture
    D3D11_TEXTURE2D_DESC desc;
    acquiredTex->GetDesc(&desc);
    desc.BindFlags = D3D11_BIND_SHADER_RESOURCE | D3D11_BIND_RENDER_TARGET;
    desc.MiscFlags = D3D11_RESOURCE_MISC_SHARED; // Allows zero-copy sharing across processes / WPF
    desc.Usage = D3D11_USAGE_DEFAULT;
    desc.CPUAccessFlags = 0;

    ID3D11Texture2D* copyTex = nullptr;
    hr = m_d3d11Device->CreateTexture2D(&desc, nullptr, &copyTex);
    if (SUCCEEDED(hr)) {
        m_d3d11Context->CopyResource(copyTex, acquiredTex);
        m_acquiredTexture = copyTex;
        *outTexture = copyTex;
        copyTex->AddRef();
    }

    acquiredTex->Release();
    return SUCCEEDED(hr);
}

void DxgiDuplicator::Release() {
    ReleaseFrame();

    if (m_duplication) {
        m_duplication->Release();
        m_duplication = nullptr;
    }
    if (m_d3d11Context) {
        m_d3d11Context->Release();
        m_d3d11Context = nullptr;
    }
    if (m_d3d11Device) {
        m_d3d11Device->Release();
        m_d3d11Device = nullptr;
    }
}
