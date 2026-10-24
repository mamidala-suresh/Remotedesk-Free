// RemoteDesk Free - Main Desktop Application Frontend Logic
let ipcSocket = null;
let currentAppConfig = null;
let isSessionActive = false;
let currentSessionId = null;
let pendingIncomingPeerId = null;
let activeCanvas = null;
let canvasCtx = null;
let frameCount = 0;
let lastFpsTime = performance.now();
let lastCloudFrameTime = 0;

// Multi-Client Viewer State
const activeSessions = new Map(); // peerId -> { sessionId, peerId, lastFrame, remotePath, remoteFiles, chatHistory }
let activeSessionPeerId = null;

// Address Book & Saved Devices State
let allDevicesList = [];
try {
  const cachedDevs = localStorage.getItem("remotedesk_devices");
  if (cachedDevs) {
    allDevicesList = JSON.parse(cachedDevs);
  }
} catch (_) {}

// File Manager State
let currentLocalPath = "C:\\";
let currentRemotePath = "C:\\";
let selectedLocalFile = null;
let selectedRemoteFile = null;
let userDirs = { home: "C:\\", downloads: "C:\\", desktop: "C:\\", documents: "C:\\" };
let lastFileListRequester = null;
let lastFileDownloadRequester = null;

function setHostSessionActive(active, peerId) {
  const statusDot = document.getElementById("agent-status-dot");
  const statusText = document.getElementById("agent-status-text");
  const hostWidget = document.getElementById("host-session-widget");
  const hostPeerText = document.getElementById("host-session-peer-text");
  const hostClientsCount = document.getElementById("host-clients-count");

  if (active) {
    if (statusDot) statusDot.className = "status-dot online";
    const peerCount = (globalSignaling && globalSignaling.activePeers) ? globalSignaling.activePeers.size : 1;
    if (statusText) statusText.innerText = `In Session • ${peerCount} Remote Client(s) Active`;
    if (hostWidget) hostWidget.style.display = "flex";
    if (hostClientsCount) {
      hostClientsCount.innerText = peerCount === 1 ? "1 Remote Client Connected" : `${peerCount} Remote Clients Connected`;
    }
    if (hostPeerText) {
      const activeIds = (globalSignaling && globalSignaling.activePeers) ? Array.from(globalSignaling.activePeers.keys()) : [];
      hostPeerText.innerText = `Devices: ${activeIds.map(formatDeviceId).join(", ") || formatDeviceId(peerId)}`;
    }
  } else {
    if (statusDot) statusDot.className = "status-dot online";
    if (statusText) statusText.innerText = "Network Ready • Global Rendezvous Active";
    if (hostWidget) hostWidget.style.display = "none";
    const chatDrawer = document.getElementById("chat-drawer");
    if (chatDrawer) chatDrawer.style.display = "none";
  }
}

function formatDeviceId(id) {
  if (!id) return "--- --- ---";
  const clean = String(id).replace(/\s+/g, '');
  if (clean.length === 9) {
    return `${clean.substring(0, 3)} ${clean.substring(3, 6)} ${clean.substring(6, 9)}`;
  }
  return clean;
}

function openChatDrawer() {
  const drawer = document.getElementById("chat-drawer");
  if (drawer) {
    drawer.style.display = "flex";
    const input = document.getElementById("chat-input");
    if (input) input.focus();
  }
}

// WebRTC Peer-to-Peer & coturn Connection Manager
class WebRtcPeerClient {
  constructor() {
    this.pc = null;
    this.dataChannel = null;
    this.iceServers = [
      { urls: "stun:stun.l.google.com:19302" },
      { urls: "stun:stun1.l.google.com:19302" },
      { urls: "stun:stun2.l.google.com:19302" },
      { urls: "stun:stun3.l.google.com:19302" },
      { urls: "stun:stun4.l.google.com:19302" },
      { urls: "stun:stun.cloudflare.com:3478" },
      { urls: "stun:stun.nextcloud.com:443" },
      { urls: "stun:global.stun.twilio.com:3478" },
      { urls: "stun:stun.matrix.org:443" },
      { urls: "stun:stun.services.mozilla.com" },
      { urls: "stun:stun.sipgate.net:3478" },
      { urls: "stun:stun.voip.blackberry.com:3478" },
      { urls: "stun:openrelay.metered.ca:80" },
      {
        urls: [
          "turn:openrelay.metered.ca:80",
          "turn:openrelay.metered.ca:443",
          "turn:openrelay.metered.ca:443?transport=tcp"
        ],
        username: "openrelayproject",
        credential: "openrelayproject"
      }
    ];
    this.pendingCandidates = [];
  }

  init(onDataMessage, onStateChange) {
    this.onDataMessage = onDataMessage;
    this.onStateChange = onStateChange;
    try {
      this.pendingCandidates = [];
      this.pc = new RTCPeerConnection({
        iceServers: this.iceServers,
        iceCandidatePoolSize: 4,
        iceTransportPolicy: "all"
      });

      this.pc.onicecandidate = (event) => {
        if (event.candidate) {
          const candJson = JSON.stringify(event.candidate);
          sendIpcMessage("send_webrtc_candidate", {
            session_id: currentSessionId,
            candidate: candJson
          });
          const target = (globalSignaling && globalSignaling.activePeerId) || activeSessionPeerId || pendingIncomingPeerId;
          if (target && globalSignaling) {
            globalSignaling.publishTo(target, {
              type: "webrtc_candidate",
              session_id: currentSessionId,
              from_id: globalSignaling.myId,
              candidate: candJson
            });
          }
        }
      };

      this.pc.onconnectionstatechange = () => {
        const state = this.pc ? this.pc.connectionState : "closed";
        console.log(`[WebRTC] Connection state: ${state}`);
        if (state === "connected") {
          const stat = document.getElementById("stat-quality");
          if (stat) {
            stat.innerText = "WebRTC P2P (Live)";
            stat.style.color = "var(--success)";
          }
        }
        if (onStateChange) onStateChange(state);
      };

      this.pc.ondatachannel = (event) => {
        this.setupDataChannel(event.channel, onDataMessage || this.onDataMessage);
      };
    } catch (e) {
      console.warn("[WebRTC] PeerConnection init fallback:", e);
    }
  }

  createDataChannel(onDataMessage) {
    if (!this.pc) return;
    try {
      const handler = onDataMessage || this.onDataMessage;
      this.dataChannel = this.pc.createDataChannel("remotedesk-p2p", { ordered: true });
      this.setupDataChannel(this.dataChannel, handler);
    } catch (e) {
      console.warn("[WebRTC] DataChannel error:", e);
    }
  }

  setupDataChannel(channel, onDataMessage) {
    this.dataChannel = channel;
    this.dataChannel.onopen = () => {
      console.log("[WebRTC] Direct P2P DataChannel open!");
      const stat = document.getElementById("stat-quality");
      if (stat) {
        stat.innerText = "WebRTC P2P (Live)";
        stat.style.color = "var(--success)";
      }
      if (typeof requestRemoteKeyframe === "function") {
        requestRemoteKeyframe(activeSessionPeerId);
      }
    };
    this.dataChannel.onmessage = (event) => {
      const handler = onDataMessage || this.onDataMessage;
      if (handler) handler(event.data);
    };
  }

  async createOffer(onDataMessage) {
    if (!this.pc) return null;
    if (!this.dataChannel) {
      this.createDataChannel(onDataMessage || this.onDataMessage);
    }
    const offer = await this.pc.createOffer();
    await this.pc.setLocalDescription(offer);
    return JSON.stringify(offer);
  }

  async handleOffer(offerJson) {
    if (!this.pc) return null;
    const offer = typeof offerJson === "string" ? JSON.parse(offerJson) : offerJson;
    await this.pc.setRemoteDescription(new RTCSessionDescription(offer));
    await this.drainPendingCandidates();
    const answer = await this.pc.createAnswer();
    await this.pc.setLocalDescription(answer);
    return JSON.stringify(answer);
  }

  async handleAnswer(answerJson) {
    if (!this.pc) return;
    const answer = typeof answerJson === "string" ? JSON.parse(answerJson) : answerJson;
    await this.pc.setRemoteDescription(new RTCSessionDescription(answer));
    await this.drainPendingCandidates();
  }

  async addIceCandidate(candidateJson) {
    if (!this.pc) return;
    try {
      const candidate = typeof candidateJson === "string" ? JSON.parse(candidateJson) : candidateJson;
      if (!this.pc.remoteDescription || !this.pc.remoteDescription.type) {
        this.pendingCandidates.push(candidate);
        return;
      }
      if (candidate && candidate.candidate) {
        await this.pc.addIceCandidate(new RTCIceCandidate(candidate));
      }
    } catch (e) {
      console.warn("[WebRTC] addIceCandidate error:", e);
    }
  }

  async drainPendingCandidates() {
    while (this.pendingCandidates && this.pendingCandidates.length > 0) {
      const cand = this.pendingCandidates.shift();
      try {
        if (cand && cand.candidate) {
          await this.pc.addIceCandidate(new RTCIceCandidate(cand));
        }
      } catch (e) {
        console.warn("[WebRTC] drain candidate notice:", e);
      }
    }
  }

  send(data) {
    if (this.dataChannel && this.dataChannel.readyState === "open") {
      try {
        if (this.dataChannel.bufferedAmount > 1024 * 1024) {
          return false;
        }
        this.dataChannel.send(typeof data === "string" ? data : JSON.stringify(data));
        return true;
      } catch (e) {
        console.warn("[WebRTC] send error:", e);
        return false;
      }
    }
    return false;
  }

  close() {
    this.pendingCandidates = [];
    if (this.dataChannel) {
      try { this.dataChannel.close(); } catch (_) {}
      this.dataChannel = null;
    }
    if (this.pc) {
      try { this.pc.close(); } catch (_) {}
      this.pc = null;
    }
  }
}

// Remote & Interactive Cursor Synchronization
let currentRemoteCursor = "default";

function updateRemoteCursor(shape) {
  if (!shape) return;
  const validShapes = {
    "default": "default",
    "arrow": "default",
    "text": "text",
    "ibeam": "text",
    "pointer": "pointer",
    "hand": "pointer",
    "wait": "wait",
    "progress": "progress",
    "crosshair": "crosshair",
    "move": "move",
    "sizeall": "move",
    "ns-resize": "ns-resize",
    "ew-resize": "ew-resize",
    "nwse-resize": "nwse-resize",
    "nesw-resize": "nesw-resize",
    "not-allowed": "not-allowed",
    "help": "help"
  };
  const normalized = validShapes[String(shape).toLowerCase()] || "default";
  currentRemoteCursor = normalized;
  if (!activeCanvas) {
    activeCanvas = document.getElementById("remote-screen-canvas");
  }
  if (activeCanvas && activeCanvas.style.cursor !== normalized) {
    activeCanvas.style.cursor = normalized;
  }
}

// Third-party Global Cloud Signaling Client (Multi-Broker Resilient WebRTC Rendezvous)
class GlobalSignalingClient {
  constructor() {
    this.client = null;
    this.myId = null;
    this.connected = false;
    this.activePeerId = null;
    this.activePeers = new Map(); // cleanPeerId -> { sessionId, permissions, connectedAt }
    this.pendingPeers = new Map(); // sessionId -> { peerId, peerName, permissions }
    this.pendingIncomingFrom = null;
    this.pendingSessionId = null;
    this.connectTimeoutTimer = null;
    this.brokers = [
      "wss://broker.emqx.io:8084/mqtt",
      "wss://broker.hivemq.com:8884/mqtt",
      "wss://mqtt.eclipseprojects.io:443/mqtt",
      "wss://test.mosquitto.org:8081"
    ];
    this.currentBrokerIdx = 0;
  }

  broadcastToActivePeers(packet) {
    if (this.activePeers && this.activePeers.size > 0) {
      for (const peerId of this.activePeers.keys()) {
        this.publishTo(peerId, packet);
      }
    } else if (this.activePeerId) {
      this.publishTo(this.activePeerId, packet);
    }
  }

  init(deviceId) {
    if (!deviceId) return;
    const clean = deviceId.replace(/\s+/g, '');
    if (this.myId === clean && this.connected) return;
    this.myId = clean;
    this.connectCurrentBroker();
  }

  connectCurrentBroker() {
    const brokerUrl = this.brokers[this.currentBrokerIdx % this.brokers.length];
    const clientId = "rd_" + this.myId + "_" + Math.random().toString(16).substr(2, 6);

    try {
      if (typeof mqtt !== "undefined" && mqtt.connect) {
        if (this.client) {
          try { this.client.end(true); } catch (_) {}
        }

        console.log(`[GlobalSignaling] Connecting to rendezvous broker: ${brokerUrl}`);
        this.client = mqtt.connect(brokerUrl, {
          clientId: clientId,
          clean: true,
          connectTimeout: 8000,
          reconnectPeriod: 4000,
        });

        this.client.on("connect", () => {
          this.connected = true;
          console.log(`[GlobalSignaling] Connected to Cloud Rendezvous at ${brokerUrl}!`);
          const statusText = document.getElementById("agent-status-text");
          if (statusText) {
            statusText.innerText = "Network Ready • Global Rendezvous Active (No Geo Limits)";
          }
          const topic = `remotedesk/v1/${this.myId}`;
          this.client.subscribe(topic, (err) => {
            if (!err) console.log(`[GlobalSignaling] Subscribed to ${topic}`);
          });
        });

        this.client.on("message", (topic, payload) => {
          try {
            const data = JSON.parse(payload.toString());
            this.handleMessage(data);
          } catch (e) {
            console.warn("[GlobalSignaling] Parse error:", e);
          }
        });

        this.client.on("error", (err) => {
          console.warn(`[GlobalSignaling] Broker ${brokerUrl} notice:`, err);
          this.tryNextBroker();
        });

        this.client.on("close", () => {
          this.connected = false;
        });
      }
    } catch (e) {
      console.warn("[GlobalSignaling] Init error:", e);
      this.tryNextBroker();
    }
  }

