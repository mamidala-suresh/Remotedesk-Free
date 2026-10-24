use crate::agent::AgentState;
use crate::client::ViewerSession;
use crate::file_transfer::FileManager;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use remotedesk_capture::InputInjector;
use remotedesk_core::auth::SaltedHash;
use remotedesk_core::identity::DeviceId;
use remotedesk_core::protocol::ClientMessage;
use remotedesk_core::storage::RecentDevice;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use tokio_tungstenite::tungstenite::Message;

// Embed UI assets directly into binary for a portable standalone zero-dependency build
const HTML_CONTENT: &str = include_str!("../../../ui/index.html");
const CSS_CONTENT: &str = include_str!("../../../ui/styles.css");
const JS_CONTENT: &str = include_str!("../../../ui/app.js");
const MQTT_CONTENT: &str = include_str!("../../../ui/mqtt.min.js");
const ICON_CONTENT: &[u8] = include_bytes!("../../../ui/icon.png");

fn get_local_ip() -> String {
    if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = socket.local_addr() {
                return addr.ip().to_string();
            }
        }
    }
    "127.0.0.1".to_string()
}

/// Runs the local embedded UI HTTP and WebSocket IPC server.
pub async fn start_ui_server(
    port: u16,
    agent: AgentState,
    viewer: ViewerSession,
    ui_event_tx: broadcast::Sender<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(addr).await?;

    println!("[UI Server] Desktop UI server listening on http://127.0.0.1:{}", port);

    while let Ok((mut stream, _)) = listener.accept().await {
        let agent = agent.clone();
        let viewer = viewer.clone();
        let ui_event_tx = ui_event_tx.clone();

        tokio::spawn(async move {
            let mut peek_buf = [0u8; 1024];
            let n = match stream.peek(&mut peek_buf).await {
                Ok(n) if n > 0 => n,
                _ => return,
            };

            let req_str = String::from_utf8_lossy(&peek_buf[..n]);
            let is_ws = req_str.to_lowercase().contains("upgrade: websocket");

            if is_ws {
                let callback = |_req: &Request, resp: Response| Ok(resp);
                if let Ok(ws_stream) = accept_hdr_async(stream, callback).await {
                    handle_ipc_client(ws_stream, agent, viewer, ui_event_tx).await;
                }
            } else {
                // Serve static assets
                let mut buf = vec![0u8; 2048];
                if let Ok(len) = stream.read(&mut buf).await {
                    let text = String::from_utf8_lossy(&buf[..len]);
                    let path = text
                        .lines()
                        .next()
                        .and_then(|l| l.split_whitespace().nth(1))
                        .unwrap_or("/");

                    if path == "/icon.png" || path == "/favicon.ico" {
                        let header = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            ICON_CONTENT.len()
                        );
                        let _ = stream.write_all(header.as_bytes()).await;
                        let _ = stream.write_all(ICON_CONTENT).await;
                        let _ = stream.flush().await;
                    } else {
                        let (content_type, body) = match path {
                            "/" | "/index.html" => ("text/html; charset=utf-8", HTML_CONTENT),
                            "/styles.css" => ("text/css; charset=utf-8", CSS_CONTENT),
                            "/app.js" => ("application/javascript; charset=utf-8", JS_CONTENT),
                            "/mqtt.min.js" => ("application/javascript; charset=utf-8", MQTT_CONTENT),
                            _ => ("text/plain; charset=utf-8", "Not Found"),
                        };

                        let response = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            content_type,
                            body.len(),
                            body
                        );
                        let _ = stream.write_all(response.as_bytes()).await;
                        let _ = stream.flush().await;
                    }
                }
            }
        });
    }

    Ok(())
}

