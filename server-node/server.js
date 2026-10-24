/**
 * RemoteDesk Free - Production Node.js WebRTC Signaling & Rendezvous Server
 * Supports: WebRTC SDP Negotiation, ICE Candidates, coturn Ephemeral Auth, and Admin Dashboard
 */

const http = require('http');
const crypto = require('crypto');
const { WebSocketServer, WebSocket } = require('ws');

const PORT = parseInt(process.env.PORT || '8088', 10);
const COTURN_AUTH_SECRET = process.env.COTURN_AUTH_SECRET || 'remotedesk_turn_secret_2026';
const COTURN_SERVER_URL = process.env.COTURN_SERVER_URL || 'turn:127.0.0.1:3478';

// Registry maps
const onlineDevices = new Map(); // deviceId (string) -> { deviceId, ws, ip, osInfo, appVersion, connectedAt }
const activeSessions = new Map(); // sessionId -> { sessionId, initiatorId, targetId, startedAt }

// HTTP Server
const server = http.createServer((req, res) => {
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type');

  const url = new URL(req.url, `http://${req.headers.host || 'localhost'}`);

  if (url.pathname === '/api/status') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({
      status: 'ok',
      version: '1.0.0',
      runtime: 'Node.js ' + process.version,
      online_devices: onlineDevices.size,
      active_sessions: activeSessions.size,
      uptime_seconds: Math.floor(process.uptime()),
    }));
    return;
  }

  // Generate coturn time-windowed ephemeral credentials (RFC 5766)
  if (url.pathname === '/api/turn-credentials') {
    const ttlSeconds = 3600 * 24; // 24 hours
    const expiryTimestamp = Math.floor(Date.now() / 1000) + ttlSeconds;
    const username = `${expiryTimestamp}:remotedesk_user`;
    const hmac = crypto.createHmac('sha1', COTURN_AUTH_SECRET);
    hmac.update(username);
    const password = hmac.digest('base64');

    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({
      iceServers: [
        { urls: 'stun:stun.l.google.com:19302' },
        { urls: 'stun:stun.cloudflare.com:3478' },
        {
          urls: COTURN_SERVER_URL,
          username: username,
          credential: password,
        }
      ]
    }));
    return;
  }

  // Admin Dashboard
  if (url.pathname === '/' || url.pathname === '/admin') {
    res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
    res.end(renderAdminHtml());
    return;
  }

  res.writeHead(404, { 'Content-Type': 'text/plain' });
  res.end('Not Found');
});

// WebSocket Signaling Server
const wss = new WebSocketServer({ server, path: '/ws' });

wss.on('connection', (ws, req) => {
  const remoteIp = req.socket.remoteAddress || '127.0.0.1';
  let registeredDeviceId = null;

  ws.on('message', (data) => {
    try {
      const msg = JSON.parse(data.toString());
      handleMessage(ws, msg, remoteIp, (id) => {
        registeredDeviceId = id;
      });
    } catch (e) {
      send(ws, {
        type: 'Error',
        payload: { code: 'INVALID_JSON', message: e.message }
      });
    }
  });

  ws.on('close', () => {
    if (registeredDeviceId) {
      onlineDevices.delete(registeredDeviceId);
      console.log(`[Signaling] Device ${registeredDeviceId} disconnected.`);

      // Notify affected active sessions
      for (const [sessionId, session] of activeSessions.entries()) {
        if (session.initiatorId === registeredDeviceId || session.targetId === registeredDeviceId) {
          activeSessions.delete(sessionId);
          const peerId = session.initiatorId === registeredDeviceId ? session.targetId : session.initiatorId;
          const peer = onlineDevices.get(peerId);
          if (peer && peer.ws.readyState === WebSocket.OPEN) {
            send(peer.ws, {
              type: 'SessionClosed',
              payload: { session_id: sessionId, reason: 'Remote peer disconnected' }
            });
          }
        }
      }
    }
  });
});