  tryNextBroker() {
    if (this.connected) return;
    this.currentBrokerIdx = (this.currentBrokerIdx + 1) % this.brokers.length;
    setTimeout(() => {
      if (!this.connected && this.myId) {
        this.connectCurrentBroker();
      }
    }, 2500);
  }

  publishTo(targetId, messageObj) {
    if (!targetId || !this.client || !this.connected) return false;
    const cleanId = String(targetId).replace(/\s+/g, '');
    const topic = `remotedesk/v1/${cleanId}`;
    try {
      this.client.publish(topic, JSON.stringify(messageObj), { qos: 0 });
      return true;
    } catch (e) {
      return false;
    }
  }

  connectToPeer(targetId, pin, unattendedPwd, permissions) {
    if (!targetId) return false;
    const cleanTargetId = String(targetId).replace(/\s+/g, '');
    this.activePeerId = cleanTargetId;
    const sid = "sess_" + Math.random().toString(16).substr(2, 8);
    currentSessionId = sid;

    if (this.connectTimeoutTimer) clearTimeout(this.connectTimeoutTimer);
    this.connectTimeoutTimer = setTimeout(() => {
      if (!isSessionActive) {
        const msgElem = document.getElementById("connection-status-msg");
        msgElem.innerText = `Device ${targetId} did not respond. Verify the ID and that RemoteDesk Free is open on the target system.`;
        msgElem.style.color = "var(--error)";
      }
    }, 35000);

    return this.publishTo(cleanTargetId, {
      type: "connect_request",
      session_id: sid,
      from_id: this.myId,
      from_name: currentAppConfig ? currentAppConfig.device_alias : "Remote Client",
      pin: pin || null,
      unattended_password: unattendedPwd || null,
      permissions: permissions
    });
  }

  acceptConnection(sessionId, peerId, permissions) {
    const cleanPeerId = peerId ? String(peerId).replace(/\s+/g, '') : null;
    if (!cleanPeerId) {
      console.warn("[GlobalSignaling] Cannot accept connection without valid peerId");
      return;
    }
    sendIpcMessage("start_streaming", { permissions });
    this.publishTo(cleanPeerId, {
      type: "session_response",
      session_id: sessionId,
      from_id: this.myId,
      accepted: true,
      peer_id: this.myId,
      permissions
    });
    this.activePeerId = cleanPeerId;
    this.activePeers.set(cleanPeerId, { sessionId, permissions, connectedAt: Date.now() });
    this.pendingPeers.delete(sessionId);
    this.pendingIncomingFrom = null;
    setHostSessionActive(true, cleanPeerId);
  }

  declineConnection(sessionId, reason) {
    const peerId = this.pendingIncomingFrom || pendingIncomingPeerId;
    if (peerId) {
      this.publishTo(peerId, {
        type: "session_response",
        session_id: sessionId,
        from_id: this.myId,
        accepted: false,
        reason: reason || "Declined"
      });
      this.pendingPeers.delete(sessionId);
      this.pendingIncomingFrom = null;
    }
  }

  handleMessage(msg) {
    if (msg.from_id === this.myId) return; // Ignore echoes

    switch (msg.type) {
      case "connect_request": {
        currentSessionId = msg.session_id;
        this.pendingIncomingFrom = msg.from_id;
        this.pendingSessionId = msg.session_id;
        pendingIncomingPeerId = msg.from_id;
        this.pendingPeers.set(msg.session_id, {
          from_id: msg.from_id,
          from_name: msg.from_name,
          permissions: msg.permissions
        });

        // Check unattended password match
        if (currentAppConfig && currentAppConfig.unattended_access_enabled && msg.unattended_password) {
          console.log("[GlobalSignaling] Auto-accepting unattended session");
          this.acceptConnection(msg.session_id, msg.from_id, msg.permissions || {});
          return;
        }

        // Show Incoming Modal
        document.getElementById("incoming-peer-id").innerText = formatDeviceId(msg.from_id);
        document.getElementById("incoming-peer-name").innerText = msg.from_name || "Remote Client";
        document.getElementById("incoming-peer-ip").innerText = "Cloud P2P";
        document.getElementById("incoming-modal").style.display = "flex";
        break;
      }

      case "session_response": {
        if (this.connectTimeoutTimer) clearTimeout(this.connectTimeoutTimer);

        if (msg.accepted) {
          isSessionActive = true;
          currentSessionId = msg.session_id;
          this.activePeerId = msg.from_id;

          // Multi-session viewer tab registration
          addViewerSession(msg.from_id, msg.session_id);

          document.getElementById("session-viewer-overlay").style.display = "flex";
          document.getElementById("connection-status-msg").innerText = "";

          webrtcPeer.init((data) => {
            try {
              const packet = typeof data === "string" ? JSON.parse(data) : data;
              if (packet.channel === "CursorShape" || packet.type === "cursor_shape") {
                const s = (packet.data && packet.data.shape) || packet.shape || "default";
                updateRemoteCursor(s);
                return;
              }
              if (packet.channel === "Chat" || packet.type === "Chat") {
                const c = packet.data || packet;
                appendChatMessage(c.sender || "Remote Client", c.message || "", false);
                openChatDrawer();
                return;
              }
              if (packet.type === "peer_packet" || packet.channel) {
                handlePeerSessionPacket(packet);
                return;
              }
              renderFrame(packet);
            } catch (_) {}
          });

          webrtcPeer.createDataChannel((data) => {
            try {
              const packet = typeof data === "string" ? JSON.parse(data) : data;
              if (packet.channel === "CursorShape" || packet.type === "cursor_shape") {
                const s = (packet.data && packet.data.shape) || packet.shape || "default";
                updateRemoteCursor(s);
                return;
              }
              if (packet.channel === "Chat" || packet.type === "Chat") {
                const c = packet.data || packet;
                appendChatMessage(c.sender || "Remote Client", c.message || "", false);
                openChatDrawer();
                return;
              }
              if (packet.type === "peer_packet" || packet.channel) {
                handlePeerSessionPacket(packet);
                return;
              }
              renderFrame(packet);
            } catch (_) {}
          });

          webrtcPeer.createOffer().then((offer) => {
            if (offer) {
              this.publishTo(msg.from_id, {
                type: "webrtc_offer",
                session_id: msg.session_id,
                from_id: this.myId,
                sdp: offer
              });
            }
          });
        } else {
          const msgElem = document.getElementById("connection-status-msg");
          if (msgElem) {
            msgElem.innerText = `Connection request was declined by remote host (${msg.reason || "Declined"})`;
            msgElem.style.color = "var(--error)";
          }
          const multiStatus = document.getElementById("multi-connect-status");
          if (multiStatus) {
            multiStatus.innerText = `Connection declined: ${msg.reason || "Declined"}`;
            multiStatus.style.color = "var(--error)";
          }
        }
        break;
      }

      case "webrtc_offer": {
        this.activePeerId = msg.from_id;
        webrtcPeer.init((data) => {
          try {
            const packet = typeof data === "string" ? JSON.parse(data) : data;
            if (packet.channel === "CursorShape" || packet.type === "cursor_shape") {
              const s = (packet.data && packet.data.shape) || packet.shape || "default";
              updateRemoteCursor(s);
              return;
            }
            if (packet.channel === "Chat" || packet.type === "Chat") {
              const c = packet.data || packet;
              appendChatMessage(c.sender || "Remote Client", c.message || "", false);
              openChatDrawer();
              return;
            }
            if (packet.channel === "RequestKeyframe" || packet.type === "request_keyframe") {
              sendIpcMessage("request_keyframe", {});
              return;
            }
            if (packet.type === "peer_packet" || packet.channel) {
              handlePeerSessionPacket(packet);
              return;
            }
            if (packet.type === "MouseMove" || packet.type === "MouseDown" || packet.type === "MouseUp" || packet.type === "KeyDown" || packet.type === "KeyUp" || packet.type === "MouseWheel" || packet.type === "KeyChar" || packet.type === "SpecialAction" || packet.type === "SystemAction") {
              if (packet.seq) {
                if (processedHostInputSeqs.has(packet.seq)) return;
                processedHostInputSeqs.add(packet.seq);
                if (processedHostInputSeqs.size > 2000) {
                  const first = processedHostInputSeqs.values().next().value;
                  processedHostInputSeqs.delete(first);
                }
              }
              sendIpcMessage("send_input", packet);
            }
          } catch (_) {}
        });

        webrtcPeer.handleOffer(msg.sdp).then((answer) => {
          if (answer) {
            this.publishTo(msg.from_id, {
              type: "webrtc_answer",
              session_id: msg.session_id,
              from_id: this.myId,
              sdp: answer
            });
          }
        });
        break;
      }

      case "webrtc_answer": {
        webrtcPeer.handleAnswer(msg.sdp);
        break;
      }

      case "webrtc_candidate": {
        webrtcPeer.addIceCandidate(msg.candidate);
        break;
      }

      case "peer_input": {
        if (msg.input) {
          if (msg.input.channel === "RequestKeyframe" || msg.input.type === "request_keyframe") {
            sendIpcMessage("request_keyframe", {});
          } else {
            if (msg.input.seq) {
              if (processedHostInputSeqs.has(msg.input.seq)) break;
              processedHostInputSeqs.add(msg.input.seq);
              if (processedHostInputSeqs.size > 2000) {
                const first = processedHostInputSeqs.values().next().value;
                processedHostInputSeqs.delete(first);
              }
            }
            sendIpcMessage("send_input", msg.input);
          }
        }
        break;
      }

      case "request_keyframe":
      case "RequestKeyframe": {
        sendIpcMessage("request_keyframe", {});
        break;
      }

      case "cursor_shape": {
        if (msg.shape) {
          updateRemoteCursor(msg.shape);
        }
        break;
      }

      case "screen_frame_relay": {
        if (msg.payload) {
          renderFrame(msg.payload);
        }
        break;
      }

      case "peer_chat": {
        appendChatMessage(msg.sender || msg.from_id || "Remote Client", msg.message || "", false);
        openChatDrawer();
        break;
      }

      case "peer_packet": {
        handlePeerSessionPacket(msg);
        break;
      }

      case "disconnect": {
        // Handle host side multi-client cleanup
        if (this.activePeers.has(msg.from_id)) {
          this.activePeers.delete(msg.from_id);
          if (this.activePeers.size === 0) {
            webrtcPeer.close();
            setHostSessionActive(false);
          } else {
            setHostSessionActive(true);
          }
        }
        // Handle viewer side multi-session cleanup
        if (activeSessions.has(msg.from_id)) {
          closeSessionTab(msg.from_id);
        }
        break;
      }
    }
  }
}

const webrtcPeer = new WebRtcPeerClient();
const globalSignaling = new GlobalSignalingClient();

document.addEventListener("DOMContentLoaded", () => {
  initTabs();
  initTheme();
  initCanvas();
  renderRecentDevices(allDevicesList);
  renderSavedDevices(allDevicesList);
  connectToBackendIpc();
  setupEventListeners();
});

// Tab Navigation
function initTabs() {
  const tabs = document.querySelectorAll(".sidebar-item, .nav-tab");
  tabs.forEach((tab) => {
    tab.addEventListener("click", () => {
      tabs.forEach((t) => t.classList.remove("active"));
      tab.classList.add("active");

      const targetTab = tab.getAttribute("data-tab");
      document.querySelectorAll(".tab-pane").forEach((pane) => {
        pane.classList.remove("active");
      });
      const activePane = document.getElementById(`tab-${targetTab}`);
      if (activePane) activePane.classList.add("active");

      // Update Top Nav Breadcrumb active title
      const breadcrumb = document.getElementById("active-breadcrumb-title");
      if (breadcrumb) {
        breadcrumb.innerText = tab.innerText.trim();
      }

      if (targetTab === "filemanager") {
        refreshLocalFiles();
      }
    });
  });
}

// Light / Dark Theme Toggle
function initTheme() {
  const saved = localStorage.getItem("remotedesk-theme") || "light";
  document.documentElement.setAttribute("data-theme", saved);

  const toggleBtn = document.getElementById("theme-toggle");
  if (toggleBtn) {
    toggleBtn.addEventListener("click", () => {
      const current = document.documentElement.getAttribute("data-theme");
      const next = current === "dark" ? "light" : "dark";
      document.documentElement.setAttribute("data-theme", next);
      localStorage.setItem("remotedesk-theme", next);
    });
  }
}

let g_inputSeq = 0;
const processedHostInputSeqs = new Set();
let isKeyboardShortcutsPassthrough = true;

