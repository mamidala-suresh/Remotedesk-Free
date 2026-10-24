// RemoteDesk Free - DXGI Desktop Duplicator
// Copyright (C) 2026 Suresh Mamidala | Freeware

#pragma once

#include <windows.h>
#include <d3d11.h>
#include <dxgi1_2.h>
#include <stdint.h>
#include <string>
#include <vector>
#include <memory>
#include "remotedesk_native_api.h"

class DxgiDuplicator {
public:
    DxgiDuplicator();
    ~DxgiDuplicator();

    static std::vector<RdMonitorInfo> EnumerateMonitors();

    bool Initialize(uint32_t monitorIndex);
    void Release();

    // Acquires next desktop frame texture. Returns true if frame was updated.
    bool AcquireFrame(ID3D11Texture2D** outTexture, uint32_t timeoutMs, bool& outCursorUpdated, POINT& outCursorPos, bool& outCursorVisible);
    void ReleaseFrame();

    ID3D11Device* GetDevice() const { return m_d3d11Device; }
    ID3D11DeviceContext* GetContext() const { return m_d3d11Context; }
    uint32_t GetWidth() const { return m_width; }
    uint32_t GetHeight() const { return m_height; }

private:
    bool InitializeD3D11();
    bool InitializeDuplication(uint32_t monitorIndex);

    ID3D11Device*           m_d3d11Device = nullptr;
    ID3D11DeviceContext*    m_d3d11Context = nullptr;
    IDXGIOutputDuplication* m_duplication = nullptr;
    ID3D11Texture2D*        m_acquiredTexture = nullptr;

    uint32_t m_monitorIndex = 0;
    uint32_t m_width = 1920;
    uint32_t m_height = 1080;
    bool     m_hasAcquiredFrame = false;
};