function handleMessage(ws, msg, remoteIp, setRegisteredId) {
  const type = msg.type;
  const payload = msg.payload || {};

  switch (type) {
    case 'Register': {
      const deviceId = typeof payload.device_id === 'object' ? payload.device_id.DeviceId || payload.device_id : payload.device_id;
      const cleanId = String(deviceId).replace(/\D/g, '');

      onlineDevices.set(cleanId, {
        deviceId: cleanId,
        ws,
        ip: remoteIp,
        osInfo: payload.os_info || 'Unknown OS',
        appVersion: payload.app_version || '1.0.0',
        connectedAt: new Date().toISOString()
      });

      setRegisteredId(cleanId);
      console.log(`[Signaling] Registered device ${cleanId} from ${remoteIp}`);

      send(ws, {
        type: 'Registered',
        payload: {
          success: true,
          message: 'Device registered successfully',
          assigned_id: cleanId
        }
      });
      break;
    }

    case 'ConnectRequest': {
      const targetId = String(payload.target_id).replace(/\D/g, '');
      const target = onlineDevices.get(targetId);

      if (!target || target.ws.readyState !== WebSocket.OPEN) {
        send(ws, {
          type: 'Error',
          payload: {
            code: 'PEER_OFFLINE',
            message: `Device ${targetId} is currently offline or unreachable.`
          }
        });
        return;
      }

      const sessionId = 'sess_' + crypto.randomBytes(6).toString('hex');
      let senderId = 'Unknown';
      for (const [id, dev] of onlineDevices.entries()) {
        if (dev.ws === ws) {
          senderId = id;
          break;
        }
      }

      activeSessions.set(sessionId, {
        sessionId,
        initiatorId: senderId,
        targetId,
        startedAt: new Date().toISOString()
      });

      // Forward incoming request to target
      send(target.ws, {
        type: 'IncomingSession',
        payload: {
          session_id: sessionId,
          peer_id: senderId,
          peer_name: `Remote Client (${senderId})`,
          peer_ip: remoteIp,
          requested_permissions: payload.requested_permissions || {
            allow_screen_view: true,
            allow_mouse_keyboard: true,
            allow_clipboard: true,
            allow_file_transfer: true,
            allow_audio: false
          }
        }
      });
      break;
    }

    case 'SessionResponse': {
      const session = activeSessions.get(payload.session_id);
      if (!session) return;

      const initiator = onlineDevices.get(session.initiatorId);
      if (!initiator || initiator.ws.readyState !== WebSocket.OPEN) return;

      if (payload.accepted) {
        send(initiator.ws, {
          type: 'SessionEstablished',
          payload: {
            session_id: payload.session_id,
            peer_id: session.targetId,
            permissions: payload.granted_permissions || {},
            relay_url: `/relay?session=${payload.session_id}`
          }
        });
      } else {
        activeSessions.delete(payload.session_id);
        send(initiator.ws, {
          type: 'SessionRejected',
          payload: {
            session_id: payload.session_id,
            reason: payload.reason || 'Session rejected by host'
          }
        });
      }
      break;
    }

    // WebRTC SDP Offer / Answer Exchange
    case 'SessionOffer': {
      const session = activeSessions.get(payload.session_id);
      if (!session) return;
      const target = onlineDevices.get(session.targetId);
      if (target && target.ws.readyState === WebSocket.OPEN) {
        send(target.ws, {
          type: 'PeerOffer',
          payload: { session_id: payload.session_id, sdp: payload.sdp }
        });
      }
      break;
    }

    case 'SessionAnswer': {
      const session = activeSessions.get(payload.session_id);
      if (!session) return;
      const initiator = onlineDevices.get(session.initiatorId);
      if (initiator && initiator.ws.readyState === WebSocket.OPEN) {
        send(initiator.ws, {
          type: 'PeerAnswer',
          payload: { session_id: payload.session_id, sdp: payload.sdp }
        });
      }
      break;
    }

    // WebRTC ICE Candidate Exchange
    case 'IceCandidate': {
      const session = activeSessions.get(payload.session_id);
      if (!session) return;
      let otherId = session.initiatorId;
      for (const [id, dev] of onlineDevices.entries()) {
        if (dev.ws === ws) {
          otherId = (id === session.initiatorId) ? session.targetId : session.initiatorId;
          break;
        }
      }
      const peer = onlineDevices.get(otherId);
      if (peer && peer.ws.readyState === WebSocket.OPEN) {
        send(peer.ws, {
          type: 'PeerIceCandidate',
          payload: { session_id: payload.session_id, candidate: payload.candidate }
        });
      }
      break;
    }

    case 'Disconnect': {
      activeSessions.delete(payload.session_id);
      break;
    }

    case 'Ping': {
      send(ws, { type: 'Pong', payload: {} });
      break;
    }
  }
}

function send(ws, obj) {
  if (ws && ws.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify(obj));
  }
}

function renderAdminHtml() {
  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>RemoteDesk Free - Node.js Signaling & WebRTC Server</title>
  <style>
    body { font-family: -apple-system, sans-serif; background: #0f141c; color: #f8fafc; padding: 24px; }
    .card { background: #19202c; border: 1px solid #334155; border-radius: 8px; padding: 20px; margin-bottom: 20px; }
    .badge { background: #10b981; color: #fff; padding: 3px 8px; border-radius: 12px; font-size: 12px; font-weight: bold; }
    h1 { margin-top: 0; }
    table { width: 100%; border-collapse: collapse; margin-top: 12px; }
    th, td { text-align: left; padding: 10px; border-bottom: 1px solid #334155; }
    th { color: #94a3b8; }
  </style>
</head>
<body>
  <h1>RemoteDesk Free <span class="badge">Node.js + WebRTC + coturn</span></h1>
  <div class="card">
    <h3>Server Health & Metrics</h3>
    <p>Online Devices: <strong>${onlineDevices.size}</strong> | Active Sessions: <strong>${activeSessions.size}</strong></p>
    <p>Signaling: <code>ws://localhost:${PORT}/ws</code> | WebRTC STUN/TURN: <code>${COTURN_SERVER_URL}</code></p>
  </div>
  <div class="card">
    <h3>Online Registered Devices</h3>
    <table>
      <thead><tr><th>Device ID</th><th>IP Address</th><th>Platform</th><th>Connected At</th></tr></thead>
      <tbody>
        ${Array.from(onlineDevices.values()).map(d => `<tr><td>${d.deviceId}</td><td>${d.ip}</td><td>${d.osInfo}</td><td>${d.connectedAt}</td></tr>`).join('') || '<tr><td colspan="4" style="color:#94a3b8">No devices connected.</td></tr>'}
      </tbody>
    </table>
  </div>
</body>
</html>`;
}

server.listen(PORT, '0.0.0.0', () => {
  console.log(`============================================================`);
  console.log(`  RemoteDesk Free - Node.js Signaling & WebRTC Server`);
  console.log(`  Listening on: ws://0.0.0.0:${PORT}/ws`);
  console.log(`  Admin & Health: http://localhost:${PORT}/`);
  console.log(`  coturn URL: ${COTURN_SERVER_URL}`);
  console.log(`============================================================`);
});