function toggleKeyboardPassthrough() {
  isKeyboardShortcutsPassthrough = !isKeyboardShortcutsPassthrough;
  const badge = document.getElementById("keyboard-passthrough-badge");
  if (badge) {
    if (isKeyboardShortcutsPassthrough) {
      badge.innerText = "⌨️ Shortcuts: Remote";
      badge.style.background = "rgba(34, 197, 94, 0.15)";
      badge.style.color = "#4ade80";
      badge.style.borderColor = "rgba(34, 197, 94, 0.3)";
      badge.title = "All keyboard shortcuts (Win+R, Alt+Tab, TaskMgr, etc.) are active on the remote computer";
    } else {
      badge.innerText = "⌨️ Shortcuts: Local";
      badge.style.background = "rgba(148, 163, 184, 0.15)";
      badge.style.color = "#94a3b8";
      badge.style.borderColor = "rgba(148, 163, 184, 0.3)";
      badge.title = "Keyboard shortcuts stay on your local computer";
    }
  }
}
window.toggleKeyboardPassthrough = toggleKeyboardPassthrough;

function dispatchInputPacket(packet) {
  if (!packet.seq) {
    packet.seq = ++g_inputSeq;
  }
  packet.ts = Date.now();

  const targetPeer = activeSessionPeerId || (globalSignaling && globalSignaling.activePeerId);
  let sent = false;
  if (webrtcPeer && webrtcPeer.dataChannel && webrtcPeer.dataChannel.readyState === "open") {
    sent = webrtcPeer.send(packet);
  }
  // Fallback to cloud rendezvous broker only if direct P2P data channel is unavailable
  if (!sent && globalSignaling && targetPeer) {
    globalSignaling.publishTo(targetPeer, {
      type: "peer_input",
      session_id: currentSessionId,
      from_id: globalSignaling.myId,
      input: packet
    });
  }
}

function isTargetingTextInput(target) {
  if (!target) return false;
  const tag = target.tagName ? target.tagName.toLowerCase() : "";
  if (tag === "input" || tag === "textarea" || tag === "select") {
    return true;
  }
  return false;
}

function handleRemoteKeyDown(e) {
  if (!isSessionActive) return;
  const overlay = document.getElementById("session-viewer-overlay");
  if (!overlay || overlay.style.display === "none") return;
  if (isTargetingTextInput(e.target)) return;

  if (e._rdHandled) return;
  e._rdHandled = true;

  const isMeta = e.key === "Meta" || e.code === "MetaLeft" || e.code === "MetaRight" || e.keyCode === 91 || e.keyCode === 92;

  if (isKeyboardShortcutsPassthrough) {
    // 1. Win + Key shortcuts
    if (e.metaKey || isMeta) {
      if (e.key === "r" || e.key === "R" || e.code === "KeyR" || e.keyCode === 82) {
        e.preventDefault();
        e.stopPropagation();
        triggerSystemAction("run_dialog");
        return;
      }
      if (e.key === "e" || e.key === "E" || e.code === "KeyE" || e.keyCode === 69) {
        e.preventDefault();
        e.stopPropagation();
        triggerSystemAction("explorer");
        return;
      }
      if (e.key === "d" || e.key === "D" || e.code === "KeyD" || e.keyCode === 68) {
        e.preventDefault();
        e.stopPropagation();
        triggerSystemAction("show_desktop");
        return;
      }
      if (e.key === "x" || e.key === "X" || e.code === "KeyX" || e.keyCode === 88) {
        e.preventDefault();
        e.stopPropagation();
        triggerSystemAction("quick_link_menu");
        return;
      }
      if (e.key === "i" || e.key === "I" || e.code === "KeyI" || e.keyCode === 73) {
        e.preventDefault();
        e.stopPropagation();
        triggerSystemAction("settings");
        return;
      }
      if (e.key === "l" || e.key === "L" || e.code === "KeyL" || e.keyCode === 76) {
        e.preventDefault();
        e.stopPropagation();
        triggerSystemAction("lock_workstation");
        return;
      }
      if (e.key === "s" || e.key === "S" || e.code === "KeyS" || e.keyCode === 83) {
        e.preventDefault();
        e.stopPropagation();
        triggerSystemAction("search");
        return;
      }
    }

    // 2. Ctrl + Shift + Esc -> Task Manager
    if (e.ctrlKey && e.shiftKey && (e.key === "Escape" || e.keyCode === 27)) {
      e.preventDefault();
      e.stopPropagation();
      triggerSystemAction("task_manager");
      return;
    }

    // 3. Alt + Tab -> Window Switch
    if (e.altKey && (e.key === "Tab" || e.keyCode === 9)) {
      e.preventDefault();
      e.stopPropagation();
      triggerSystemAction("alt_tab");
      return;
    }

    // 4. Alt + F4 -> Close Remote Window
    if (e.altKey && (e.key === "F4" || e.keyCode === 115)) {
      e.preventDefault();
      e.stopPropagation();
      triggerSystemAction("alt_f4");
      return;
    }
  }

  // Prevent local browser accelerators (Ctrl+W, Ctrl+R, Ctrl+T, Ctrl+N, Ctrl+F, F5, F11)
  if ((e.ctrlKey && (e.key === "w" || e.key === "r" || e.key === "t" || e.key === "n" || e.key === "f")) || e.key === "F5") {
    e.preventDefault();
    e.stopPropagation();
  } else {
    e.preventDefault();
  }

  // Automatically switch to text cursor indicator when entering characters
  if (!e.ctrlKey && !e.altKey && !e.metaKey && (e.key.length === 1 || e.key === "Backspace" || e.key === "Enter" || e.key === "Tab")) {
    updateRemoteCursor("text");
  }

  // Send hardware-faithful KeyDown (no unicode injection, preventing double characters)
  dispatchInputPacket({
    type: "KeyDown",
    key_code: e.keyCode,
    code: e.code,
    key: e.key,
    modifiers: {
      shift: e.shiftKey,
      ctrl: e.ctrlKey,
      alt: e.altKey,
      meta: e.metaKey || isMeta,
    },
  });
}

function handleRemoteKeyUp(e) {
  if (!isSessionActive) return;
  const overlay = document.getElementById("session-viewer-overlay");
  if (!overlay || overlay.style.display === "none") return;
  if (isTargetingTextInput(e.target)) return;

  if (e._rdHandled) return;
  e._rdHandled = true;

  const isMeta = e.key === "Meta" || e.code === "MetaLeft" || e.code === "MetaRight" || e.keyCode === 91 || e.keyCode === 92;
  e.preventDefault();

  dispatchInputPacket({
    type: "KeyUp",
    key_code: e.keyCode,
    code: e.code,
    key: e.key,
    modifiers: {
      shift: e.shiftKey,
      ctrl: e.ctrlKey,
      alt: e.altKey,
      meta: e.metaKey || isMeta,
    },
  });
}

// Canvas Initialization
function initCanvas() {
  activeCanvas = document.getElementById("remote-screen-canvas");
  if (activeCanvas) {
    canvasCtx = activeCanvas.getContext("2d");
    activeCanvas.width = 1920;
    activeCanvas.height = 1080;
    activeCanvas.setAttribute("tabindex", "0");
    activeCanvas.style.cursor = currentRemoteCursor || "default";
  }

  activeCanvas.addEventListener("mouseenter", () => {
    if (activeCanvas) activeCanvas.style.cursor = currentRemoteCursor || "default";
  });

  // Global window listeners for capturing keyboard shortcuts faithfully
  window.removeEventListener("keydown", handleRemoteKeyDown, { capture: true });
  window.removeEventListener("keyup", handleRemoteKeyUp, { capture: true });
  window.addEventListener("keydown", handleRemoteKeyDown, { capture: true });
  window.addEventListener("keyup", handleRemoteKeyUp, { capture: true });

  // Mouse Input Handling with 35ms throttling to prevent network choking
  let lastMouseMoveTime = 0;
  activeCanvas.addEventListener("mousemove", (e) => {
    if (!isSessionActive) return;
    const now = performance.now();
    if (now - lastMouseMoveTime < 35) return; // ~28 events/sec max
    lastMouseMoveTime = now;
    const pos = getNormalizedCoords(e);
    dispatchInputPacket({
      type: "MouseMove",
      x: pos.x,
      y: pos.y,
      screen_w: activeCanvas.width,
      screen_h: activeCanvas.height,
      monitor_idx: 0,
    });
  });

  activeCanvas.addEventListener("mousedown", (e) => {
    if (!isSessionActive) return;
    activeCanvas.focus();
    const pos = getNormalizedCoords(e);
    const btn = e.button === 0 ? "Left" : e.button === 2 ? "Right" : "Middle";
    dispatchInputPacket({
      type: "MouseDown",
      button: btn,
      x: pos.x,
      y: pos.y,
      screen_w: activeCanvas.width,
      screen_h: activeCanvas.height,
    });
  });

  activeCanvas.addEventListener("mouseup", (e) => {
    if (!isSessionActive) return;
    const pos = getNormalizedCoords(e);
    const btn = e.button === 0 ? "Left" : e.button === 2 ? "Right" : "Middle";
    dispatchInputPacket({
      type: "MouseUp",
      button: btn,
      x: pos.x,
      y: pos.y,
      screen_w: activeCanvas.width,
      screen_h: activeCanvas.height,
    });
  });

  activeCanvas.addEventListener("wheel", (e) => {
    if (!isSessionActive) return;
    e.preventDefault();
    const deltaY = e.deltaY < 0 ? 1 : -1;
    dispatchInputPacket({
      type: "MouseWheel",
      delta_x: 0,
      delta_y: deltaY,
    });
  });

  activeCanvas.addEventListener("contextmenu", (e) => e.preventDefault());

  // Window-level mouseup release to prevent stuck mouse drag when releasing outside canvas
  window.addEventListener("mouseup", (e) => {
    if (!isSessionActive) return;
    const overlay = document.getElementById("session-viewer-overlay");
    if (!overlay || overlay.style.display === "none") return;
    const pos = getNormalizedCoords(e);
    const btn = e.button === 0 ? "Left" : e.button === 2 ? "Right" : "Middle";
    dispatchInputPacket({
      type: "MouseUp",
      button: btn,
      x: pos.x,
      y: pos.y,
      screen_w: activeCanvas ? activeCanvas.width : 1920,
      screen_h: activeCanvas ? activeCanvas.height : 1080,
    });
  });

  window.addEventListener("resize", resizeCanvasToFit);
  if (window.ResizeObserver) {
    const ro = new ResizeObserver(() => {
      resizeCanvasToFit();
    });
    const container = document.getElementById("canvas-container");
    if (container) ro.observe(container);
    const overlay = document.getElementById("session-viewer-overlay");
    if (overlay) ro.observe(overlay);
  }
  setTimeout(resizeCanvasToFit, 50);
}

function resizeCanvasToFit() {
  const container = document.getElementById("canvas-container");
  const canvas = document.getElementById("remote-screen-canvas");
  if (!container || !canvas) return;

  const selector = document.getElementById("scaling-selector");
  const mode = selector ? selector.value : "fit";

  const containerW = container.clientWidth;
  const containerH = container.clientHeight;

  if (containerW <= 0 || containerH <= 0) return;

  const nativeW = canvas.width || 1920;
  const nativeH = canvas.height || 1080;

  if (mode === "original") {
    canvas.style.maxWidth = "none";
    canvas.style.maxHeight = "none";
    canvas.style.width = `${nativeW}px`;
    canvas.style.height = `${nativeH}px`;
    container.style.overflow = "auto";
  } else if (mode === "stretch") {
    canvas.style.maxWidth = "100%";
    canvas.style.maxHeight = "100%";
    canvas.style.width = "100%";
    canvas.style.height = "100%";
    container.style.overflow = "hidden";
  } else {
    // "fit" (Default Proportional Aspect Ratio Scaling)
    container.style.overflow = "hidden";
    canvas.style.maxWidth = "100%";
    canvas.style.maxHeight = "100%";

    const aspect = nativeW / nativeH;
    let targetW = containerW;
    let targetH = Math.round(containerW / aspect);

    if (targetH > containerH) {
      targetH = containerH;
      targetW = Math.round(containerH * aspect);
    }

    canvas.style.width = `${targetW}px`;
    canvas.style.height = `${targetH}px`;
  }
}
window.resizeCanvasToFit = resizeCanvasToFit;

function getNormalizedCoords(e) {
  if (!activeCanvas) {
    activeCanvas = document.getElementById("remote-screen-canvas");
  }
  if (!activeCanvas) return { x: 0, y: 0 };
  const rect = activeCanvas.getBoundingClientRect();
  if (!rect.width || !rect.height) return { x: 0, y: 0 };

  const relX = Math.max(0, Math.min(e.clientX - rect.left, rect.width));
  const relY = Math.max(0, Math.min(e.clientY - rect.top, rect.height));

  const scaleX = activeCanvas.width / rect.width;
  const scaleY = activeCanvas.height / rect.height;

  const x = Math.round(relX * scaleX);
  const y = Math.round(relY * scaleY);
  return { x: Math.max(0, Math.min(x, activeCanvas.width)), y: Math.max(0, Math.min(y, activeCanvas.height)) };
}

