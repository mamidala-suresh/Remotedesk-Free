use crate::registry::{DeviceRegistry, OnlineDevice};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use rand::Rng;
use remotedesk_core::identity::DeviceId;
use remotedesk_core::protocol::{ClientMessage, ServerMessage};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

/// Handles an incoming client WebSocket connection for signaling.
pub async fn handle_signaling_connection(
    ws_stream: WebSocketStream<TcpStream>,
    peer_addr: SocketAddr,
    registry: Arc<DeviceRegistry>,
) {
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<ServerMessage>();

    // Spawn a background task to flush ServerMessages back to the client over WebSocket
    let flush_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let json = match serde_json::to_string(&msg) {
                Ok(j) => j,
                Err(e) => {
                    eprintln!("Failed to serialize ServerMessage: {}", e);
                    continue;
                }
            };
            if ws_sender.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    });

    let mut current_device_id: Option<DeviceId> = None;

    while let Some(Ok(msg)) = ws_receiver.next().await {
        match msg {
            Message::Text(text) => {
                let client_msg: ClientMessage = match serde_json::from_str(&text) {
                    Ok(m) => m,
                    Err(e) => {
                        let _ = tx.send(ServerMessage::Error {
                            code: "BAD_REQUEST".to_string(),
                            message: format!("Malformed message: {}", e),
                        });
                        continue;
                    }
                };

                process_client_message(
                    client_msg,
                    &tx,
                    &mut current_device_id,
                    peer_addr,
                    &registry,
                )
                .await;
            }
            Message::Ping(payload) => {
                // Handled automatically by tungstenite
                let _ = payload;
            }
            Message::Close(_) => {
                break;
            }
            _ => {}
        }
    }

    // Cleanup on disconnect
    if let Some(ref dev_id) = current_device_id {
        let affected = registry.unregister(dev_id).await;
        for sess_id in affected {
            println!("Closed orphaned session {} due to disconnect of {}", sess_id, dev_id);
        }
    }

    flush_task.abort();
}

