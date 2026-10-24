# RemoteDesk Free - Self-Hosted Server Deployment Guide

The `remotedesk-server` binary provides signaling, peer rendezvous, low-latency relay fallback, and an embedded web administration console.

---

## 1. Quick Start

### Running on Windows:
```powershell
# From repository root
.\scripts\run-server.ps1 -Port 8088
```

### Running the Pre-Compiled Binary Directly:
```bash
./remotedesk-server.exe
```

By default, the server binds to `0.0.0.0:8088`.

Endpoints available:
- **Signaling WebSocket**: `ws://<your-server-ip>:8088/ws`
- **Relay Fallback**: `ws://<your-server-ip>:8088/relay`
- **Web Admin Dashboard**: `http://<your-server-ip>:8088/admin`
- **Health Check API**: `http://<your-server-ip>:8088/api/status`

---

## 2. Production TLS Setup with Caddy (Recommended)

Caddy automatically obtains and renews Let's Encrypt SSL/TLS certificates:

```caddyfile
# /etc/caddy/Caddyfile
remote.yourdomain.com {
    # WebSocket Signaling and Relay
    reverse_proxy /ws 127.0.0.1:8088
    reverse_proxy /relay* 127.0.0.1:8088

    # Web Administration Console & APIs
    reverse_proxy /* 127.0.0.1:8088
}
```

Once configured, client applications can set their signaling server URL to:
```text
wss://remote.yourdomain.com/ws
```

---

## 3. Production TLS Setup with Nginx

```nginx
server {
    listen 443 ssl http2;
    server_name remote.yourdomain.com;

    ssl_certificate /etc/letsencrypt/live/remote.yourdomain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/remote.yourdomain.com/privkey.pem;

    location / {
        proxy_pass http://127.0.0.1:8088;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_read_timeout 86400;
    }
}
```

---

## 4. Firewall & Network Ports

| Port | Protocol | Purpose | Direction |
| :--- | :--- | :--- | :--- |
| **8088** | TCP | Signaling, Relay, & Web Admin | Inbound |
| **80 / 443** | TCP | Standard HTTP/HTTPS via Reverse Proxy | Inbound |

---

## 5. Web Administration Console

Navigate to `http://<server-ip>:8088/admin` in any web browser to view:
- Total online registered devices.
- Active concurrent remote sessions.
- Endpoint IP addresses, client operating systems, and client software versions.
- Real-time connection durations.