// Connect to Local Rust Backend IPC WebSocket
function connectToBackendIpc() {
  const loc = window.location;
  const ipcUrl = `ws://${loc.hostname || "127.0.0.1"}:${loc.port || "9090"}/ipc`;

  ipcSocket = new WebSocket(ipcUrl);

  ipcSocket.onopen = () => {
    console.log("[IPC] Connected to RemoteDesk local backend engine.");
    document.getElementById("agent-status-dot").className = "status-dot online";
    document.getElementById("agent-status-text").innerText = "Network Ready";
    sendIpcMessage("get_state", {});
    sendIpcMessage("get_user_dirs", {});
  };

  ipcSocket.onmessage = (event) => {
    try {
      const data = JSON.parse(event.data);
      handleIpcMessage(data);
    } catch (e) {
      console.error("[IPC] Failed to parse message:", e);
    }
  };

  ipcSocket.onclose = () => {
    console.warn("[IPC] Disconnected from local engine. Reconnecting in 2s...");
    document.getElementById("agent-status-dot").className = "status-dot connecting";
    document.getElementById("agent-status-text").innerText = "Connecting...";
    setTimeout(connectToBackendIpc, 2000);
  };
}

function sendIpcMessage(type, payload) {
  if (ipcSocket && ipcSocket.readyState === WebSocket.OPEN) {
    ipcSocket.send(JSON.stringify({ type, payload }));
  }
}

// Handle Incoming IPC events from Rust backend
function handleIpcMessage(msg) {
  switch (msg.type) {
    case "state_update": {
      const s = msg.payload;
      currentAppConfig = s.config;

      // Update Desk ID display
      if (s.device_id) {
        document.getElementById("my-desk-id").innerText = s.formatted_id || s.device_id;
        globalSignaling.init(s.device_id);
      }
      if (s.lan_ip) {
        document.getElementById("lan-ip-badge").innerText = `Network: ${s.lan_ip}:8088 (Auto-Signaling Active)`;
      }
      if (s.pin) {
        document.getElementById("session-pin-text").innerText = s.pin;
      }

      // Populate Recent Devices & Address Book
      if (s.recent_devices) {
        allDevicesList = s.recent_devices;
        localStorage.setItem("remotedesk_devices", JSON.stringify(allDevicesList));
        renderRecentDevices(allDevicesList);
        renderSavedDevices(allDevicesList);
      }

      // Populate Audit Logs
      renderAuditLogs(s.audit_logs || []);

      // Update Unattended Toggle
      if (currentAppConfig) {
        document.getElementById("unattended-enabled-toggle").checked = currentAppConfig.unattended_access_enabled;
        document.getElementById("unattended-config-panel").style.display = currentAppConfig.unattended_access_enabled ? "block" : "none";
        document.getElementById("setting-signaling-server").value = currentAppConfig.signaling_server;
        document.getElementById("setting-device-alias").value = currentAppConfig.device_alias;
      }
      break;
    }

    case "recent_devices_updated": {
      if (msg.payload && msg.payload.devices) {
        allDevicesList = msg.payload.devices;
        localStorage.setItem("remotedesk_devices", JSON.stringify(allDevicesList));
        renderRecentDevices(allDevicesList);
        renderSavedDevices(allDevicesList);
      }
      break;
    }

    case "incoming_session": {
      const req = msg.payload;
      currentSessionId = req.session_id;
      pendingIncomingPeerId = req.peer_id;
      document.getElementById("incoming-peer-id").innerText = req.peer_id;
      document.getElementById("incoming-peer-name").innerText = req.peer_name;
      document.getElementById("incoming-peer-ip").innerText = req.peer_ip;
      document.getElementById("incoming-modal").style.display = "flex";
      break;
    }

    case "session_started": {
      isSessionActive = true;
      currentSessionId = msg.payload.session_id;
      document.getElementById("session-peer-title").innerText = `Connected to: ${msg.payload.peer_id || "Remote Peer"}`;
      document.getElementById("session-viewer-overlay").style.display = "flex";
      document.getElementById("connection-status-msg").innerText = "";
      setTimeout(() => {
        if (activeCanvas) activeCanvas.focus();
      }, 150);
      webrtcPeer.init((data) => {
        try {
          const packet = typeof data === "string" ? JSON.parse(data) : data;
          renderFrame(packet);
        } catch (_) {}
      });
      webrtcPeer.createDataChannel();
      webrtcPeer.createOffer().then((offer) => {
        if (offer) {
          sendIpcMessage("send_webrtc_offer", {
            session_id: currentSessionId,
            sdp: offer
          });
        }
      });
      break;
    }

    case "peer_ice_candidate": {
      if (msg.payload && msg.payload.candidate) {
        webrtcPeer.addIceCandidate(msg.payload.candidate);
      }
      break;
    }

    case "peer_offer": {
      if (msg.payload && msg.payload.sdp) {
        webrtcPeer.handleOffer(msg.payload.sdp).then((answer) => {
          if (answer) {
            sendIpcMessage("send_webrtc_answer", {
              session_id: currentSessionId,
              sdp: answer
            });
          }
        });
      }
      break;
    }

    case "peer_answer": {
      if (msg.payload && msg.payload.sdp) {
        webrtcPeer.handleAnswer(msg.payload.sdp);
      }
      break;
    }

    case "session_ended": {
      isSessionActive = false;
      webrtcPeer.close();
      setHostSessionActive(false);
      document.getElementById("session-viewer-overlay").style.display = "none";
      document.getElementById("connection-status-msg").innerText = `Session ended: ${msg.payload.reason || "Disconnected"}`;
      break;
    }

    case "cursor_shape": {
      if (msg.payload && msg.payload.shape) {
        updateRemoteCursor(msg.payload.shape);
        // Relay cursor shape change to all connected viewers
        if (webrtcPeer && webrtcPeer.dataChannel && webrtcPeer.dataChannel.readyState === "open") {
          webrtcPeer.send({
            channel: "CursorShape",
            type: "cursor_shape",
            shape: msg.payload.shape
          });
        } else if (globalSignaling) {
          globalSignaling.broadcastToActivePeers({
            channel: "CursorShape",
            type: "cursor_shape",
            shape: msg.payload.shape
          });
        }
      }
      break;
    }

    case "screen_frame": {
      const isRtcOpen = webrtcPeer && webrtcPeer.dataChannel && webrtcPeer.dataChannel.readyState === "open";
      let sentRtc = false;
      if (isRtcOpen) {
        // Direct peer-to-peer WebRTC stream (sub-20ms ultra low latency)
        sentRtc = webrtcPeer.send(msg.payload);
      }
      if (!sentRtc && globalSignaling && (globalSignaling.activePeers.size > 0 || globalSignaling.activePeerId)) {
        // Resilient Global Mesh Relay fallback throttled to ~11-12 FPS to prevent MQTT broker queue congestion
        const now = performance.now();
        if (now - lastCloudFrameTime >= 85) {
          lastCloudFrameTime = now;
          if (globalSignaling.activePeers.size > 0) {
            for (const peerId of globalSignaling.activePeers.keys()) {
              globalSignaling.publishTo(peerId, {
                type: "screen_frame_relay",
                payload: msg.payload
              });
            }
          } else if (globalSignaling.activePeerId) {
            globalSignaling.publishTo(globalSignaling.activePeerId, {
              type: "screen_frame_relay",
              payload: msg.payload
            });
          }
        }
      }
      break;
    }

    case "chat_received": {
      appendChatMessage(msg.payload.sender, msg.payload.message, false);
      break;
    }

    case "error": {
      if (globalSignaling && globalSignaling.connected && globalSignaling.connectTimeoutTimer) {
        console.log("[Signaling] Local peer not found; continuing rendezvous via global cloud broker...");
        break;
      }
      const msgElem = document.getElementById("connection-status-msg");
      if (msgElem) {
        msgElem.innerText = `Error: ${msg.payload.message}`;
        msgElem.style.color = "var(--error)";
      }
      break;
    }

    case "local_files": {
      if (lastFileListRequester) {
        globalSignaling.publishTo(lastFileListRequester, {
          type: "peer_packet",
          channel: "FileListResponse",
          session_id: currentSessionId,
          from_id: globalSignaling.myId,
          data: {
            path: msg.payload.path,
            entries: msg.payload.entries
          }
        });
        lastFileListRequester = null;
      }
      renderLocalFiles(msg.payload.path, msg.payload.entries);
      break;
    }

    case "remote_files": {
      renderRemoteFiles(msg.payload.path, msg.payload.entries);
      break;
    }

    case "user_dirs": {
      if (msg.payload) {
        userDirs = msg.payload;
        if (!currentLocalPath || currentLocalPath === "C:\\") {
          currentLocalPath = userDirs.downloads || "C:\\";
        }
        document.querySelectorAll("#local-path, #sfm-local-path").forEach((inp) => inp.value = currentLocalPath);
        refreshLocalFiles();
      }
      break;
    }

    case "file_data_read": {
      const p = msg.payload;
      if (lastFileDownloadRequester) {
        // Host serving a download request to remote client
        globalSignaling.publishTo(lastFileDownloadRequester, {
          type: "peer_packet",
          channel: "FileTransferDownloadRes",
          session_id: currentSessionId,
          from_id: globalSignaling.myId,
          data: {
            filename: p.filename,
            size: p.size,
            data_base64: p.data_base64
          }
        });
        lastFileDownloadRequester = null;
      } else {
        // Viewer uploading local file to remote host
        const targetPeer = activeSessionPeerId || globalSignaling.activePeerId;
        if (targetPeer) {
          updateTransferProgress(50, `Uploading ${p.filename} (${formatBytes(p.size)}) to remote host...`);
          const packet = {
            type: "peer_packet",
            channel: "FileTransferUpload",
            session_id: currentSessionId,
            from_id: globalSignaling.myId,
            data: {
              filename: p.filename,
              destination_dir: currentRemotePath,
              data_base64: p.data_base64,
              size: p.size
            }
          };
          if (!webrtcPeer.send(packet)) {
            globalSignaling.publishTo(targetPeer, packet);
          }
          updateTransferProgress(85, `Transferring data packets to remote host...`);
        }
      }
      break;
    }

    case "file_saved": {
      const p = msg.payload;
      updateTransferProgress(100, `File saved: ${p.filename} (${formatBytes(p.size)}) to ${p.saved_path}`);
      refreshLocalFiles();
      break;
    }

    case "file_error": {
      updateTransferProgress(0, `File error: ${msg.payload.message}`);
      alert(`File operation error: ${msg.payload.message}`);
      break;
    }
  }
}

// Frame Renderer (Offscreen ImageBitmap GPU Pipeline for 60 FPS Low-Latency Display)
let isDecodingFrame = false;
let nextPendingFrame = null;
let currentRenderBitmap = null;

// Fast binary base64 decode to Uint8Array (zero intermediate DOM string allocation)
function base64ToUint8Array(base64) {
  const binaryString = atob(base64);
  const len = binaryString.length;
  const bytes = new Uint8Array(len);
  for (let i = 0; i < len; i++) {
    bytes[i] = binaryString.charCodeAt(i);
  }
  return bytes;
}

function renderFrame(frame) {
  if (!frame) return;

  // Cache in active session for instant restore upon unminimizing or switching tabs
  if (activeSessionPeerId && activeSessions.has(activeSessionPeerId)) {
    activeSessions.get(activeSessionPeerId).lastFrame = frame;
  }

  if (isDecodingFrame) {
    nextPendingFrame = frame;
    return;
  }
  processAndDrawFrame(frame);
}