async fn handle_ipc_client(
    ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    agent: AgentState,
    viewer: ViewerSession,
    ui_event_tx: broadcast::Sender<String>,
) {
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let mut ui_rx = ui_event_tx.subscribe();

    // Outbound task: forwards state changes & screen frames to UI WebSocket
    let outbound_task = tokio::spawn(async move {
        while let Ok(msg) = ui_rx.recv().await {
            if ws_sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Inbound task state: packet deduplication tracking
    let mut seen_input_seqs = std::collections::HashSet::<u64>::new();
    let mut seq_order = std::collections::VecDeque::<u64>::new();
    let mut last_key_event: Option<(String, u32, tokio::time::Instant)> = None;

    // Inbound task: processes commands from the desktop frontend
    while let Some(Ok(msg)) = ws_receiver.next().await {
        if let Message::Text(text) = msg {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                let msg_type = val["type"].as_str().unwrap_or("");
                let payload = &val["payload"];

                match msg_type {
                    "get_state" => {
                        let cfg = agent.db.get_config();
                        let pin = agent.session_pin.read().await.as_str().to_string();
                        let recent = agent.db.get_recent_devices();
                        let audit = agent.db.get_audit_logs();

                        let state_json = serde_json::json!({
                            "type": "state_update",
                            "payload": {
                                "device_id": agent.device_id.as_str(),
                                "formatted_id": agent.device_id.formatted(),
                                "lan_ip": get_local_ip(),
                                "pin": pin,
                                "config": cfg,
                                "recent_devices": recent,
                                "audit_logs": audit,
                            }
                        });
                        let _ = ui_event_tx.send(state_json.to_string());
                    }

                    "regenerate_pin" => {
                        let new_pin = agent.regenerate_pin().await;
                        let json = serde_json::json!({
                            "type": "state_update",
                            "payload": {
                                "pin": new_pin
                            }
                        });
                        let _ = ui_event_tx.send(json.to_string());
                    }

                    "initiate_connect" => {
                        let target_str = payload["target_id"].as_str().unwrap_or("");
                        let pin = payload["pin"].as_str().map(|s| s.to_string());
                        let unattended_pwd = payload["unattended_password"].as_str().map(|s| s.to_string());

                        match DeviceId::new(target_str) {
                            Ok(target_id) => {
                                // Record in recent devices
                                let dev = RecentDevice {
                                    device_id: target_id.clone(),
                                    alias: format!("Device {}", target_id.formatted()),
                                    last_seen: chrono::Utc::now(),
                                    is_favorite: false,
                                    group_tag: "Recent".to_string(),
                                };
                                let _ = agent.db.add_or_update_device(dev);
                                let recent = agent.db.get_recent_devices();
                                let recent_evt = serde_json::json!({
                                    "type": "recent_devices_updated",
                                    "payload": { "devices": recent }
                                });
                                let _ = ui_event_tx.send(recent_evt.to_string());

                                let perms = serde_json::from_value(payload["permissions"].clone()).unwrap_or_default();
                                let tx = agent.signaling_tx.lock().await.clone();
                                if let Some(sender) = tx {
                                    let _ = sender.send(ClientMessage::ConnectRequest {
                                        target_id: target_id.clone(),
                                        pin,
                                        unattended_password: unattended_pwd,
                                        requested_permissions: perms,
                                    });
                                } else {
                                    let err = serde_json::json!({
                                        "type": "error",
                                        "payload": { "message": "Signaling server is not connected" }
                                    });
                                    let _ = ui_event_tx.send(err.to_string());
                                }
                            }
                            Err(e) => {
                                let err = serde_json::json!({
                                    "type": "error",
                                    "payload": { "message": format!("Invalid Device ID format: {}", e) }
                                });
                                let _ = ui_event_tx.send(err.to_string());
                            }
                        }
                    }

                    "approve_incoming" => {
                        let sid = payload["session_id"].as_str().unwrap_or("");
                        let perms = serde_json::from_value(payload["permissions"].clone()).unwrap_or_default();
                        let _ = agent.approve_session(sid, perms).await;
                        let recent = agent.db.get_recent_devices();
                        let recent_evt = serde_json::json!({
                            "type": "recent_devices_updated",
                            "payload": { "devices": recent }
                        });
                        let _ = ui_event_tx.send(recent_evt.to_string());
                    }

                    "start_streaming" => {
                        let perms = serde_json::from_value(payload["permissions"].clone()).unwrap_or_default();
                        agent.start_streaming(perms).await;
                    }

                    "request_keyframe" => {
                        agent.request_keyframe();
                    }

                    "decline_incoming" => {
                        let sid = payload["session_id"].as_str().unwrap_or("");
                        let reason = payload["reason"].as_str().unwrap_or("Declined");
                        let _ = agent.decline_session(sid, reason).await;
                    }

                    "disconnect_session" => {
                        agent.terminate_session().await;
                        viewer.disconnect().await;
                        let json = serde_json::json!({
                            "type": "session_ended",
                            "payload": { "reason": "Disconnected by user" }
                        });
                        let _ = ui_event_tx.send(json.to_string());
                    }

                    "send_input" => {
                        let mut injector = remotedesk_capture::WindowsInputInjector::new();
                        let input_type = payload["type"].as_str().or_else(|| payload["channel"].as_str()).unwrap_or("");
                        let sw = payload["screen_w"].as_u64().unwrap_or(0) as u32;
                        let sh = payload["screen_h"].as_u64().unwrap_or(0) as u32;

                        // Deduplication by sequence ID if supplied by client
                        if let Some(seq) = payload["seq"].as_u64() {
                            if seq > 0 {
                                if seen_input_seqs.contains(&seq) {
                                    continue; // Duplicate packet received (e.g. delivered across both WebRTC & MQTT or retransmitted)
                                }
                                seen_input_seqs.insert(seq);
                                seq_order.push_back(seq);
                                if seq_order.len() > 1000 {
                                    if let Some(old) = seq_order.pop_front() {
                                        seen_input_seqs.remove(&old);
                                    }
                                }
                            }
                        }

                        // Anti-duplicate protection for rapid identical KeyDown/KeyChar events (<25ms without KeyUp)
                        if input_type == "KeyDown" || input_type == "KeyChar" {
                            let key_id = payload["key_code"].as_u64().unwrap_or(0) as u32;
                            let now = tokio::time::Instant::now();
                            if let Some((prev_type, prev_key, prev_time)) = &last_key_event {
                                if *prev_type == input_type && *prev_key == key_id && now.duration_since(*prev_time).as_millis() < 25 {
                                    continue; // Discard synthetic bounce / duplicate delivery
                                }
                            }
                            last_key_event = Some((input_type.to_string(), key_id, now));
                        } else if input_type == "KeyUp" {
                            last_key_event = None;
                        }

                        match input_type {
                            "MouseMove" => {
                                let x = payload["x"].as_u64().unwrap_or(0) as u32;
                                let y = payload["y"].as_u64().unwrap_or(0) as u32;
                                let _ = injector.mouse_move(x, y, sw, sh);
                            }
                            "MouseDown" => {
                                let btn = match payload["button"].as_str().unwrap_or("Left") {
                                    "Right" => remotedesk_core::protocol::MouseButton::Right,
                                    "Middle" => remotedesk_core::protocol::MouseButton::Middle,
                                    _ => remotedesk_core::protocol::MouseButton::Left,
                                };
                                let x = payload["x"].as_u64().unwrap_or(0) as u32;
                                let y = payload["y"].as_u64().unwrap_or(0) as u32;
                                let _ = injector.mouse_down(btn, x, y, sw, sh);
                            }
                            "MouseUp" => {
                                let btn = match payload["button"].as_str().unwrap_or("Left") {
                                    "Right" => remotedesk_core::protocol::MouseButton::Right,
                                    "Middle" => remotedesk_core::protocol::MouseButton::Middle,
                                    _ => remotedesk_core::protocol::MouseButton::Left,
                                };
                                let x = payload["x"].as_u64().unwrap_or(0) as u32;
                                let y = payload["y"].as_u64().unwrap_or(0) as u32;
                                let _ = injector.mouse_up(btn, x, y, sw, sh);
                            }
                            "MouseWheel" => {
                                let dy = payload["delta_y"].as_i64().unwrap_or(0) as i32;
                                let dx = payload["delta_x"].as_i64().unwrap_or(0) as i32;
                                let _ = injector.mouse_wheel(dx, dy);
                            }
                            "KeyDown" => {
                                let key = payload["key_code"].as_u64().unwrap_or(0) as u32;
                                let _ = injector.key_down(key, 0, Default::default());
                            }
                            "KeyUp" => {
                                let key = payload["key_code"].as_u64().unwrap_or(0) as u32;
                                let _ = injector.key_up(key, 0, Default::default());
                            }
                            "KeyChar" => {
                                if let Some(ch) = payload["character"].as_str() {
                                    let _ = injector.text_input(ch);
                                }
                            }
                            "SpecialAction" | "system_action" => {
                                if let Some(action) = payload["action"].as_str() {
                                    let _ = injector.execute_system_action(action);
                                }
                            }
                            "RequestKeyframe" | "request_keyframe" => {
                                agent.request_keyframe();
                            }
                            _ => {}
                        }
                    }

                    "system_action" => {
                        let mut injector = remotedesk_capture::WindowsInputInjector::new();
                        let action = payload["action"].as_str().unwrap_or("");
                        if action == "text_input" || action == "send_text" {
                            if let Some(text) = payload["text"].as_str() {
                                let _ = injector.text_input(text);
                            }
                        } else {
                            let _ = injector.execute_system_action(action);
                        }
                    }

                    "save_address_book_device" => {
                        let target_str = payload["device_id"].as_str().unwrap_or("");
                        let alias = payload["alias"].as_str().unwrap_or("");
                        let group_tag = payload["group_tag"].as_str().unwrap_or("General");
                        let is_fav = payload["is_favorite"].as_bool().unwrap_or(true);

                        if let Ok(dev_id) = DeviceId::new(target_str) {
                            let dev = RecentDevice {
                                device_id: dev_id,
                                alias: if alias.is_empty() { format!("Device {}", target_str) } else { alias.to_string() },
                                last_seen: chrono::Utc::now(),
                                is_favorite: is_fav,
                                group_tag: group_tag.to_string(),
                            };
                            let _ = agent.db.add_or_update_device(dev);
                            let recent = agent.db.get_recent_devices();
                            let recent_evt = serde_json::json!({
                                "type": "recent_devices_updated",
                                "payload": { "devices": recent }
                            });
                            let _ = ui_event_tx.send(recent_evt.to_string());
                        }
                    }

                    "delete_device" => {
                        let target_str = payload["device_id"].as_str().unwrap_or("");
                        if let Ok(dev_id) = DeviceId::new(target_str) {
                            let _ = agent.db.remove_device(&dev_id);
                            let recent = agent.db.get_recent_devices();
                            let recent_evt = serde_json::json!({
                                "type": "recent_devices_updated",
                                "payload": { "devices": recent }
                            });
                            let _ = ui_event_tx.send(recent_evt.to_string());
                        }
                    }

                    "clear_recent_history" => {
                        let keep_fav = payload["keep_favorites"].as_bool().unwrap_or(true);
                        let _ = agent.db.clear_recent_history(keep_fav);
                        let recent = agent.db.get_recent_devices();
                        let recent_evt = serde_json::json!({
                            "type": "recent_devices_updated",
                            "payload": { "devices": recent }
                        });
                        let _ = ui_event_tx.send(recent_evt.to_string());
                    }

                    "send_webrtc_candidate" => {
                        let session_id = payload["session_id"].as_str().unwrap_or("").to_string();
                        let candidate = payload["candidate"].as_str().unwrap_or("").to_string();
                        let tx = agent.signaling_tx.lock().await.clone();
                        if let Some(sender) = tx {
                            let _ = sender.send(ClientMessage::IceCandidate { session_id, candidate });
                        }
                    }

                    "send_webrtc_offer" => {
                        let session_id = payload["session_id"].as_str().unwrap_or("").to_string();
                        let sdp = payload["sdp"].as_str().unwrap_or("").to_string();
                        let tx = agent.signaling_tx.lock().await.clone();
                        if let Some(sender) = tx {
                            let _ = sender.send(ClientMessage::SessionOffer { session_id, sdp });
                        }
                    }

                    "send_webrtc_answer" => {
                        let session_id = payload["session_id"].as_str().unwrap_or("").to_string();
                        let sdp = payload["sdp"].as_str().unwrap_or("").to_string();
                        let tx = agent.signaling_tx.lock().await.clone();
                        if let Some(sender) = tx {
                            let _ = sender.send(ClientMessage::SessionAnswer { session_id, sdp });
                        }
                    }

                    "send_chat" => {
                        let msg = payload["message"].as_str().unwrap_or("");
                        viewer.send_chat("You", msg).await;
                    }

                    "set_unattended_password" => {
                        let pwd = payload["password"].as_str().unwrap_or("");
                        let enabled = payload["enabled"].as_bool().unwrap_or(true);
                        let hash = SaltedHash::create(pwd);
                        let _ = agent.db.update_config(|c| {
                            c.unattended_access_enabled = enabled;
                            c.unattended_password_hash = Some(hash);
                        });
                        let _ = agent.db.log_audit(
                            "UNATTENDED_CONFIG_UPDATED",
                            "LocalUser",
                            "127.0.0.1",
                            true,
                            "Unattended master password changed",
                        );
                    }

                    "update_settings" => {
                        let srv = payload["signaling_server"].as_str();
                        let alias = payload["device_alias"].as_str();
                        let relay = payload["enable_relay"].as_bool();

                        let _ = agent.db.update_config(|c| {
                            if let Some(s) = srv { c.signaling_server = s.to_string(); }
                            if let Some(a) = alias { c.device_alias = a.to_string(); }
                            if let Some(r) = relay { c.enable_relay = r; }
                        });
                        agent.reconnect_notify.notify_one();
                    }

                    "list_local_files" => {
                        let path = payload["path"].as_str().unwrap_or("C:\\");
                        let entries = FileManager::list_directory(path).unwrap_or_default();
                        let json = serde_json::json!({
                            "type": "local_files",
                            "payload": {
                                "path": path,
                                "entries": entries
                            }
                        });
                        let _ = ui_event_tx.send(json.to_string());
                    }

                    "get_user_dirs" => {
                        let userprofile = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string());
                        let downloads = format!("{}\\Downloads", userprofile);
                        let desktop = format!("{}\\Desktop", userprofile);
                        let documents = format!("{}\\Documents", userprofile);
                        let json = serde_json::json!({
                            "type": "user_dirs",
                            "payload": {
                                "home": userprofile,
                                "downloads": downloads,
                                "desktop": desktop,
                                "documents": documents,
                            }
                        });
                        let _ = ui_event_tx.send(json.to_string());
                    }

                    "read_local_file" => {
                        let path_str = payload["path"].as_str().unwrap_or("");
                        let req_id = payload["req_id"].as_str().unwrap_or("");
                        match std::fs::read(path_str) {
                            Ok(bytes) => {
                                let b64 = BASE64.encode(&bytes);
                                let filename = std::path::Path::new(path_str)
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("file")
                                    .to_string();
                                let json = serde_json::json!({
                                    "type": "file_data_read",
                                    "payload": {
                                        "req_id": req_id,
                                        "path": path_str,
                                        "filename": filename,
                                        "size": bytes.len(),
                                        "data_base64": b64
                                    }
                                });
                                let _ = ui_event_tx.send(json.to_string());
                            }
                            Err(e) => {
                                let json = serde_json::json!({
                                    "type": "file_error",
                                    "payload": { "req_id": req_id, "message": format!("Cannot read file {}: {}", path_str, e) }
                                });
                                let _ = ui_event_tx.send(json.to_string());
                            }
                        }
                    }

                    "save_local_file" => {
                        let dest_dir = payload["destination_dir"].as_str().unwrap_or("");
                        let filename = payload["filename"].as_str().unwrap_or("downloaded_file");
                        let data_b64 = payload["data_base64"].as_str().unwrap_or("");

                        let target_dir = if !dest_dir.is_empty() {
                            std::path::PathBuf::from(dest_dir)
                        } else {
                            std::env::var("USERPROFILE")
                                .map(|p| std::path::PathBuf::from(p).join("Downloads"))
                                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                        };

                        let target_path = target_dir.join(filename);

                        match BASE64.decode(data_b64) {
                            Ok(bytes) => {
                                if let Some(parent) = target_path.parent() {
                                    let _ = std::fs::create_dir_all(parent);
                                }
                                match std::fs::write(&target_path, &bytes) {
                                    Ok(_) => {
                                        let json = serde_json::json!({
                                            "type": "file_saved",
                                            "payload": {
                                                "saved_path": target_path.to_string_lossy().to_string(),
                                                "filename": filename,
                                                "size": bytes.len()
                                            }
                                        });
                                        let _ = ui_event_tx.send(json.to_string());
                                    }
                                    Err(e) => {
                                        let json = serde_json::json!({
                                            "type": "file_error",
                                            "payload": { "message": format!("Failed to write file {}: {}", target_path.display(), e) }
                                        });
                                        let _ = ui_event_tx.send(json.to_string());
                                    }
                                }
                            }
                            Err(e) => {
                                let json = serde_json::json!({
                                    "type": "file_error",
                                    "payload": { "message": format!("Base64 decoding error: {}", e) }
                                });
                                let _ = ui_event_tx.send(json.to_string());
                            }
                        }
                    }

                    _ => {}
                }
            }
        }
    }

    outbound_task.abort();
}
