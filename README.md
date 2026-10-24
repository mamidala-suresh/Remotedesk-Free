# RemoteDesk Free

> **Production-Oriented, High-Performance, Open-Source Remote Desktop Application for Windows**  
> *An open, self-hostable alternative to Remote Access Tool without subscription fees, proprietary cloud dependencies, advertisements, or device limits.*

[![Platform](https://img.shields.io/badge/Platform-Windows%2010%20%2F%2011%20(64--bit)-blue.svg)](#)
[![Rust](https://img.shields.io/badge/Language-Rust%202021-orange.svg)](#)
[![License](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-green.svg)](./LICENSE)
[![Tests](https://img.shields.io/badge/Automated%20Tests-25%20Passing-brightgreen.svg)](#automated-testing)
[![Telemetry](https://img.shields.io/badge/Cloud%20Telemetry-Zero%20%2F%20None-success.svg)](#security-and-privacy)

---

## 1. Key Features

- **RemoteDesk-Free-Style 9-Digit Device IDs**: Deterministically generated from hardware fingerprints (e.g. `482 910 231`) for consistent identification across reboots.
- **Real-Time High-FPS Screen Streaming**: Windows GDI / BitBlt capture engine with pure-Rust JPEG compression, delta frame skip, and low-latency HTML5 Canvas rendering.
- **Full Remote Input Control**: Hardware mouse movement (normalized `0..65535`), left/right/middle clicks, dragging, scrolling, virtual keys, scancodes, and Unicode typing using Windows `SendInput`.
- **Multi-Monitor Support**: Real-time display detection and switching between connected monitors.
- **Dual Connection Modes**:
  - **Interactive Support**: Requires explicit permission with an on-screen approval modal showing the remote user's ID, computer name, and IP address.
  - **Unattended IT Access**: Optional, secured with salted PBKDF2/SHA-256 password verification (10,000 iterations).
- **Dual-Pane File Manager**: Secure, chunked (64 KB) file transfers with cryptographic SHA-256 integrity verification.
- **In-Session Support Tools**: Real-time text chat, session metrics (FPS, RTT latency, resolution), and clipboard synchronization.
- **Persistent Local Database**: Stores connection history, address book / saved devices, settings, and local security audit logs (`%APPDATA%\RemoteDeskFree`).
- **Self-Hostable Infrastructure**: Self-contained signaling server, peer rendezvous, encrypted relay fallback, and embedded web admin dashboard.
- **Lightweight & Portable**: Single standalone Windows executable (**1.49 MB**) with zero external runtime dependencies.

---

## 2. Repository Structure

```text
remote-desk-free/
├── Cargo.toml                    # Root Cargo workspace manifest
├── LICENSE                       # MIT / Apache-2.0 Dual License
├── THIRD_PARTY_LICENSES.md       # Third-party notices and licenses
├── docs/
│   ├── ARCHITECTURE.md           # System architecture & protocol specification
│   ├── THREAT_MODEL.md           # Threat model, mitigations, and security
│   ├── DEPLOYMENT.md             # Self-hosted server deployment guide
│   ├── USER_MANUAL.md            # End-user operation & setup guide
│   └── GITHUB_HOSTING_GUIDE.md   # Step-by-step GitHub hosting & releases guide
├── crates/
│   ├── remotedesk-core/          # Protocol models, identity, crypto, storage
│   ├── remotedesk-capture/       # Windows screen capture & SendInput injection
│   ├── remotedesk-server/        # Signaling, relay, & web administration server
│   ├── remotedesk-app/           # Unified desktop application & host agent
│   └── remotedesk-tests/         # Comprehensive integration test suite
├── ui/                           # Modern desktop interface (HTML5/CSS3/JS)
│   ├── index.html                # Dashboard, canvas viewer, file manager
│   ├── styles.css                # Polished dark/light theme stylesheet
│   └── app.js                    # Canvas renderer, input events, IPC client
└── scripts/
    ├── build.ps1                 # Automated Windows release build script
    ├── test.ps1                  # Test suite runner
    ├── run-server.ps1            # Signaling server launcher
    └── run-client.ps1            # Desktop client application launcher
```

---

## 3. Quick Start & Build Instructions

### Prerequisites
- Windows 10 or 11 (64-bit)
- Rust toolchain (`stable-x86_64-pc-windows-gnu`)

### Build Release Binaries
To build the standalone release binaries:
```powershell
.\scripts\build.ps1
```
The output executables will be placed in `target\release\`:
- `target\release\remotedesk-app.exe` (Desktop Application & Host Agent - 1.49 MB)
- `target\release\remotedesk-server.exe` (Signaling, Relay, & Admin Server - 0.84 MB)

---

## 4. Running the Application

### 1. Launch the Signaling & Relay Server
```powershell
.\scripts\run-server.ps1 -Port 8088
```
- Signaling endpoint: `ws://127.0.0.1:8088/ws`
- Web Admin Console: `http://127.0.0.1:8088/admin`

### 2. Launch the Desktop Client Application
```powershell
.\scripts\run-client.ps1
```
The application window opens displaying your unique 9-digit Device ID. Enter a remote Device ID and click **Connect**.

To run in headless background service mode (unattended agent):
```powershell
.\scripts\run-client.ps1 -Daemon
```

---

## 5. Automated Testing

Run all unit, integration, and security test suites:
```powershell
.\scripts\test.ps1
```

### Verified Test Results (23 Tests Passing):
- **`remotedesk-core` (10 tests)**:
  - `identity::tests::test_device_id_validation` [PASS]
  - `identity::tests::test_random_device_id` [PASS]
  - `identity::tests::test_deterministic_fingerprint` [PASS]
  - `auth::tests::test_session_pin_generate_and_verify` [PASS]
  - `auth::tests::test_salted_hash_verification` [PASS]
  - `auth::tests::test_session_token_expiry` [PASS]
  - `auth::tests::test_brute_force_limiter` [PASS]
  - `protocol::tests::test_signaling_serde` [PASS]
  - `protocol::tests::test_session_packet_serde` [PASS]
  - `storage::tests::test_local_database_persistence` [PASS]
- **`remotedesk-capture` (1 test)**:
  - `encoder::tests::test_encode_rgba_to_jpeg_packet` [PASS]
- **`remotedesk-tests` (12 tests)**:
  - `auth_tests::test_salted_password_hash_security` [PASS]
  - `auth_tests::test_session_pin_verification` [PASS]
  - `auth_tests::test_session_token_validation_and_expiration` [PASS]
  - `auth_tests::test_unauthorized_access_rejection_and_rate_limiting` [PASS]
  - `auth_tests::test_session_permission_scoping` [PASS]
  - `file_transfer_tests::test_file_chunking_and_cryptographic_integrity` [PASS]
  - `file_transfer_tests::test_directory_enumeration` [PASS]
  - `protocol_tests::test_device_id_parsing_and_formatting` [PASS]
  - `protocol_tests::test_monitor_info_and_dir_entry` [PASS]
  - `protocol_tests::test_session_packets_multiplexing` [PASS]
  - `protocol_tests::test_signaling_messages_roundtrip` [PASS]
  - `session_e2e_tests::test_signaling_session_lifecycle_and_rejection` [PASS]

---

## 6. Security and Privacy

- **No Remote Shells**: Arbitrary command execution is not implemented to prevent exploitation.
- **Permission Enforcement**: View-only, mouse/keyboard control, clipboard synchronization, and file transfer are individually scoped.
- **Rate-Limiting**: Failed connection attempts trigger an automated lockout to prevent brute-force attacks.
- **Audit Trails**: Security actions are logged locally in `%APPDATA%\RemoteDeskFree\audit.json`.
- **Zero Telemetry**: No user analytics or tracking.