async function processAndDrawFrame(frame) {
  if (!activeCanvas) {
    activeCanvas = document.getElementById("remote-screen-canvas");
  }
  if (!canvasCtx && activeCanvas) {
    canvasCtx = activeCanvas.getContext("2d", { alpha: false, desynchronized: true });
  }
  if (!canvasCtx || !frame) return;

  const cursorShape = frame.cursor_shape || (frame.data && frame.data.cursor_shape) || (frame.payload && frame.payload.cursor_shape);
  if (cursorShape) {
    updateRemoteCursor(cursorShape);
  }

  const payloadBase64 = frame.payload_base64 ||
    (frame.data && frame.data.payload_base64) ||
    (frame.payload && frame.payload.payload_base64) ||
    (frame.data && frame.data.data && frame.data.data.payload_base64);
  if (!payloadBase64) return;

  const screenW = frame.screen_w || (frame.data && frame.data.screen_w) || (frame.payload && frame.payload.screen_w) || activeCanvas.width;
  const screenH = frame.screen_h || (frame.data && frame.data.screen_h) || (frame.payload && frame.payload.screen_h) || activeCanvas.height;

  isDecodingFrame = true;
  try {
    const bytes = base64ToUint8Array(payloadBase64);
    const blob = new Blob([bytes], { type: "image/jpeg" });
    const bitmap = await createImageBitmap(blob);

    if (activeCanvas.width !== screenW || activeCanvas.height !== screenH) {
      activeCanvas.width = screenW;
      activeCanvas.height = screenH;
      resizeCanvasToFit();
    }

    // Direct GPU raster blit (takes < 0.2ms)
    canvasCtx.drawImage(bitmap, 0, 0, screenW, screenH);
    if (!activeCanvas.style.width) {
      resizeCanvasToFit();
    }

    // Cache latest bitmap for instant redraw on minimize/restore
    if (currentRenderBitmap && currentRenderBitmap !== bitmap) {
      try { currentRenderBitmap.close(); } catch (_) {}
    }
    currentRenderBitmap = bitmap;

    if (activeSessionPeerId && activeSessions.has(activeSessionPeerId)) {
      activeSessions.get(activeSessionPeerId).lastBitmap = bitmap;
    }

    // Dynamic FPS Calculation
    frameCount++;
    const now = performance.now();
    if (now - lastFpsTime >= 1000) {
      const fps = Math.round((frameCount * 1000) / (now - lastFpsTime));
      const fpsElem = document.getElementById("stat-fps");
      if (fpsElem) {
        if (fps >= 3) {
          fpsElem.innerText = `${fps} FPS`;
          fpsElem.style.color = "var(--success)";
        } else {
          fpsElem.innerText = `Active • Idle`;
          fpsElem.style.color = "var(--accent)";
        }
      }
      frameCount = 0;
      lastFpsTime = now;
    }

    // Dynamic Quality Display
    const statQuality = document.getElementById("stat-quality");
    if (statQuality) {
      const isRtc = webrtcPeer && webrtcPeer.dataChannel && webrtcPeer.dataChannel.readyState === "open";
      statQuality.innerText = `${isRtc ? "WebRTC P2P" : "Global Mesh"} • ${screenW}x${screenH}`;
    }

    // Dynamic Latency Display
    const frameTs = frame.timestamp_ms || (frame.data && frame.data.timestamp_ms);
    if (frameTs) {
      const diff = Math.max(1, Math.min(999, Date.now() - frameTs));
      const latElem = document.getElementById("stat-latency");
      if (latElem) {
        latElem.innerText = `${diff} ms`;
      }
    }
  } catch (err) {
    // Fallback if createImageBitmap fails
    try {
      const img = new Image();
      img.onload = () => {
        if (activeCanvas.width !== screenW || activeCanvas.height !== screenH) {
          activeCanvas.width = screenW;
          activeCanvas.height = screenH;
          resizeCanvasToFit();
        }
        canvasCtx.drawImage(img, 0, 0, screenW, screenH);
        if (!activeCanvas.style.width) {
          resizeCanvasToFit();
        }
      };
      img.src = `data:image/jpeg;base64,${payloadBase64}`;
    } catch (_) {}
  } finally {
    isDecodingFrame = false;
    if (nextPendingFrame) {
      const next = nextPendingFrame;
      nextPendingFrame = null;
      processAndDrawFrame(next);
    }
  }
}

// Instant Remote Desktop Screen Refresh & Keyframe Synchronization
function requestRemoteKeyframe(peerId) {
  const target = peerId || activeSessionPeerId;
  const packet = {
    channel: "RequestKeyframe",
    type: "request_keyframe"
  };

  // 1. Direct WebRTC DataChannel
  let sent = false;
  if (webrtcPeer && webrtcPeer.dataChannel && webrtcPeer.dataChannel.readyState === "open") {
    sent = webrtcPeer.send(packet);
  }

  // 2. Global Rendezvous Broker
  if (globalSignaling && target) {
    globalSignaling.publishTo(target, packet);
    globalSignaling.publishTo(target, {
      type: "peer_input",
      session_id: currentSessionId,
      input: packet
    });
  }

  // 3. Local Rust backend IPC
  sendIpcMessage("request_keyframe", {});
}

function onWindowRestored() {
  console.log("[Viewer] Window or session restored/focused. Instantly repainting and requesting keyframe...");
  resizeCanvasToFit();
  if (activeSessionPeerId && activeSessions.has(activeSessionPeerId)) {
    const s = activeSessions.get(activeSessionPeerId);
    if (s.lastBitmap && activeCanvas && canvasCtx) {
      canvasCtx.drawImage(s.lastBitmap, 0, 0, activeCanvas.width, activeCanvas.height);
    } else if (s.lastFrame) {
      renderFrame(s.lastFrame);
    }
  }
  requestRemoteKeyframe(activeSessionPeerId);
  setTimeout(resizeCanvasToFit, 50);
}

// Window state listeners for instantaneous minimize/restore performance
document.addEventListener("visibilitychange", () => {
  if (document.visibilityState === "visible") {
    onWindowRestored();
  }
});
window.addEventListener("focus", onWindowRestored);
window.addEventListener("resize", () => {
  if (isSessionActive) onWindowRestored();
});

function safeAddListener(id, event, fn) {
  const el = document.getElementById(id);
  if (el) {
    el.addEventListener(event, fn);
  } else {
    console.warn(`[UI] Element with id '${id}' not found for listener '${event}'`);
  }
}

// Modal Handlers (Callable both via addEventListener and inline onclick)
function acceptIncomingConnection() {
  try {
    const permView = document.getElementById("modal-perm-view");
    const permControl = document.getElementById("modal-perm-control");
    const permClipboard = document.getElementById("modal-perm-clipboard");
    const permFiles = document.getElementById("modal-perm-files");

    const perms = {
      allow_screen_view: permView ? permView.checked : true,
      allow_mouse_keyboard: permControl ? permControl.checked : true,
      allow_clipboard: permClipboard ? permClipboard.checked : true,
      allow_file_transfer: permFiles ? permFiles.checked : true,
      allow_audio: false,
    };

    const peerId = globalSignaling.pendingIncomingFrom || pendingIncomingPeerId || document.getElementById("incoming-peer-id").innerText.trim();

    if (peerId && peerId !== "--- --- ---") {
      globalSignaling.acceptConnection(currentSessionId, peerId, perms);
    }

    sendIpcMessage("approve_incoming", {
      session_id: currentSessionId,
      permissions: perms,
    });
    sendIpcMessage("start_streaming", { permissions: perms });
  } catch (err) {
    console.error("[Modal] Accept error:", err);
  } finally {
    const modal = document.getElementById("incoming-modal");
    if (modal) modal.style.display = "none";
  }
}
window.acceptIncomingConnection = acceptIncomingConnection;

function declineIncomingConnection() {
  try {
    const peerId = globalSignaling.pendingIncomingFrom || pendingIncomingPeerId || document.getElementById("incoming-peer-id").innerText.trim();
    if (peerId && peerId !== "--- --- ---") {
      globalSignaling.declineConnection(currentSessionId, "Declined by local host");
    }
    sendIpcMessage("decline_incoming", {
      session_id: currentSessionId,
      reason: "Declined by local host",
    });
  } catch (err) {
    console.error("[Modal] Decline error:", err);
  } finally {
    const modal = document.getElementById("incoming-modal");
    if (modal) modal.style.display = "none";
  }
}
window.declineIncomingConnection = declineIncomingConnection;

