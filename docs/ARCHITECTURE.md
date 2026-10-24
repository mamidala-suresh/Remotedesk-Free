# RemoteDesk Free - System Architecture & Technical Specification

## 1. High-Level Architecture

RemoteDesk Free is designed around a decentralized peer-to-peer topology backed by a lightweight signaling and encrypted relay network.

```text
+-------------------------------------------------------------------------+
|                         RemoteDesk Free Network                         |
+-------------------------------------------------------------------------+
                                    |
          +-------------------------+-------------------------+
          |                                                   |
          v                                                   v
+-------------------+                               +-------------------+
| Host Agent        |                               | Viewer Client     |
| (Target Machine)  |                               | (Tech/Operator)   |
|                   |        Signaling / WS         |                   |
| - GDI Screen      |<=============================>| - HTML5 Canvas    |
|   Capturer        |    [remotedesk-server:8088]   |   Viewer          |
| - JPEG Encoder    |                               | - Mouse/Keyboard  |
| - SendInput Engine|        Direct P2P / Relay     |   Capture         |
| - Local Audit DB  |<----------------------------->| - File Manager UI |
+-------------------+       [Low-Latency Stream]    +-------------------+
```

---

## 2. Core Modules & Crates

The repository is organized as a Cargo workspace with four specialized crates:

1. **`remotedesk-core`**:
   - **Identity Subsystem**: Generates and formats 9-digit AnyDesk-style IDs (`XXX XXX XXX`) from cryptographic hardware fingerprints.
   - **Authentication Subsystem**: Constant-time PIN verification, salted PBKDF2/SHA-256 password hashing for unattended access, rate-limiting brute-force guard.
   - **Protocol Definitions**: Serialization contracts for signaling messages (`ClientMessage`, `ServerMessage`) and stream data packets (`SessionPacket`).
   - **Local Storage**: JSON/SQLite-compatible database for persistent settings, trusted device book, recent sessions, and local security audit records.

2. **`remotedesk-capture`**:
   - **Capture Engine**: High-performance Win32 GDI / BitBlt desktop capture with multi-monitor enumeration (`SM_CMONITORS`, `SM_CXSCREEN`, `SM_CYSCREEN`).
   - **Streaming Encoder**: Converts raw 32-bit BGRA frames to RGB8 JPEG with configurable quality (50–90%) and delta-skip frame hashing to minimize CPU and network utilization on static screens.
   - **Input Injector**: Implements Windows `SendInput` API for mouse moves (normalized to `0..65535`), left/right/middle clicks, mouse wheel, virtual keys, scancodes, and Unicode text typing.

3. **`remotedesk-server`**:
   - **Signaling Engine**: Handles device registration, ID mapping, and peer rendezvous over WebSocket (`/ws`).
   - **Session Routing**: Coordinates connection offers, interactive acceptance/rejection, ICE candidates, and disconnect propagation.
   - **Encrypted Relay Fallback**: Provides bidirectional packet forwarding when direct peer connectivity is prohibited by symmetric NAT or restrictive corporate firewalls.
   - **Administration Console**: Embedded responsive web dashboard (`/admin`) and JSON health status API (`/api/status`, `/api/devices`, `/api/sessions`).

4. **`remotedesk-app`**:
   - **Unified Application Binary**: Can act simultaneously as a host agent (accepting remote assistance) and a viewer client (controlling remote machines).
   - **Embedded UI Engine**: Self-contained local web server (`127.0.0.1:9090`) serving embedded HTML5/CSS3/JavaScript frontend with zero external dependencies.
   - **IPC Bridge**: High-speed local WebSocket bridge multiplexing live video frames, input events, file transfers, and chat between the native Rust core and the web UI.

---

## 3. Streaming & Input Lifecycle

1. **Screen Capture Loop**:
   - Runs in a dedicated background Tokio thread at up to 30–60 FPS.
   - Extracts BGRA pixel buffer from the primary or selected monitor display context.
   - Computes a fast 64-bit sampling hash; if identical to previous frame, skips re-encoding (delta frame skip).
   - Converts pixels to RGB8 and compresses via pure-Rust JPEG encoder.
   - Sends Base64/binary `SessionPacket::ScreenFrame` to remote viewer.

2. **Remote Input Injection**:
   - Canvas in the viewer calculates normalized mouse coordinates relative to remote resolution.
   - Input packets (`MouseMove`, `MouseDown`, `MouseUp`, `MouseWheel`, `KeyDown`, `KeyUp`) are dispatched over the session data channel.
   - Host agent verifies that `SessionPermissions.allow_mouse_keyboard` is enabled before calling `SendInput`.

---

## 4. Dual-Pane File Transfer Protocol

- Files are chunked into 64 KB segments (`CHUNK_SIZE`).
- The sender computes a full SHA-256 cryptographic digest before transmission.
- `FileOffer` packet contains filename, total size, and SHA-256 hash.
- Upon `FileAccept`, chunks are streamed with `FileChunk` packets.
- Destination writes chunks sequentially using atomic disk operations and verifies the SHA-256 hash upon completion before declaring `FileFinished`.
