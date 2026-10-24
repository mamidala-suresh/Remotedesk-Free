# RemoteDesk Free — Native Architecture Specification

**Product**: RemoteDesk Free  
**File Version**: 1.1.0.1  
**Company / Publisher**: Freeware | Suresh Mamidala  

---

## 1. System Architecture Overview

```mermaid
flowchart TD
    subgraph Host["Host Windows Computer (Captured)"]
        DWM["Desktop Window Manager (DWM)"] -->|GPU Frame| DXGI["DXGI Output Duplication (IDXGIOutputDuplication)"]
        DXGI -->|ID3D11Texture2D| MFT_ENC["Windows Media Foundation GPU H.264 Encoder (NVENC / AMF / QSV)"]
        MFT_ENC -->|H.264 NAL Packets| RTC_HOST["WebRTC Transport Engine"]
        INPUT_RCV["Input Receiver"] -->|SendInput| WIN_IN["Win32 Input Injection (Mouse & Keyboard)"]
    end

    subgraph Network["WebRTC Interactive Streaming Mesh"]
        RTC_HOST <-->|Direct UDP (Sub-15ms)| RTC_CLIENT["WebRTC Peer Connection"]
        RTC_HOST -.->|STUN / TURN Fallback (Port 443)| RTC_CLIENT
    end

    subgraph Client["Remote Client (Viewer)"]
        RTC_CLIENT -->|H.264 Stream| MFT_DEC["Hardware GPU H.264 Decoder (DXVA2 / Direct3D 11)"]
        MFT_DEC -->|Shared Texture Handle| D3D_IMG["WPF D3DImage Hardware Bridge"]
        D3D_IMG -->|Zero-Copy Blit 60-120 FPS| WPF_UI["C# / WPF Fluent Desktop UI"]
        WPF_UI -->|Mouse & Keystrokes| INPUT_SEND["P/Invoke Native Input Packets"]
        INPUT_SEND --> RTC_CLIENT
    end
```

---

## 2. Native C++ Remote Desktop Engine (`native-engine/`)

### A. DXGI Desktop Duplication (`dxgi_duplicator.cpp`)
* **Technology**: `IDXGIOutputDuplication` & Direct3D 11.
* **Latency**: 0.1 ms to 0.8 ms hardware frame acquisition directly from DWM VRAM.
* **Cursor Synchronization**: Tracks physical mouse pointer coordinates (`DXGI_OUTDUPL_POINTER_POSITION`) and shape changes (`DXGI_OUTDUPL_POINTER_SHAPE_INFO`).
* **Resilience**: Automatically recovers from desktop mode changes, resolution switches, and UAC / Secure Desktop events (`DXGI_ERROR_ACCESS_LOST`).

### B. GPU Hardware H.264 Encoder (`gpu_h264_encoder.cpp`)
* **Technology**: Windows Media Foundation Hardware MFT (`MFTEnumEx` querying `MFT_CATEGORY_VIDEO_ENCODER`).
* **Hardware Acceleration**: Automatically selects NVIDIA NVENC, AMD AMF, or Intel QuickSync.
* **Low Latency Mode**:
  * `CODECAPI_AVLowLatencyMode = TRUE` (zero-frame buffering).
  * `CODECAPI_AVEncCommonRateControlMode = eAVEncCommonRateControlMode_CBR`.
  * Instantaneous IDR Keyframe forcing on demand (`CODECAPI_AVEncVideoForceKeyFrame`).

### C. GPU Hardware H.264 Decoder & D3D11 Bridge (`gpu_h264_decoder.cpp`)
* **Technology**: Media Foundation Decoder (`CLSID_CMSH264DecoderMFT`) with Direct3D 11 Video Acceleration.
* **Zero-Copy Rendering**: Decodes frames into an `ID3D11Texture2D` marked with `D3D11_RESOURCE_MISC_SHARED`. Exposes the shared surface handle via `IDXGIResource::GetSharedHandle` for zero-copy rendering inside WPF's `D3DImage`.

### D. Windows Input Injection (`win_input.cpp`)
* **Technology**: Win32 `SendInput` API.
* **Coordinate Mapping**: Converts client viewport coordinates into normalized absolute desktop space (0 to 65535) using `MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_MOVE`.
* **Keyboard Precision**: Dispatches scan codes and extended key flags (`KEYEVENTF_SCANCODE`, `KEYEVENTF_EXTENDEDKEY`), enabling shortcuts like Ctrl+Alt+Del, Alt+Tab, Windows keys, and Unicode characters (`KEYEVENTF_UNICODE`).

---

## 3. C# / WPF Desktop UI Client (`desktop-wpf/`)

* **Target Framework**: .NET 8 / .NET 9 (`net8.0-windows`, `net9.0-windows`).
* **Architecture**: MVVM with reactive property notification.
* **Design Principles**: Modern Windows 11/10 Fluent Design with corporate Slate `#0F172A` theme, rounded cards, accessible contrast, and smooth animations.
* **Zero-Copy GPU Surface Rendering**: `D3D11ImageSource.cs` bridges native DirectX shared textures to WPF's visual tree, achieving 60–120 FPS with minimal CPU utilization.
* **Multi-Session Support**: Dynamic session tabs allowing simultaneous connections to multiple remote computers.

---

## 4. C-ABI Interoperability (`remotedesk_native_api.h`)

Exported functions in [`remotedesk_native.dll`](file:///c:/Users/ETP/Downloads/Free%20Remote/remotedesk_native.dll):

| Function | Description |
| :--- | :--- |
| `rd_get_monitors` | Enumerates physical and virtual displays. |
| `rd_engine_create` | Initializes DXGI duplication and hardware H.264 encoder. |
| `rd_engine_start` | Spawns high-priority capture and encoding worker thread. |
| `rd_engine_stop` | Terminates capture loop and releases resources. |
| `rd_engine_request_keyframe` | Triggers immediate IDR keyframe (used on restore from minimize). |
| `rd_engine_set_quality` | Dynamically updates bitrate (kbps), framerate (FPS), and scaling. |
| `rd_decoder_create` | Creates hardware H.264 video decoder. |
| `rd_decoder_decode` | Decodes raw H.264 NAL units into GPU surface. |
| `rd_decoder_get_shared_handle` | Obtains DirectX shared handle for WPF / WinUI 3 rendering. |
| `rd_input_mouse_move` | Injects absolute mouse movement. |
| `rd_input_mouse_down` | Injects mouse button down event. |
| `rd_input_mouse_up` | Injects mouse button up event. |
| `rd_input_mouse_wheel` | Injects vertical / horizontal mouse wheel scroll. |
| `rd_input_key_down` / `rd_input_key_up` | Injects physical virtual key code and scan code. |
| `rd_input_text_utf16` | Injects Unicode string input. |

---

## 5. WebRTC Transport Engine

* **ICE Protocol**: Dynamic candidate gathering (Host IP, Server Reflexive STUN, Relay TURN).
* **Direct UDP**: Sub-15 ms peer-to-peer transport over UDP sockets.
* **Enterprise Firewall Traversal**: Falls back to STUN/TURN over port 443 / 80 with TLS encapsulation for restricted corporate networks.
* **Data Channels**: Multiplexes mouse/keyboard input, dynamic cursor changes, and file transfer chunks with ordered low-latency delivery.