// Event Listeners for UI buttons
function setupEventListeners() {
  // Copy ID Button
  safeAddListener("copy-id-btn", "click", () => {
    const text = document.getElementById("my-desk-id").innerText;
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(text).catch(() => {});
    }
    const btn = document.getElementById("copy-id-btn");
    if (btn) {
      const origHtml = btn.innerHTML;
      btn.innerHTML = `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="20 6 9 17 4 12"></polyline></svg> Copied!`;
      btn.classList.add("btn-primary");
      btn.classList.remove("btn-secondary");
      setTimeout(() => {
        btn.innerHTML = origHtml;
        btn.classList.remove("btn-primary");
        btn.classList.add("btn-secondary");
      }, 1800);
    }
  });

  // Regenerate PIN
  safeAddListener("refresh-pin-btn", "click", () => {
    sendIpcMessage("regenerate_pin", {});
  });

  // Direct Network / WAN / VPN connection helper
  function connectDirectNetwork(targetAddress, pin, unattendedPwd, perms) {
    let host = targetAddress.trim();
    let port = 8088;
    if (host.includes(":")) {
      const parts = host.split(":");
      host = parts[0];
      port = parseInt(parts[1], 10) || 8088;
    }
    const directWsUrl = `ws://${host}:${port}/ws`;
    console.log(`[DirectConnect] Connecting directly to host at ${directWsUrl}...`);

    const statusElem = document.getElementById("connection-status-msg");
    if (statusElem) {
      statusElem.innerText = `Connecting directly to ${host}:${port}...`;
      statusElem.style.color = "var(--primary)";
    }

    let directSocket = null;
    try {
      directSocket = new WebSocket(directWsUrl);
    } catch (err) {
      if (statusElem) {
        statusElem.innerText = `Failed to connect to ${host}:${port}. Verify IP and firewall settings.`;
        statusElem.style.color = "var(--error)";
      }
      return;
    }

    const directTimeout = setTimeout(() => {
      if (directSocket && directSocket.readyState !== WebSocket.OPEN) {
        directSocket.close();
        if (statusElem) {
          statusElem.innerText = `Connection to ${host}:${port} timed out. Verify RemoteDesk is running on the host.`;
          statusElem.style.color = "var(--error)";
        }
      }
    }, 25000);

    directSocket.onopen = () => {
      clearTimeout(directTimeout);
      console.log(`[DirectConnect] Socket connected to ${host}:${port}. Authenticating...`);
      if (statusElem) statusElem.innerText = `Authenticating with ${host}...`;

      const myId = (globalSignaling && globalSignaling.myId) || "999888777";
      directSocket.send(JSON.stringify({
        channel: "Register",
        data: {
          device_id: myId,
          os_info: "Windows Direct Client",
          app_version: "1.1.0.1"
        }
      }));

      setTimeout(() => {
        directSocket.send(JSON.stringify({
          channel: "ConnectRequest",
          data: {
            target_id: "000000000",
            pin: pin || null,
            unattended_password: unattendedPwd || null,
            requested_permissions: perms
          }
        }));
      }, 200);
    };

    directSocket.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        console.log("[DirectConnect] Received message:", data);
        if (data.channel === "SessionResponse" || data.type === "session_response") {
          isSessionActive = true;
          document.getElementById("session-peer-title").innerText = `Connected to: ${host} (Direct Network)`;
          document.getElementById("session-viewer-overlay").style.display = "flex";
          if (statusElem) statusElem.innerText = "";
        }
      } catch (_) {}
    };

    directSocket.onerror = () => {
      clearTimeout(directTimeout);
      if (statusElem) {
        statusElem.innerText = `Could not reach ${host}:${port}. Ensure port 8088 is open or use 9-digit Device ID.`;
        statusElem.style.color = "var(--error)";
      }
    };
  }

  // Connect Button
  safeAddListener("connect-btn", "click", () => {
    const targetInput = document.getElementById("target-id-input").value.trim();
    const pin = document.getElementById("target-pin-input").value.trim();
    const unattendedPwd = document.getElementById("target-unattended-pwd").value;

    if (!targetInput) {
      alert("Please enter a valid 9-digit remote Device ID or IP address");
      return;
    }

    const perms = {
      allow_screen_view: document.getElementById("perm-view") ? document.getElementById("perm-view").checked : true,
      allow_mouse_keyboard: document.getElementById("perm-control") ? document.getElementById("perm-control").checked : true,
      allow_clipboard: document.getElementById("perm-clipboard") ? document.getElementById("perm-clipboard").checked : true,
      allow_file_transfer: document.getElementById("perm-files") ? document.getElementById("perm-files").checked : true,
      allow_audio: false,
    };

    const isDirect = targetInput.includes('.') || targetInput.includes(':') || /[a-zA-Z]/.test(targetInput);

    if (isDirect) {
      document.getElementById("connection-status-msg").innerText = `Connecting directly to ${targetInput} across network...`;
      document.getElementById("connection-status-msg").style.color = "var(--primary)";
      connectDirectNetwork(targetInput, pin, unattendedPwd, perms);
    } else {
      document.getElementById("connection-status-msg").innerText = `Connecting to ${targetInput} via Global Rendezvous...`;
      document.getElementById("connection-status-msg").style.color = "var(--primary)";

      // 1. Initiate cross-network cloud rendezvous
      globalSignaling.connectToPeer(targetInput, pin, unattendedPwd, perms);

      // 2. Also send to local backend for LAN / embedded routing
      sendIpcMessage("initiate_connect", {
        target_id: targetInput,
        pin: pin || null,
        unattended_password: unattendedPwd || null,
        permissions: perms,
      });
    }
  });

  // Disconnect Button
  safeAddListener("btn-disconnect", "click", () => {
    if (globalSignaling.activePeerId) {
      globalSignaling.publishTo(globalSignaling.activePeerId, {
        type: "disconnect",
        session_id: currentSessionId,
        from_id: globalSignaling.myId,
      });
    }
    webrtcPeer.close();
    isSessionActive = false;
    document.getElementById("session-viewer-overlay").style.display = "none";
    document.getElementById("connection-status-msg").innerText = "Session disconnected.";
    sendIpcMessage("disconnect_session", { session_id: currentSessionId });
  });

  // Full Screen
  safeAddListener("btn-fullscreen", "click", () => {
    if (!document.fullscreenElement) {
      document.getElementById("session-viewer-overlay").requestFullscreen();
    } else {
      document.exitFullscreen();
    }
  });

  // Screen Scaling Mode (Fit, Original 1:1, Stretch)
  safeAddListener("scaling-selector", "change", () => {
    resizeCanvasToFit();
  });

  // Toggle Chat Drawer (Viewer Toolbar)
  safeAddListener("btn-toggle-chat", "click", () => {
    const drawer = document.getElementById("chat-drawer");
    if (drawer) {
      drawer.style.display = drawer.style.display === "none" ? "flex" : "none";
      if (drawer.style.display === "flex") {
        const input = document.getElementById("chat-input");
        if (input) input.focus();
      }
    }
  });

  // Host In-Session Floating Widget Buttons
  safeAddListener("host-btn-chat", "click", () => {
    const drawer = document.getElementById("chat-drawer");
    if (drawer) {
      drawer.style.display = drawer.style.display === "none" ? "flex" : "none";
      if (drawer.style.display === "flex") {
        const input = document.getElementById("chat-input");
        if (input) input.focus();
      }
    }
  });

  safeAddListener("host-btn-disconnect", "click", () => {
    if (globalSignaling.activePeerId) {
      globalSignaling.publishTo(globalSignaling.activePeerId, {
        type: "disconnect",
        session_id: currentSessionId,
        from_id: globalSignaling.myId,
      });
    }
    webrtcPeer.close();
    setHostSessionActive(false);
    sendIpcMessage("disconnect_session", { session_id: currentSessionId });
  });

  safeAddListener("chat-close-btn", "click", () => {
    const drawer = document.getElementById("chat-drawer");
    if (drawer) drawer.style.display = "none";
  });

  // Send Chat
  safeAddListener("chat-send-btn", "click", sendChatMessage);
  const chatInput = document.getElementById("chat-input");
  if (chatInput) {
    chatInput.addEventListener("keypress", (e) => {
      if (e.key === "Enter") sendChatMessage();
    });
  }

  // Modal Buttons (Backup event listeners in addition to inline onclick)
  safeAddListener("modal-accept-btn", "click", acceptIncomingConnection);
  safeAddListener("modal-decline-btn", "click", declineIncomingConnection);

  // Unattended Access Toggle
  safeAddListener("unattended-enabled-toggle", "change", (e) => {
    const pnl = document.getElementById("unattended-config-panel");
    if (pnl) pnl.style.display = e.target.checked ? "block" : "none";
  });

  // Save Unattended Password
  safeAddListener("save-unattended-pwd-btn", "click", () => {
    const p1 = document.getElementById("unattended-pwd-input").value;
    const p2 = document.getElementById("unattended-pwd-confirm").value;
    if (p1.length < 8) {
      alert("Password must be at least 8 characters long.");
      return;
    }
    if (p1 !== p2) {
      alert("Passwords do not match.");
      return;
    }
    sendIpcMessage("set_unattended_password", { password: p1, enabled: true });
    alert("Unattended access master password saved successfully!");
  });

  // Save Settings
  safeAddListener("save-settings-btn", "click", () => {
    const srv = document.getElementById("setting-signaling-server").value.trim();
    const alias = document.getElementById("setting-device-alias").value.trim();
    const relayToggle = document.getElementById("setting-relay-toggle");
    const relay = relayToggle ? relayToggle.checked : true;

    sendIpcMessage("update_settings", {
      signaling_server: srv,
      device_alias: alias,
      enable_relay: relay,
    });
    alert("Settings saved!");
  });

  // In-Session File Manager Toggle
  safeAddListener("btn-toggle-filemanager", "click", () => {
    const modal = document.getElementById("session-filemanager-modal");
    if (modal) {
      modal.style.display = "flex";
      refreshLocalFiles();
      sendRemoteFileListRequest(currentRemotePath || "C:\\");
    }
  });

  safeAddListener("sfm-close-btn", "click", () => {
    const modal = document.getElementById("session-filemanager-modal");
    if (modal) modal.style.display = "none";
  });

  // Minimize Remote Viewer to Dashboard
  safeAddListener("btn-minimize-session", "click", () => {
    document.getElementById("session-viewer-overlay").style.display = "none";
    const banner = document.getElementById("minimized-sessions-banner");
    if (banner) {
      banner.style.display = "flex";
      updateSessionToolbarTitle();
    }
  });

  safeAddListener("btn-resume-session", "click", () => {
    document.getElementById("minimized-sessions-banner").style.display = "none";
    document.getElementById("session-viewer-overlay").style.display = "flex";
    onWindowRestored();
  });

  safeAddListener("btn-close-all-minimized", "click", () => {
    for (const peerId of Array.from(activeSessions.keys())) {
      closeSessionTab(peerId);
    }
    document.getElementById("minimized-sessions-banner").style.display = "none";
  });

  // Multi-Connection Modal (+ Connect More)
  safeAddListener("btn-add-session", "click", () => {
    const modal = document.getElementById("new-connection-modal");
    if (modal) {
      modal.style.display = "flex";
      document.getElementById("multi-target-id").value = "";
      document.getElementById("multi-target-pin").value = "";
      document.getElementById("multi-connect-status").innerText = "";
      document.getElementById("multi-target-id").focus();
    }
  });

  safeAddListener("multi-connect-cancel-btn", "click", () => {
    const modal = document.getElementById("new-connection-modal");
    if (modal) modal.style.display = "none";
  });

  safeAddListener("multi-connect-confirm-btn", "click", () => {
    const targetId = document.getElementById("multi-target-id").value.trim();
    const pin = document.getElementById("multi-target-pin").value.trim();
    if (!targetId) {
      alert("Please enter a valid 9-digit Device ID or remote IP address.");
      return;
    }
    document.getElementById("multi-connect-status").innerText = `Connecting to ${targetId}...`;
    document.getElementById("multi-connect-status").style.color = "var(--primary)";

    const perms = {
      allow_screen_view: true,
      allow_mouse_keyboard: true,
      allow_clipboard: true,
      allow_file_transfer: true,
      allow_audio: false
    };

    if (targetId.includes('.') || targetId.includes(':') || /[a-zA-Z]/.test(targetId)) {
      connectDirectNetwork(targetId, pin, null, perms);
    } else {
      globalSignaling.connectToPeer(targetId, pin, null, perms);
    }
    setTimeout(() => {
      const modal = document.getElementById("new-connection-modal");
      if (modal) modal.style.display = "none";
    }, 1500);
  });

  // Direct File Pickers (Upload any file instantly)
  ["direct-upload-file-picker", "tab-direct-upload-input"].forEach((id) => {
    const inp = document.getElementById(id);
    if (inp) {
      inp.addEventListener("change", (e) => {
        if (e.target.files && e.target.files[0]) {
          executeDirectFileUpload(e.target.files[0]);
          e.target.value = "";
        }
      });
    }
  });

  // Upload Buttons
  safeAddListener("btn-upload", "click", executeFileUpload);
  safeAddListener("sfm-btn-upload", "click", executeFileUpload);

  // Download Buttons
  safeAddListener("btn-download", "click", executeFileDownload);
  safeAddListener("sfm-btn-download", "click", executeFileDownload);

  // Refresh Local and Remote Files
  safeAddListener("local-refresh-btn", "click", refreshLocalFiles);
  safeAddListener("sfm-local-refresh-btn", "click", refreshLocalFiles);
  safeAddListener("remote-refresh-btn", "click", () => sendRemoteFileListRequest(currentRemotePath));
  safeAddListener("sfm-remote-refresh-btn", "click", () => sendRemoteFileListRequest(currentRemotePath));

  // Auto-format 9-digit Device ID with spaces (e.g. 123 456 789), preserving IP addresses and hostnames
  const onDeviceIdInput = (e) => {
    const val = e.target.value.trim();
    if (val.includes('.') || val.includes(':') || /[a-zA-Z]/.test(val)) {
      // Direct IP address, port, or hostname: keep exact input without digit formatting
      return;
    }
    let raw = e.target.value.replace(/\D/g, "").slice(0, 9);
    let formatted = "";
    for (let i = 0; i < raw.length; i++) {
      if (i > 0 && i % 3 === 0) formatted += " ";
      formatted += raw[i];
    }
    e.target.value = formatted;
  };
  ["target-id-input", "multi-target-id", "modal-device-id"].forEach((id) => {
    const el = document.getElementById(id);
    if (el) {
      el.addEventListener("input", onDeviceIdInput);
    }
  });

  // Address Book & Saved Devices Listeners
  safeAddListener("search-devices", "input", filterSavedDevices);
  safeAddListener("filter-device-group", "change", filterSavedDevices);
  safeAddListener("btn-open-add-device", "click", () => openSaveDeviceModal());
  safeAddListener("btn-close-device-modal", "click", () => {
    document.getElementById("modal-saved-device").style.display = "none";
  });
  safeAddListener("btn-cancel-device-modal", "click", () => {
    document.getElementById("modal-saved-device").style.display = "none";
  });
  const formDev = document.getElementById("form-saved-device");
  if (formDev) {
    formDev.addEventListener("submit", (e) => {
      e.preventDefault();
      saveDeviceFromModal();
    });
  }

  // Clear Recent History
  safeAddListener("btn-clear-recent", "click", clearRecentHistory);

  // In-Session Quick Actions Dropdown
  safeAddListener("btn-session-actions", "click", (e) => {
    e.stopPropagation();
    const menu = document.getElementById("session-actions-menu");
    if (menu) {
      menu.style.display = menu.style.display === "none" ? "flex" : "none";
    }
  });
  document.addEventListener("click", () => {
    const menu = document.getElementById("session-actions-menu");
    if (menu) menu.style.display = "none";
  });

  // Send Clipboard Modal Listeners
  safeAddListener("btn-close-clipboard-modal", "click", () => {
    document.getElementById("modal-send-clipboard").style.display = "none";
  });
  safeAddListener("btn-cancel-clipboard-modal", "click", () => {
    document.getElementById("modal-send-clipboard").style.display = "none";
  });
  safeAddListener("btn-read-local-clipboard", "click", readFromLocalClipboard);
  safeAddListener("btn-confirm-send-clipboard", "click", sendClipboardTextToRemote);
}

// Multi-Session Management Functions
function addViewerSession(peerId, sessionId) {
  const cleanId = String(peerId).replace(/\s+/g, '');
  if (!activeSessions.has(cleanId)) {
    activeSessions.set(cleanId, {
      peerId: cleanId,
      sessionId: sessionId,
      remotePath: "C:\\",
      remoteFiles: [],
      lastFrame: null,
      lastBitmap: null,
      connectedAt: Date.now()
    });
  }
  activeSessionPeerId = cleanId;
  currentSessionId = sessionId;
  globalSignaling.activePeerId = cleanId;
  renderSessionTabs();
  updateSessionToolbarTitle();
  sendRemoteFileListRequest("C:\\");
  trackRecentDeviceConnection(cleanId);
  requestRemoteKeyframe(cleanId);
  resizeCanvasToFit();
  setTimeout(() => {
    if (activeCanvas) activeCanvas.focus();
    resizeCanvasToFit();
  }, 100);
}

function switchSessionTab(peerId) {
  const cleanId = String(peerId).replace(/\s+/g, '');
  if (!activeSessions.has(cleanId)) return;
  activeSessionPeerId = cleanId;
  const s = activeSessions.get(cleanId);
  currentSessionId = s.sessionId;
  globalSignaling.activePeerId = cleanId;
  renderSessionTabs();
  updateSessionToolbarTitle();
  setTimeout(() => { if (activeCanvas) activeCanvas.focus(); }, 150);
  if (s.lastBitmap && activeCanvas && canvasCtx) {
    canvasCtx.drawImage(s.lastBitmap, 0, 0, activeCanvas.width, activeCanvas.height);
  } else if (s.lastFrame) {
    renderFrame(s.lastFrame);
  }
  requestRemoteKeyframe(cleanId);
  if (s.remotePath) {
    sendRemoteFileListRequest(s.remotePath);
  }
}

function closeSessionTab(peerId) {
  const cleanId = String(peerId).replace(/\s+/g, '');
  const s = activeSessions.get(cleanId);
  if (s) {
    globalSignaling.publishTo(cleanId, {
      type: "disconnect",
      session_id: s.sessionId,
      from_id: globalSignaling.myId,
    });
    activeSessions.delete(cleanId);
  }
  if (activeSessions.size === 0) {
    isSessionActive = false;
    activeSessionPeerId = null;
    webrtcPeer.close();
    document.getElementById("session-viewer-overlay").style.display = "none";
    const banner = document.getElementById("minimized-sessions-banner");
    if (banner) banner.style.display = "none";
    document.getElementById("connection-status-msg").innerText = "All remote sessions closed.";
  } else {
    const nextPeer = activeSessions.keys().next().value;
    switchSessionTab(nextPeer);
  }
}

function renderSessionTabs() {
  const bar = document.getElementById("session-tabs-bar");
  if (!bar) return;
  let html = "";
  for (const [peerId, s] of activeSessions.entries()) {
    const isActive = peerId === activeSessionPeerId;
    const formatted = formatDeviceId(peerId);
    html += `<div class="session-tab ${isActive ? "active" : ""}" onclick="switchSessionTab('${peerId}')">
      <span>🖥️ ${formatted}</span>
      <span class="tab-close" onclick="event.stopPropagation(); closeSessionTab('${peerId}')" title="Disconnect this system">✕</span>
    </div>`;
  }
  bar.innerHTML = html;
}

