use futures_util::{SinkExt, StreamExt};
use remotedesk_capture::{
    EncoderConfig, FrameEncoder, ScreenCapturer, WindowsInputInjector, WindowsScreenCapturer,
};
use remotedesk_core::auth::{SessionPermissions, SessionPin};
use remotedesk_core::identity::DeviceId;
use remotedesk_core::protocol::{ClientMessage, ServerMessage, SessionPacket};
use remotedesk_core::storage::{LocalDatabase, RecentDevice};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex, RwLock};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum AgentStatus {
    Disconnected,
    Connecting,
    Online,
    InSession,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PendingSession {
    pub session_id: String,
    pub peer_id: DeviceId,
    pub peer_name: String,
    pub peer_ip: String,
    pub requested_permissions: SessionPermissions,
}

#[derive(Clone)]
pub struct AgentState {
    pub status: Arc<RwLock<AgentStatus>>,
    pub device_id: DeviceId,
    pub session_pin: Arc<RwLock<SessionPin>>,
    pub pending_session: Arc<RwLock<Option<PendingSession>>>,
    pub active_session_id: Arc<RwLock<Option<String>>>,
    pub active_sessions: Arc<RwLock<HashMap<String, (DeviceId, SessionPermissions)>>>,
    pub is_streaming: Arc<AtomicBool>,
    pub force_keyframe: Arc<AtomicBool>,
    pub db: LocalDatabase,
    pub ui_event_tx: broadcast::Sender<String>,
    // Outbound signaling channel
    pub signaling_tx: Arc<Mutex<Option<mpsc::UnboundedSender<ClientMessage>>>>,
    // Outbound session data packet channel
    pub session_tx: Arc<Mutex<Option<mpsc::UnboundedSender<SessionPacket>>>>,
    // Notification to immediately reconnect signaling client
    pub reconnect_notify: Arc<tokio::sync::Notify>,
}

impl AgentState {
    pub fn new(db: LocalDatabase, ui_event_tx: broadcast::Sender<String>) -> Self {
        let config = db.get_config();
        Self {
            status: Arc::new(RwLock::new(AgentStatus::Disconnected)),
            device_id: config.device_id,
            session_pin: Arc::new(RwLock::new(SessionPin::generate())),
            pending_session: Arc::new(RwLock::new(None)),
            active_session_id: Arc::new(RwLock::new(None)),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            is_streaming: Arc::new(AtomicBool::new(false)),
            force_keyframe: Arc::new(AtomicBool::new(false)),
            db,
            ui_event_tx,
            signaling_tx: Arc::new(Mutex::new(None)),
            session_tx: Arc::new(Mutex::new(None)),
            reconnect_notify: Arc::new(tokio::sync::Notify::new()),
        }
    }

    pub async fn regenerate_pin(&self) -> String {
        let new_pin = SessionPin::generate();
        let pin_str = new_pin.as_str().to_string();
        *self.session_pin.write().await = new_pin;
        pin_str
    }

    /// Triggers an immediate keyframe on the next capture tick.
    pub fn request_keyframe(&self) {
        self.force_keyframe.store(true, Ordering::SeqCst);
    }

    pub async fn approve_session(
        &self,
        session_id: &str,
        permissions: SessionPermissions,
    ) -> Result<(), String> {
        let pending = self.pending_session.read().await.clone();
        if let Some(p) = pending {
            if p.session_id == session_id {
                let tx = self.signaling_tx.lock().await.clone();
                if let Some(sender) = tx {
                    let _ = sender.send(ClientMessage::SessionResponse {
                        session_id: session_id.to_string(),
                        accepted: true,
                        reason: None,
                        granted_permissions: Some(permissions),
                    });
                }
                *self.pending_session.write().await = None;
                *self.active_session_id.write().await = Some(session_id.to_string());
                self.active_sessions.write().await.insert(session_id.to_string(), (p.peer_id.clone(), permissions));
                *self.status.write().await = AgentStatus::InSession;

                let _ = self.db.log_audit(
                    "SESSION_ACCEPTED",
                    p.peer_id.as_str(),
                    &p.peer_ip,
                    true,
                    "Session approved by local user",
                );
                spawn_capture_streaming_task(self.clone(), permissions);
                return Ok(());
            }
        }
        // If not in pending_session, also check if direct approval (e.g. multi-client)
        *self.active_session_id.write().await = Some(session_id.to_string());
        *self.status.write().await = AgentStatus::InSession;
        spawn_capture_streaming_task(self.clone(), permissions);
        Ok(())
    }

    pub async fn start_streaming(&self, permissions: SessionPermissions) {
        *self.status.write().await = AgentStatus::InSession;
        spawn_capture_streaming_task(self.clone(), permissions);
    }

    pub async fn decline_session(&self, session_id: &str, reason: &str) -> Result<(), String> {
        let pending = self.pending_session.read().await.clone();
        if let Some(p) = pending {
            if p.session_id == session_id {
                let tx = self.signaling_tx.lock().await.clone();
                if let Some(sender) = tx {
                    let _ = sender.send(ClientMessage::SessionResponse {
                        session_id: session_id.to_string(),
                        accepted: false,
                        reason: Some(reason.to_string()),
                        granted_permissions: None,
                    });
                }
                *self.pending_session.write().await = None;

                let _ = self.db.log_audit(
                    "SESSION_DECLINED",
                    p.peer_id.as_str(),
                    &p.peer_ip,
                    false,
                    reason,
                );
                return Ok(());
            }
        }
        Err("No matching pending session to decline".to_string())
    }

    pub async fn terminate_session(&self) {
        let sess_id = self.active_session_id.read().await.clone();
        if let Some(id) = sess_id {
            let tx = self.signaling_tx.lock().await.clone();
            if let Some(sender) = tx {
                let _ = sender.send(ClientMessage::Disconnect {
                    session_id: id.clone(),
                    reason: Some("Host terminated session".to_string()),
                });
            }
        }
        self.active_sessions.write().await.clear();
        *self.active_session_id.write().await = None;
        *self.status.write().await = AgentStatus::Online;

        let _ = self.db.log_audit(
            "SESSION_TERMINATED",
            "RemotePeer",
            "127.0.0.1",
            true,
            "Host terminated all sessions",
        );
    }
}

/// Runs the background Host Agent event loop.
pub async fn run_agent_loop(state: AgentState, mut stop_rx: mpsc::Receiver<()>) {
    loop {
        *state.status.write().await = AgentStatus::Connecting;
        let config = state.db.get_config();

        let ws_url = &config.signaling_server;
        println!("[Agent] Connecting to signaling server at {}", ws_url);

        match connect_async(ws_url).await {
            Ok((ws_stream, _)) => {
                println!("[Agent] Connected to signaling network. Registering device ID: {}", state.device_id.formatted());
                *state.status.write().await = AgentStatus::Online;

                let (mut ws_sender, mut ws_receiver) = ws_stream.split();
                let (client_tx, mut client_rx) = mpsc::unbounded_channel::<ClientMessage>();
                *state.signaling_tx.lock().await = Some(client_tx.clone());

                // Send registration message
                let _ = client_tx.send(ClientMessage::Register {
                    device_id: state.device_id.clone(),
                    token: None,
                    os_info: format!("Windows x86_64 ({})", config.device_alias),
                    app_version: "1.1.0.1".to_string(),
                });

                // Spawn outbound sender task
                let send_task = tokio::spawn(async move {
                    while let Some(msg) = client_rx.recv().await {
                        if let Ok(json) = serde_json::to_string(&msg) {
                            if ws_sender.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                });

                // Listen for messages from signaling server
                loop {
                    tokio::select! {
                        _ = stop_rx.recv() => {
                            println!("[Agent] Shutdown signal received.");
                            send_task.abort();
                            return;
                        }
                        _ = state.reconnect_notify.notified() => {
                            println!("[Agent] Signaling server reconfigured. Reconnecting...");
                            send_task.abort();
                            break;
                        }
                        maybe_msg = ws_receiver.next() => {
                            match maybe_msg {
                                Some(Ok(Message::Text(text))) => {
                                    if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&text) {
                                        handle_server_message(&state, server_msg).await;
                                    }
                                }
                                Some(Ok(Message::Close(_))) | None => {
                                    println!("[Agent] Signaling connection closed by server.");
                                    break;
                                }
                                _ => {}
                            }
                        }
                    }
                }

                send_task.abort();
                *state.signaling_tx.lock().await = None;
                *state.status.write().await = AgentStatus::Disconnected;
            }
            Err(_) => {
                // Background retry while embedded or external server initializes
                *state.status.write().await = AgentStatus::Connecting;
            }
        }

        tokio::select! {
            _ = stop_rx.recv() => return,
            _ = state.reconnect_notify.notified() => {},
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(800)) => {}
        }
    }
}

async fn handle_server_message(state: &AgentState, msg: ServerMessage) {
    match msg {
        ServerMessage::Registered { assigned_id, .. } => {
            println!("[Agent] Successfully registered with ID: {}", assigned_id.formatted());
        }
        ServerMessage::IncomingSession {
            session_id,
            peer_id,
            peer_name,
            peer_ip,
            requested_permissions,
        } => {
            println!(
                "[Agent] Incoming session request from {} ({}) [IP: {}]",
                peer_name, peer_id.formatted(), peer_ip
            );

            let config = state.db.get_config();

            // Check if unattended access mode applies
            if config.unattended_access_enabled {
                println!("[Agent] Auto-accepting session via Unattended Access policy");
                let _ = state.approve_session(&session_id, config.default_permissions).await;
                let _ = state.db.log_audit(
                    "UNATTENDED_LOGIN",
                    peer_id.as_str(),
                    &peer_ip,
                    true,
                    "Authenticated via unattended access policy",
                );
                return;
            }

            // Interactive approval required: notify UI
            *state.pending_session.write().await = Some(PendingSession {
                session_id: session_id.clone(),
                peer_id: peer_id.clone(),
                peer_name: peer_name.clone(),
                peer_ip: peer_ip.clone(),
                requested_permissions,
            });

            let _ = state.db.log_audit(
                "INCOMING_SESSION_REQUEST",
                peer_id.as_str(),
                &peer_ip,
                true,
                "Prompting local user for interactive authorization",
            );

            // Forward incoming request modal to UI
            let evt = serde_json::json!({
                "type": "incoming_session",
                "payload": {
                    "session_id": session_id,
                    "peer_id": peer_id.formatted(),
                    "peer_name": peer_name,
                    "peer_ip": peer_ip,
                }
            });
            let _ = state.ui_event_tx.send(evt.to_string());
        }
        ServerMessage::SessionEstablished { session_id, peer_id, permissions, .. } => {
            println!("[Agent] Session {} active with peer {}", session_id, peer_id.formatted());
            *state.active_session_id.write().await = Some(session_id.clone());
            *state.status.write().await = AgentStatus::InSession;

            let _ = state.db.log_audit(
                "SESSION_STARTED",
                peer_id.as_str(),
                "127.0.0.1",
                true,
                "Active remote session established",
            );

            // Record session in recent devices
            let _ = state.db.add_or_update_device(RecentDevice {
                device_id: peer_id.clone(),
                alias: format!("Device {}", peer_id.formatted()),
                last_seen: chrono::Utc::now(),
                is_favorite: false,
                group_tag: "Recent".to_string(),
            });

            // Forward session started to UI
            let evt = serde_json::json!({
                "type": "session_started",
                "payload": {
                    "session_id": session_id,
                    "peer_id": peer_id.formatted(),
                }
            });
            let _ = state.ui_event_tx.send(evt.to_string());

            // Broadcast updated recent devices to UI
            let recent = state.db.get_recent_devices();
            let recent_evt = serde_json::json!({
                "type": "recent_devices_updated",
                "payload": { "devices": recent }
            });
            let _ = state.ui_event_tx.send(recent_evt.to_string());

            // Spawn live screen capture streamer task
            spawn_capture_streaming_task(state.clone(), permissions);
        }
        ServerMessage::SessionRejected { session_id: _, reason } => {
            println!("[Agent] Session request rejected: {}", reason);
            let evt = serde_json::json!({
                "type": "error",
                "payload": {
                    "message": format!("Connection request declined: {}", reason),
                }
            });
            let _ = state.ui_event_tx.send(evt.to_string());
        }
        ServerMessage::SessionClosed { session_id, reason } => {
            println!("[Agent] Session {} ended: {}", session_id, reason);
            *state.active_session_id.write().await = None;
            *state.status.write().await = AgentStatus::Online;

            let evt = serde_json::json!({
                "type": "session_ended",
                "payload": {
                    "session_id": session_id,
                    "reason": reason,
                }
            });
            let _ = state.ui_event_tx.send(evt.to_string());
        }
        ServerMessage::Error { code: _, message } => {
            let evt = serde_json::json!({
                "type": "error",
                "payload": {
                    "message": message,
                }
            });
            let _ = state.ui_event_tx.send(evt.to_string());
        }
        ServerMessage::PeerIceCandidate { session_id: _, candidate } => {
            let evt = serde_json::json!({
                "type": "peer_ice_candidate",
                "payload": {
                    "candidate": candidate,
                }
            });
            let _ = state.ui_event_tx.send(evt.to_string());
        }
        ServerMessage::PeerOffer { session_id: _, sdp } => {
            let evt = serde_json::json!({
                "type": "peer_offer",
                "payload": {
                    "sdp": sdp,
                }
            });
            let _ = state.ui_event_tx.send(evt.to_string());
        }
        ServerMessage::PeerAnswer { session_id: _, sdp } => {
            let evt = serde_json::json!({
                "type": "peer_answer",
                "payload": {
                    "sdp": sdp,
                }
            });
            let _ = state.ui_event_tx.send(evt.to_string());
        }
        _ => {}
    }
}

/// Spawns the real-time screen capture & input processing worker.
fn spawn_capture_streaming_task(state: AgentState, permissions: SessionPermissions) {
    if state.is_streaming.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        // Streamer loop is already active for existing sessions
        return;
    }

    tokio::spawn(async move {
        let mut capturer = WindowsScreenCapturer::new();
        let mut encoder = FrameEncoder::new(EncoderConfig::default());
        let _input_injector = WindowsInputInjector::new();

        println!("[Streamer] Screen capture engine started (30 FPS target)");

        let mut last_cursor_shape = String::new();
        while *state.status.read().await == AgentStatus::InSession {
            let start = tokio::time::Instant::now();

            if state.force_keyframe.swap(false, Ordering::SeqCst) {
                encoder.force_next_keyframe();
            }

            if permissions.allow_screen_view {
                match capturer.capture_monitor(0) {
                    Ok(frame) => {
                        // Immediately detect and sync remote cursor shape changes (arrow <-> I-beam text)
                        if frame.cursor_shape != last_cursor_shape {
                            last_cursor_shape = frame.cursor_shape.clone();
                            let cursor_evt = serde_json::json!({
                                "type": "cursor_shape",
                                "payload": { "shape": last_cursor_shape.clone() },
                            });
                            let _ = state.ui_event_tx.send(cursor_evt.to_string());

                            let tx = state.session_tx.lock().await.clone();
                            if let Some(sender) = tx {
                                let _ = sender.send(SessionPacket::CursorShape {
                                    shape: last_cursor_shape.clone(),
                                });
                            }
                        }

                        if let Ok(Some(packet)) = encoder.encode_to_packet(&frame) {
                            // Forward encoded frame to local/remote viewer via UI event channel
                            let frame_evt = serde_json::json!({
                                "type": "screen_frame",
                                "payload": packet,
                            });
                            let _ = state.ui_event_tx.send(frame_evt.to_string());

                            let tx = state.session_tx.lock().await.clone();
                            if let Some(sender) = tx {
                                let _ = sender.send(packet);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[Streamer] Frame capture error: {}", e);
                    }
                }
            }

            let elapsed = start.elapsed();
            let target_fps = encoder.config.max_fps.max(10).min(60) as u64;
            let frame_target = tokio::time::Duration::from_millis(1000 / target_fps);
            if elapsed < frame_target {
                tokio::time::sleep(frame_target - elapsed).await;
            }
        }

        state.is_streaming.store(false, Ordering::SeqCst);
        println!("[Streamer] Screen capture loop stopped.");
    });
}