async fn process_client_message(
    msg: ClientMessage,
    tx: &mpsc::UnboundedSender<ServerMessage>,
    current_device_id: &mut Option<DeviceId>,
    peer_addr: SocketAddr,
    registry: &Arc<DeviceRegistry>,
) {
    match msg {
        ClientMessage::Register {
            device_id,
            token: _,
            os_info,
            app_version,
        } => {
            *current_device_id = Some(device_id.clone());
            let online_device = OnlineDevice {
                device_id: device_id.clone(),
                peer_addr: peer_addr.to_string(),
                os_info,
                app_version,
                connected_at: Utc::now(),
                sender: tx.clone(),
            };
            registry.register(online_device).await;

            let _ = tx.send(ServerMessage::Registered {
                success: true,
                message: "Registered successfully on signaling network".to_string(),
                assigned_id: device_id,
            });
        }

        ClientMessage::ConnectRequest {
            target_id,
            pin: _,
            unattended_password: _,
            requested_permissions,
        } => {
            let sender_id = match current_device_id {
                Some(ref id) => id.clone(),
                None => {
                    let _ = tx.send(ServerMessage::Error {
                        code: "NOT_REGISTERED".to_string(),
                        message: "Must register device ID before initiating connection".to_string(),
                    });
                    return;
                }
            };

            // Check if target device is online
            if !registry.is_online(&target_id).await {
                let _ = tx.send(ServerMessage::Error {
                    code: "PEER_OFFLINE".to_string(),
                    message: format!(
                        "Device {} is currently offline or unreachable. Verify the ID and try again.",
                        target_id.formatted()
                    ),
                });
                return;
            }

            // Generate unique session identifier
            let session_id = format!("sess_{:08x}", rand::thread_rng().gen::<u32>());

            // Store active session in registry
            registry
                .create_session(
                    session_id.clone(),
                    sender_id.clone(),
                    target_id.clone(),
                    requested_permissions,
                )
                .await;

            // Route incoming request to target device
            let sent = registry
                .send_to(
                    &target_id,
                    ServerMessage::IncomingSession {
                        session_id: session_id.clone(),
                        peer_id: sender_id.clone(),
                        peer_name: format!("Remote Client ({})", sender_id.formatted()),
                        peer_ip: peer_addr.ip().to_string(),
                        requested_permissions,
                    },
                )
                .await;

            if !sent {
                let _ = registry.remove_session(&session_id).await;
                let _ = tx.send(ServerMessage::Error {
                    code: "ROUTING_FAILED".to_string(),
                    message: "Failed to deliver request to target host".to_string(),
                });
            }
        }

        ClientMessage::SessionResponse {
            session_id,
            accepted,
            reason,
            granted_permissions,
        } => {
            let host_id = match current_device_id {
                Some(ref id) => id.clone(),
                None => return,
            };

            if let Some(session) = registry.get_session(&session_id).await {
                if accepted {
                    let perms = granted_permissions.unwrap_or(session.permissions);
                    let relay_url = format!("/relay?session={}", session_id);

                    // Notify Initiator that session is established
                    let _ = registry
                        .send_to(
                            &session.initiator_id,
                            ServerMessage::SessionEstablished {
                                session_id: session_id.clone(),
                                peer_id: host_id.clone(),
                                permissions: perms,
                                relay_url: Some(relay_url.clone()),
                            },
                        )
                        .await;

                    // Notify Host that session is established
                    let _ = registry
                        .send_to(
                            &host_id,
                            ServerMessage::SessionEstablished {
                                session_id: session_id.clone(),
                                peer_id: session.initiator_id.clone(),
                                permissions: perms,
                                relay_url: Some(relay_url),
                            },
                        )
                        .await;
                } else {
                    let reason_str = reason.unwrap_or_else(|| "Session request declined by host".to_string());
                    let _ = registry
                        .send_to(
                            &session.initiator_id,
                            ServerMessage::SessionRejected {
                                session_id: session_id.clone(),
                                reason: reason_str,
                            },
                        )
                        .await;
                    let _ = registry.remove_session(&session_id).await;
                }
            } else {
                // Direct response fallback if session entry was not found
                if accepted {
                    let perms = granted_permissions.unwrap_or_default();
                    let _ = tx.send(ServerMessage::SessionEstablished {
                        session_id: session_id.clone(),
                        peer_id: host_id,
                        permissions: perms,
                        relay_url: Some(format!("/relay?session={}", session_id)),
                    });
                } else {
                    let reason_str = reason.unwrap_or_else(|| "Session request declined by host".to_string());
                    let _ = tx.send(ServerMessage::SessionRejected {
                        session_id,
                        reason: reason_str,
                    });
                }
            }
        }

        ClientMessage::Disconnect { session_id, reason } => {
            let reason_str = reason.unwrap_or_else(|| "User disconnected session".to_string());
            if let Some(session) = registry.remove_session(&session_id).await {
                let other_peer = if Some(&session.initiator_id) == current_device_id.as_ref() {
                    &session.target_id
                } else {
                    &session.initiator_id
                };
                let _ = registry
                    .send_to(
                        other_peer,
                        ServerMessage::SessionClosed {
                            session_id,
                            reason: reason_str,
                        },
                    )
                    .await;
            }
        }

        ClientMessage::IceCandidate { session_id, candidate } => {
            if let Some(session) = registry.get_session(&session_id).await {
                let target = if Some(&session.initiator_id) == current_device_id.as_ref() {
                    &session.target_id
                } else {
                    &session.initiator_id
                };
                let _ = registry
                    .send_to(
                        target,
                        ServerMessage::PeerIceCandidate {
                            session_id,
                            candidate,
                        },
                    )
                    .await;
            }
        }

        ClientMessage::SessionOffer { session_id, sdp } => {
            if let Some(session) = registry.get_session(&session_id).await {
                let target = if Some(&session.initiator_id) == current_device_id.as_ref() {
                    &session.target_id
                } else {
                    &session.initiator_id
                };
                let _ = registry
                    .send_to(target, ServerMessage::PeerOffer { session_id, sdp })
                    .await;
            }
        }

        ClientMessage::SessionAnswer { session_id, sdp } => {
            if let Some(session) = registry.get_session(&session_id).await {
                let target = if Some(&session.initiator_id) == current_device_id.as_ref() {
                    &session.target_id
                } else {
                    &session.initiator_id
                };
                let _ = registry
                    .send_to(target, ServerMessage::PeerAnswer { session_id, sdp })
                    .await;
            }
        }

        ClientMessage::Ping => {
            let _ = tx.send(ServerMessage::Pong);
        }
    }
}