function updateSessionToolbarTitle() {
  const title = document.getElementById("session-peer-title");
  if (title && activeSessionPeerId) {
    title.innerText = `Connected to: ${formatDeviceId(activeSessionPeerId)}`;
  }
  const minCount = document.getElementById("minimized-sessions-count");
  const minList = document.getElementById("minimized-sessions-list");
  if (minCount) {
    minCount.innerText = `${activeSessions.size} Active Remote Session(s)`;
  }
  if (minList) {
    minList.innerText = Array.from(activeSessions.keys()).map(formatDeviceId).join(" • ");
  }
}

// File Manager Navigation and Rendering Functions
function navigateLocalDir(target) {
  if (target === "up") {
    const trimmed = currentLocalPath.replace(/[/\\]+$/, '');
    const idx = Math.max(trimmed.lastIndexOf('\\'), trimmed.lastIndexOf('/'));
    if (idx > 0) {
      currentLocalPath = trimmed.substring(0, idx);
      if (!currentLocalPath.includes('\\')) currentLocalPath += '\\';
    } else {
      currentLocalPath = "C:\\";
    }
  } else if (target === "downloads" && userDirs.downloads) {
    currentLocalPath = userDirs.downloads;
  } else if (target === "desktop" && userDirs.desktop) {
    currentLocalPath = userDirs.desktop;
  } else if (target === "documents" && userDirs.documents) {
    currentLocalPath = userDirs.documents;
  } else if (target === "c:\\") {
    currentLocalPath = "C:\\";
  } else {
    currentLocalPath = currentLocalPath.replace(/[/\\]+$/, '') + "\\" + target;
  }
  refreshLocalFiles();
}
window.navigateLocalDir = navigateLocalDir;

function navigateRemoteDir(target) {
  if (target === "up") {
    const trimmed = currentRemotePath.replace(/[/\\]+$/, '');
    const idx = Math.max(trimmed.lastIndexOf('\\'), trimmed.lastIndexOf('/'));
    if (idx > 0) {
      currentRemotePath = trimmed.substring(0, idx);
      if (!currentRemotePath.includes('\\')) currentRemotePath += '\\';
    } else {
      currentRemotePath = "C:\\";
    }
  } else if (target === "downloads") {
    currentRemotePath = "C:\\Users\\Public\\Downloads";
  } else if (target === "desktop") {
    currentRemotePath = "C:\\Users\\Public\\Desktop";
  } else if (target === "c:\\") {
    currentRemotePath = "C:\\";
  } else {
    currentRemotePath = currentRemotePath.replace(/[/\\]+$/, '') + "\\" + target;
  }
  sendRemoteFileListRequest(currentRemotePath);
}
window.navigateRemoteDir = navigateRemoteDir;

function refreshLocalFiles() {
  document.querySelectorAll("#local-path, #sfm-local-path").forEach((inp) => inp.value = currentLocalPath);
  sendIpcMessage("list_local_files", { path: currentLocalPath });
}

function sendRemoteFileListRequest(path) {
  const target = activeSessionPeerId || globalSignaling.activePeerId;
  if (!target) return;
  currentRemotePath = path || currentRemotePath || "C:\\";
  document.querySelectorAll("#remote-path, #sfm-remote-path").forEach((inp) => inp.value = currentRemotePath);

  const packet = {
    type: "peer_packet",
    channel: "FileListRequest",
    session_id: currentSessionId,
    from_id: globalSignaling.myId,
    data: { path: currentRemotePath }
  };
  if (!webrtcPeer.send(packet)) {
    globalSignaling.publishTo(target, packet);
  }
}

function renderLocalFiles(path, entries) {
  currentLocalPath = path || currentLocalPath;
  document.querySelectorAll("#local-path, #sfm-local-path").forEach((inp) => inp.value = currentLocalPath);
  ["local-file-list", "sfm-local-file-list"].forEach((cid) => {
    renderFileListPane(cid, entries, (name, isDir) => {
      if (isDir) {
        navigateLocalDir(name);
      } else {
        selectedLocalFile = name;
        updateTransferStatus(`Selected local file: ${name}`);
      }
    });
  });
}

function renderRemoteFiles(path, entries) {
  currentRemotePath = path || currentRemotePath;
  document.querySelectorAll("#remote-path, #sfm-remote-path").forEach((inp) => inp.value = currentRemotePath);
  ["remote-file-list", "sfm-remote-file-list"].forEach((cid) => {
    renderFileListPane(cid, entries, (name, isDir) => {
      if (isDir) {
        navigateRemoteDir(name);
      } else {
        selectedRemoteFile = name;
        updateTransferStatus(`Selected remote file: ${name}`);
      }
    });
  });
  if (activeSessionPeerId && activeSessions.has(activeSessionPeerId)) {
    const s = activeSessions.get(activeSessionPeerId);
    s.remotePath = currentRemotePath;
    s.remoteFiles = entries;
  }
}

function renderFileListPane(containerId, entries, onAction) {
  const container = document.getElementById(containerId);
  if (!container) return;
  if (!entries || entries.length === 0) {
    container.innerHTML = '<div class="empty-state" style="padding: 16px; text-align: center; color: var(--text-muted); font-size: 13px;">Folder is empty</div>';
    return;
  }
  let html = "";
  if (currentLocalPath !== "C:\\" && currentLocalPath !== "C:/" && containerId.includes("local")) {
    html += `<div class="file-item dir" data-name="up" data-dir="true"><span style="font-weight: 600;">📁 .. (Parent Directory)</span><span class="text-muted">&lt;DIR&gt;</span></div>`;
  }
  if (currentRemotePath !== "C:\\" && currentRemotePath !== "C:/" && containerId.includes("remote")) {
    html += `<div class="file-item dir" data-name="up" data-dir="true"><span style="font-weight: 600;">📁 .. (Parent Directory)</span><span class="text-muted">&lt;DIR&gt;</span></div>`;
  }

  entries.forEach((e) => {
    const icon = e.is_dir ? "📁" : "📄";
    const size = e.is_dir ? "<DIR>" : formatBytes(e.size_bytes);
    html += `<div class="file-item ${e.is_dir ? "dir" : "file"}" data-name="${escapeHtml(e.name)}" data-dir="${e.is_dir}">
      <span style="display: flex; align-items: center; gap: 8px;"><span>${icon}</span> <span>${escapeHtml(e.name)}</span></span>
      <span class="text-muted" style="font-size: 11px;">${size}</span>
    </div>`;
  });
  container.innerHTML = html;

  container.querySelectorAll(".file-item").forEach((el) => {
    const isDir = el.getAttribute("data-dir") === "true";
    const name = el.getAttribute("data-name");

    el.addEventListener("click", () => {
      container.querySelectorAll(".file-item").forEach((i) => i.classList.remove("selected"));
      el.classList.add("selected");
      if (!isDir && onAction) onAction(name, false);
    });

    el.addEventListener("dblclick", () => {
      if (onAction) {
        if (name === "up") {
          if (containerId.includes("local")) navigateLocalDir("up");
          else navigateRemoteDir("up");
        } else if (isDir) {
          onAction(name, true);
        }
      }
    });
  });
}

function executeFileUpload() {
  if (!selectedLocalFile) {
    alert("Please click on a file in the Local Machine pane to select it for upload.");
    return;
  }
  const fullPath = currentLocalPath.replace(/[/\\]+$/, '') + "\\" + selectedLocalFile;
  updateTransferProgress(15, `Reading local file: ${selectedLocalFile}...`);
  sendIpcMessage("read_local_file", { path: fullPath, req_id: "upload_" + Date.now() });
}

function executeDirectFileUpload(file) {
  if (!file) return;
  const targetPeer = activeSessionPeerId || globalSignaling.activePeerId;
  if (!targetPeer) {
    alert("Please connect to a remote desktop before uploading files.");
    return;
  }
  updateTransferProgress(20, `Reading ${file.name} (${formatBytes(file.size)})...`);

  const reader = new FileReader();
  reader.onload = () => {
    const result = reader.result;
    const b64 = result.split(',')[1] || result;
    updateTransferProgress(50, `Uploading ${file.name} to remote destination...`);

    const packet = {
      type: "peer_packet",
      channel: "FileTransferUpload",
      session_id: currentSessionId,
      from_id: globalSignaling.myId,
      data: {
        filename: file.name,
        destination_dir: currentRemotePath,
        data_base64: b64,
        size: file.size
      }
    };
    if (!webrtcPeer.send(packet)) {
      globalSignaling.publishTo(targetPeer, packet);
    }
    updateTransferProgress(85, `Transferring data packets to remote host...`);
  };
  reader.readAsDataURL(file);
}

function executeFileDownload() {
  if (!selectedRemoteFile) {
    alert("Please click on a file in the Remote Desktop pane to select it for download.");
    return;
  }
  const targetPeer = activeSessionPeerId || globalSignaling.activePeerId;
  if (!targetPeer) {
    alert("Please connect to a remote desktop before downloading files.");
    return;
  }
  updateTransferProgress(20, `Requesting download of ${selectedRemoteFile} from remote desktop...`);
  const packet = {
    type: "peer_packet",
    channel: "FileTransferDownloadReq",
    session_id: currentSessionId,
    from_id: globalSignaling.myId,
    data: {
      remote_path: currentRemotePath,
      filename: selectedRemoteFile
    }
  };
  if (!webrtcPeer.send(packet)) {
    globalSignaling.publishTo(targetPeer, packet);
  }
}

function updateTransferProgress(percent, statusText) {
  document.querySelectorAll("#transfer-progress-bar, #tab-transfer-progress-bar").forEach((bar) => {
    bar.style.width = `${percent}%`;
  });
  document.querySelectorAll("#transfer-progress-percent, #tab-transfer-progress-percent").forEach((pct) => {
    pct.innerText = `${percent}%`;
  });
  document.querySelectorAll("#transfer-status-text, #tab-transfer-status-text").forEach((txt) => {
    txt.innerText = statusText;
  });
}

function updateTransferStatus(text) {
  document.querySelectorAll("#transfer-status-text, #tab-transfer-status-text").forEach((txt) => {
    txt.innerText = text;
  });
}

function triggerBrowserDownload(filename, base64Data) {
  try {
    const byteCharacters = atob(base64Data);
    const byteNumbers = new Array(byteCharacters.length);
    for (let i = 0; i < byteCharacters.length; i++) {
      byteNumbers[i] = byteCharacters.charCodeAt(i);
    }
    const byteArray = new Uint8Array(byteNumbers);
    const blob = new Blob([byteArray]);
    const link = document.createElement("a");
    link.href = URL.createObjectURL(blob);
    link.download = filename;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    setTimeout(() => URL.revokeObjectURL(link.href), 10000);
  } catch (e) {
    console.warn("Browser download trigger notice:", e);
  }
}

function handlePeerSessionPacket(msg) {
  const ch = msg.channel || (msg.data && msg.data.channel) || (msg.payload && msg.payload.channel) || msg.type;
  const d = msg.data || (msg.payload && msg.payload.data) || msg;

  if (ch === "RequestKeyframe" || ch === "request_keyframe") {
    sendIpcMessage("request_keyframe", {});
    return;
  }

  if (ch === "FileListRequest") {
    const reqPath = (d.data && d.data.path) || d.path || "C:\\";
    lastFileListRequester = msg.from_id;
    sendIpcMessage("list_local_files", { path: reqPath });
  } else if (ch === "FileListResponse") {
    const resPath = (d.data && d.data.path) || d.path || "C:\\";
    const resEntries = (d.data && d.data.entries) || d.entries || [];
    renderRemoteFiles(resPath, resEntries);
  } else if (ch === "FileTransferUpload") {
    const f = d.data || d;
    sendIpcMessage("save_local_file", {
      destination_dir: f.destination_dir,
      filename: f.filename,
      data_base64: f.data_base64
    });
    globalSignaling.publishTo(msg.from_id, {
      type: "peer_packet",
      channel: "FileTransferComplete",
      session_id: currentSessionId,
      from_id: globalSignaling.myId,
      data: { filename: f.filename, size: f.size }
    });
  } else if (ch === "FileTransferDownloadReq") {
    const f = d.data || d;
    const targetPath = f.remote_path.replace(/[/\\]+$/, '') + "\\" + f.filename;
    lastFileDownloadRequester = msg.from_id;
    sendIpcMessage("read_local_file", { path: targetPath, req_id: "dl_" + Date.now() });
  } else if (ch === "FileTransferDownloadRes") {
    const f = d.data || d;
    sendIpcMessage("save_local_file", {
      destination_dir: currentLocalPath,
      filename: f.filename,
      data_base64: f.data_base64
    });
    triggerBrowserDownload(f.filename, f.data_base64);
    updateTransferProgress(100, `Downloaded ${f.filename} successfully (${formatBytes(f.size)})!`);
    refreshLocalFiles();
  } else if (ch === "FileTransferComplete") {
    const f = d.data || d;
    updateTransferProgress(100, `Uploaded ${f.filename} successfully (${formatBytes(f.size)})!`);
    sendRemoteFileListRequest(currentRemotePath);
  }
}

