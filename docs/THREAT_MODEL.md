# RemoteDesk Free - Security & Threat Model Documentation

## 1. Security Philosophy

RemoteDesk Free adheres to zero-trust principles, transparent user consent, and defense-in-depth:
- **No Hidden Background Control**: RemoteDesk Free always displays visible indicators when a remote session is active.
- **Explicit Consent by Default**: Sessions require interactive local approval unless unattended access is explicitly configured by the machine owner.
- **Zero Cloud Telemetry**: No usage metrics, user behaviors, or sensitive data are transmitted to external cloud entities.
- **Standard Cryptography**: Uses industry-standard SHA-256, salted PBKDF2 iterations, and constant-time comparisons.

---

## 2. Threat Analysis & Mitigations

| Threat Vector | Description | RemoteDesk Free Mitigation |
| :--- | :--- | :--- |
| **Brute-Force PIN Guessing** | Attacker repeatedly guesses 6-digit session PINs. | In-memory `BruteForceLimiter` locks out any IP or Device ID after 3 consecutive failures for 120 seconds. PIN comparison uses constant-time byte comparisons to prevent timing side channels. |
| **Unauthorized Access (Unattended)** | Unauthorized party tries to guess master unattended password. | Unattended access is **disabled by default**. When enabled, the master password is never stored in plaintext; it is hashed with a cryptographically secure 32-byte salt using 10,000 rounds of PBKDF2-style SHA-256. |
| **Unauthorized Input / Control** | Viewer attempts to take control when user only granted view-only rights. | The Host Agent enforces `SessionPermissions` strictly before calling Windows `SendInput`. If `allow_mouse_keyboard` is false, input packets are discarded immediately. |
| **Unauthorized File Access** | Attacker attempts to download arbitrary system files. | File transfer requires the `allow_file_transfer` permission flag. File transfers trigger audit logging with file names, sizes, timestamps, and cryptographic hashes. |
| **Arbitrary Command Execution** | Remote viewer tries to run shell commands or install malware. | RemoteDesk Free deliberately **does not implement arbitrary remote shell execution**. All actions are strictly bounded to screen display, mouse/keyboard inputs, and authorized file manager operations. |
| **Traffic Eavesdropping / MitM** | Adversary intercepts traffic on public networks. | Transport channels utilize TLS/WSS over public networks or direct encrypted P2P tunnels. Self-hosted deployments are documented with Caddy/Nginx TLS reverse-proxy configurations. |
| **Session Hijacking / Replay** | Attacker replays intercepted connection tokens. | `SessionToken` instances carry creation and expiration timestamps (`expires_at`), rendering stale or replayed tokens invalid. |

---

## 3. Audit Logging

Every security-sensitive event is persistently recorded in the machine's local audit log (`%APPDATA%\RemoteDeskFree\audit.json`):
- `INCOMING_SESSION_REQUEST`
- `SESSION_ACCEPTED`
- `SESSION_DECLINED`
- `UNATTENDED_LOGIN`
- `SESSION_TERMINATED`
- `UNATTENDED_CONFIG_UPDATED`

Audit logs record the timestamp, event type, peer ID, endpoint IP address, success status, and details.