function formatRelativeTime(dateStr) {
  if (!dateStr) return "Never";
  try {
    const d = new Date(dateStr);
    const now = new Date();
    const diffSec = Math.floor((now - d) / 1000);
    if (diffSec < 45) return "Just now";
    if (diffSec < 3600) return `${Math.floor(diffSec / 60)}m ago`;
    if (diffSec < 86400) return `${Math.floor(diffSec / 3600)}h ago`;
    if (diffSec < 604800) return `${Math.floor(diffSec / 86400)}d ago`;
    return d.toLocaleDateString();
  } catch (_) {
    return String(dateStr);
  }
}

function getGroupBadgeClass(group) {
  const g = String(group || "").toLowerCase();
  if (g.includes("work")) return "tag tag-work";
  if (g.includes("server")) return "tag tag-server";
  if (g.includes("client")) return "tag tag-client";
  if (g.includes("support")) return "tag tag-support";
  return "tag tag-general";
}

function renderRecentDevices(devices) {
  const tbody = document.getElementById("recent-table-body");
  if (!tbody) return;
  if (!devices || devices.length === 0) {
    tbody.innerHTML = '<tr><td colspan="5" class="text-center text-muted" style="padding: 24px;">No recent sessions recorded yet. Enter a 9-digit Device ID above to connect.</td></tr>';
    return;
  }
  let html = "";
  devices.slice(0, 15).forEach((d) => {
    const cleanId = String(d.device_id).replace(/\s+/g, '');
    const badgeClass = getGroupBadgeClass(d.group_tag);
    html += `<tr>
      <td><strong style="font-family: var(--font-mono); color: var(--primary);">${formatDeviceId(cleanId)}</strong></td>
      <td><div style="font-weight: 600;">${escapeHtml(d.alias || "Remote Computer")}</div></td>
      <td><span style="font-size: 12px; color: var(--text-muted);">${formatRelativeTime(d.last_seen)}</span></td>
      <td><span class="${badgeClass}">${escapeHtml(d.group_tag || "Recent")}</span></td>
      <td>
        <div style="display: flex; gap: 6px;">
          <button class="btn btn-primary btn-xs" onclick="quickConnect('${cleanId}')" title="Instant Connect">⚡ Connect</button>
          <button class="btn btn-secondary btn-xs" onclick="openSaveDeviceModal('${cleanId}', '${escapeHtml(d.alias || '')}', '${d.group_tag || 'General'}')" title="Save to Address Book">⭐ Bookmark</button>
        </div>
      </td>
    </tr>`;
  });
  tbody.innerHTML = html;
}

function renderSavedDevices(devices) {
  const tbody = document.getElementById("saved-devices-body");
  if (!tbody) return;

  const searchInput = document.getElementById("search-devices");
  const filterQuery = searchInput ? searchInput.value.trim().toLowerCase() : "";
  const groupSelect = document.getElementById("filter-device-group");
  const groupFilter = groupSelect ? groupSelect.value : "all";

  let list = (devices || []).filter((d) => {
    if (groupFilter !== "all" && d.group_tag !== groupFilter) {
      return false;
    }
    if (filterQuery) {
      const idMatch = String(d.device_id).toLowerCase().includes(filterQuery);
      const aliasMatch = String(d.alias || "").toLowerCase().includes(filterQuery);
      const groupMatch = String(d.group_tag || "").toLowerCase().includes(filterQuery);
      return (idMatch || aliasMatch || groupMatch);
    }
    return true;
  });

  if (list.length === 0) {
    tbody.innerHTML = `<tr><td colspan="5" class="text-center text-muted" style="padding: 24px;">No saved devices matching filter in address book. Click "➕ Add Device" to register computers.</td></tr>`;
    return;
  }

  let html = "";
  list.forEach((d) => {
    const cleanId = String(d.device_id).replace(/\s+/g, '');
    const badgeClass = getGroupBadgeClass(d.group_tag);
    html += `<tr>
      <td><strong style="font-family: var(--font-mono); color: var(--primary);">${formatDeviceId(cleanId)}</strong></td>
      <td><div style="font-weight: 600;">${escapeHtml(d.alias || "Computer")}</div></td>
      <td><span class="${badgeClass}">${escapeHtml(d.group_tag || "General")}</span></td>
      <td><span style="font-size: 12px; color: var(--text-muted);">${formatRelativeTime(d.last_seen)}</span></td>
      <td>
        <div style="display: flex; gap: 6px;">
          <button class="btn btn-primary btn-xs" onclick="quickConnect('${cleanId}')" title="1-Click Connect">⚡ Connect</button>
          <button class="btn btn-secondary btn-xs" onclick="openSaveDeviceModal('${cleanId}', '${escapeHtml(d.alias || '')}', '${d.group_tag || 'General'}')" title="Edit device details">✏ Edit</button>
          <button class="btn btn-ghost btn-xs text-danger" onclick="deleteSavedDevice('${cleanId}')" title="Delete device from address book">🗑</button>
        </div>
      </td>
    </tr>`;
  });
  tbody.innerHTML = html;
}

function filterSavedDevices() {
  renderSavedDevices(allDevicesList);
}

function openSaveDeviceModal(id, alias, group) {
  const modal = document.getElementById("modal-saved-device");
  if (!modal) return;
  const cleanId = id ? String(id).replace(/\s+/g, '') : "";
  document.getElementById("modal-device-id").value = cleanId ? formatDeviceId(cleanId) : "";
  document.getElementById("modal-device-alias").value = alias || "";
  if (group && document.getElementById("modal-device-group")) {
    document.getElementById("modal-device-group").value = group;
  }
  const savedPin = cleanId ? localStorage.getItem("pin_" + cleanId) : "";
  document.getElementById("modal-device-pin").value = savedPin || "";
  document.getElementById("modal-device-title").innerText = cleanId ? "✏ Edit Address Book Device" : "➕ Add Device to Address Book";
  modal.style.display = "flex";
  if (!cleanId) {
    document.getElementById("modal-device-id").focus();
  } else {
    document.getElementById("modal-device-alias").focus();
  }
}

function saveDeviceFromModal() {
  const rawId = document.getElementById("modal-device-id").value;
  const cleanId = rawId.replace(/\s+/g, '');
  if (!cleanId || cleanId.length !== 9) {
    alert("Please enter a valid 9-digit Device ID.");
    return;
  }
  const alias = document.getElementById("modal-device-alias").value.trim() || `Device ${formatDeviceId(cleanId)}`;
  const group = document.getElementById("modal-device-group").value || "General";
  const pin = document.getElementById("modal-device-pin").value.trim();

  if (pin) {
    localStorage.setItem("pin_" + cleanId, pin);
  }

  // Update in local cache
  const idx = allDevicesList.findIndex(d => String(d.device_id).replace(/\s+/g, '') === cleanId);
  const updatedDev = {
    device_id: cleanId,
    alias: alias,
    last_seen: (idx >= 0 && allDevicesList[idx].last_seen) ? allDevicesList[idx].last_seen : new Date().toISOString(),
    is_favorite: true,
    group_tag: group
  };
  if (idx >= 0) {
    allDevicesList[idx] = updatedDev;
  } else {
    allDevicesList.unshift(updatedDev);
  }
  localStorage.setItem("remotedesk_devices", JSON.stringify(allDevicesList));
  renderRecentDevices(allDevicesList);
  renderSavedDevices(allDevicesList);

  // Send to backend IPC
  sendIpcMessage("save_address_book_device", {
    device_id: cleanId,
    alias: alias,
    group_tag: group,
    is_favorite: true,
  });

  document.getElementById("modal-saved-device").style.display = "none";
}

function deleteSavedDevice(id) {
  const cleanId = String(id).replace(/\s+/g, '');
  if (confirm(`Remove device ${formatDeviceId(cleanId)} from Address Book?`)) {
    sendIpcMessage("delete_device", { device_id: cleanId });
    allDevicesList = allDevicesList.filter(d => String(d.device_id).replace(/\s+/g, '') !== cleanId);
    localStorage.setItem("remotedesk_devices", JSON.stringify(allDevicesList));
    renderRecentDevices(allDevicesList);
    renderSavedDevices(allDevicesList);
  }
}

function clearRecentHistory() {
  if (confirm("Are you sure you want to clear your recent connection history? Saved address book favorites will be kept.")) {
    sendIpcMessage("clear_recent_history", { keep_favorites: true });
    allDevicesList = allDevicesList.filter(d => d.is_favorite === true);
    localStorage.setItem("remotedesk_devices", JSON.stringify(allDevicesList));
    renderRecentDevices(allDevicesList);
    renderSavedDevices(allDevicesList);
  }
}

function trackRecentDeviceConnection(cleanId) {
  const idx = allDevicesList.findIndex(d => String(d.device_id).replace(/\s+/g, '') === cleanId);
  const now = new Date().toISOString();
  if (idx >= 0) {
    allDevicesList[idx].last_seen = now;
  } else {
    allDevicesList.unshift({
      device_id: cleanId,
      alias: `Device ${formatDeviceId(cleanId)}`,
      last_seen: now,
      is_favorite: false,
      group_tag: "Recent",
    });
  }
  localStorage.setItem("remotedesk_devices", JSON.stringify(allDevicesList));
  renderRecentDevices(allDevicesList);
  renderSavedDevices(allDevicesList);
}

function quickConnect(id) {
  const cleanId = String(id).replace(/\s+/g, '');
  const input = document.getElementById("target-id-input");
  if (input) {
    input.value = formatDeviceId(cleanId);
  }
  const savedPin = localStorage.getItem("pin_" + cleanId);
  if (savedPin) {
    const pinInput = document.getElementById("target-pin-input");
    if (pinInput) pinInput.value = savedPin;
  }
  const connectTabBtn = document.querySelector('[data-tab="connect"]');
  if (connectTabBtn) connectTabBtn.click();

  const connectBtn = document.getElementById("connect-btn");
  if (connectBtn) connectBtn.click();
}

function triggerSystemAction(action) {
  const menu = document.getElementById("session-actions-menu");
  if (menu) menu.style.display = "none";

  const packet = {
    type: "SpecialAction",
    action: action,
  };

  dispatchInputPacket(packet);
}

async function openSendClipboardModal() {
  const menu = document.getElementById("session-actions-menu");
  if (menu) menu.style.display = "none";
  const modal = document.getElementById("modal-send-clipboard");
  const textarea = document.getElementById("clipboard-text-input");
  if (modal && textarea) {
    textarea.value = "";
    if (navigator.clipboard && navigator.clipboard.readText) {
      try {
        const clipText = await navigator.clipboard.readText();
        if (clipText) textarea.value = clipText;
      } catch (_) {}
    }
    modal.style.display = "flex";
    textarea.focus();
  }
}

async function readFromLocalClipboard() {
  const textarea = document.getElementById("clipboard-text-input");
  if (!textarea) return;
  if (navigator.clipboard && navigator.clipboard.readText) {
    try {
      const clipText = await navigator.clipboard.readText();
      if (clipText) textarea.value = clipText;
    } catch (e) {
      alert("Unable to read local clipboard: " + e.message);
    }
  } else {
    alert("Clipboard API not available in this browser context. Please paste manually into the text box.");
  }
}

function sendClipboardTextToRemote() {
  const textarea = document.getElementById("clipboard-text-input");
  if (!textarea) return;
  const text = textarea.value;
  if (!text) {
    alert("Please enter or paste some text first.");
    return;
  }

  const packet = {
    type: "KeyChar",
    character: text,
  };

  dispatchInputPacket(packet);

  document.getElementById("modal-send-clipboard").style.display = "none";
}

function escapeHtml(text) {
  const div = document.createElement("div");
  div.innerText = text;
  return div.innerHTML;
}

function sendChatMessage() {
  const input = document.getElementById("chat-input");
  if (!input) return;
  const msg = input.value.trim();
  if (!msg) return;

  const myDeskId = document.getElementById("my-desk-id") ? document.getElementById("my-desk-id").innerText.trim() : "You";
  const senderName = (currentAppConfig && currentAppConfig.device_alias) ? currentAppConfig.device_alias : myDeskId;

  // Append locally
  appendChatMessage("You", msg, true);
  input.value = "";

  const chatPacket = {
    channel: "Chat",
    type: "Chat",
    data: {
      sender: senderName,
      message: msg,
      timestamp: Date.now()
    }
  };

  // 1. Send over direct WebRTC DataChannel
  let sent = webrtcPeer.send(chatPacket);

  // 2. Also publish over Global Cloud Rendezvous broker
  const targetPeer = globalSignaling.activePeerId || pendingIncomingPeerId;
  if (targetPeer) {
    globalSignaling.publishTo(targetPeer, {
      type: "peer_chat",
      session_id: currentSessionId,
      from_id: globalSignaling.myId,
      sender: senderName,
      message: msg,
      timestamp: Date.now()
    });
  }

  // 3. Notify local backend
  sendIpcMessage("send_chat", { message: msg });
}

function appendChatMessage(sender, text, isMe) {
  const box = document.getElementById("chat-messages");
  if (!box) return;
  const div = document.createElement("div");
  div.className = `chat-msg ${isMe ? "me" : "peer"}`;
  div.innerHTML = `<span class="chat-sender">${escapeHtml(sender)}:</span> <span class="chat-text">${escapeHtml(text)}</span>`;
  box.appendChild(div);
  box.scrollTop = box.scrollHeight;
}

function formatBytes(bytes) {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + " " + sizes[i];
}
