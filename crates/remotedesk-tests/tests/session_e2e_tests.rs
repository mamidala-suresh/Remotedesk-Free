use futures_util::{SinkExt, StreamExt};
use remotedesk_core::auth::SessionPermissions;
use remotedesk_core::identity::DeviceId;
use remotedesk_core::protocol::{ClientMessage, ServerMessage};
use tokio::net::TcpListener;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

#[tokio::test]
async fn test_signaling_session_lifecycle_and_rejection() {
    // 1. Bind test signaling server on ephemeral port
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server_url = format!("ws://127.0.0.1:{}/ws", port);

    use std::sync::Arc;
    use tokio::sync::Mutex;

    let clients: Arc<Mutex<std::collections::HashMap<DeviceId, tokio::sync::mpsc::UnboundedSender<ServerMessage>>>> =
        Arc::new(Mutex::new(std::collections::HashMap::new()));
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let server_clients = Arc::clone(&clients);
    // Spawn test signaling server
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut shutdown_rx => break,
                res = listener.accept() => {
                    let (stream, _addr) = match res {
                        Ok(val) => val,
                        Err(_) => break,
                    };
                    let ws_stream = tokio_tungstenite::accept_async(stream).await.unwrap();
                    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
                    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<ServerMessage>();
                    let clients = Arc::clone(&server_clients);

                    tokio::spawn(async move {
                        while let Some(msg) = rx.recv().await {
                            let json = serde_json::to_string(&msg).unwrap();
                            let _ = ws_sender.send(Message::Text(json.into())).await;
                        }
                    });

                    tokio::spawn(async move {
                        while let Some(Ok(Message::Text(text))) = ws_receiver.next().await {
                            let client_msg: ClientMessage = match serde_json::from_str(&text) {
                                Ok(m) => m,
                                Err(_) => continue,
                            };
                            match client_msg {
                                ClientMessage::Register { device_id, .. } => {
                                    clients.lock().await.insert(device_id.clone(), tx.clone());
                                    let _ = tx.send(ServerMessage::Registered {
                                        success: true,
                                        message: "OK".to_string(),
                                        assigned_id: device_id,
                                    });
                                }
                                ClientMessage::ConnectRequest { target_id, requested_permissions, .. } => {
                                    let guard = clients.lock().await;
                                    if let Some(target_tx) = guard.get(&target_id) {
                                        let _ = target_tx.send(ServerMessage::IncomingSession {
                                            session_id: "test_session_42".to_string(),
                                            peer_id: DeviceId::new("111222333").unwrap(),
                                            peer_name: "Viewer Laptop".to_string(),
                                            peer_ip: "127.0.0.1".to_string(),
                                            requested_permissions,
                                        });
                                    } else {
                                        let _ = tx.send(ServerMessage::Error {
                                            code: "PEER_OFFLINE".to_string(),
                                            message: format!("Device {} is currently offline", target_id),
                                        });
                                    }
                                }
                                ClientMessage::SessionResponse { session_id, accepted, reason, .. } => {
                                    let guard = clients.lock().await;
                                    let viewer_tx = guard.get(&DeviceId::new("111222333").unwrap());
                                    if let Some(vtx) = viewer_tx {
                                        if accepted {
                                            let _ = vtx.send(ServerMessage::SessionEstablished {
                                                session_id,
                                                peer_id: DeviceId::new("444555666").unwrap(),
                                                permissions: SessionPermissions::full_control(),
                                                relay_url: None,
                                            });
                                        } else {
                                            let _ = vtx.send(ServerMessage::SessionRejected {
                                                session_id,
                                                reason: reason.unwrap_or_else(|| "Declined".to_string()),
                                            });
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    });
                }
            }
        }
    });

    // 2. Connect Host Agent
    let host_id = DeviceId::new("444555666").unwrap();
    let (mut host_ws, _) = connect_async(&server_url).await.unwrap();

    let reg_host = ClientMessage::Register {
        device_id: host_id.clone(),
        token: None,
        os_info: "Windows 11".to_string(),
        app_version: "1.0.0".to_string(),
    };
    host_ws
        .send(Message::Text(serde_json::to_string(&reg_host).unwrap().into()))
        .await
        .unwrap();

    let host_resp = host_ws.next().await.unwrap().unwrap();
    let host_server_msg: ServerMessage = serde_json::from_str(&host_resp.to_text().unwrap()).unwrap();
    assert!(matches!(host_server_msg, ServerMessage::Registered { .. }));

    // 3. Connect Viewer Client
    let viewer_id = DeviceId::new("111222333").unwrap();
    let (mut viewer_ws, _) = connect_async(&server_url).await.unwrap();

    let reg_viewer = ClientMessage::Register {
        device_id: viewer_id.clone(),
        token: None,
        os_info: "Windows 10".to_string(),
        app_version: "1.0.0".to_string(),
    };
    viewer_ws
        .send(Message::Text(serde_json::to_string(&reg_viewer).unwrap().into()))
        .await
        .unwrap();

    let viewer_resp = viewer_ws.next().await.unwrap().unwrap();
    let viewer_server_msg: ServerMessage = serde_json::from_str(&viewer_resp.to_text().unwrap()).unwrap();
    assert!(matches!(viewer_server_msg, ServerMessage::Registered { .. }));

    // 4. Test Rejection when targeting Offline Device
    let offline_id = DeviceId::new("999888777").unwrap();
    let offline_req = ClientMessage::ConnectRequest {
        target_id: offline_id,
        pin: None,
        unattended_password: None,
        requested_permissions: SessionPermissions::default(),
    };
    viewer_ws
        .send(Message::Text(serde_json::to_string(&offline_req).unwrap().into()))
        .await
        .unwrap();

    let offline_resp = viewer_ws.next().await.unwrap().unwrap();
    let offline_msg: ServerMessage = serde_json::from_str(&offline_resp.to_text().unwrap()).unwrap();
    match offline_msg {
        ServerMessage::Error { code, message } => {
            assert_eq!(code, "PEER_OFFLINE");
            assert!(message.contains("offline"));
        }
        other => panic!("Expected PEER_OFFLINE error, got: {:?}", other),
    }

    // 5. Test Successful Interactive Session Handshake
    let online_req = ClientMessage::ConnectRequest {
        target_id: host_id.clone(),
        pin: None,
        unattended_password: None,
        requested_permissions: SessionPermissions::full_control(),
    };
    viewer_ws
        .send(Message::Text(serde_json::to_string(&online_req).unwrap().into()))
        .await
        .unwrap();

    // Host receives IncomingSession
    let incoming_resp = host_ws.next().await.unwrap().unwrap();
    let incoming_msg: ServerMessage = serde_json::from_str(&incoming_resp.to_text().unwrap()).unwrap();
    match incoming_msg {
        ServerMessage::IncomingSession { session_id, peer_id, .. } => {
            assert_eq!(session_id, "test_session_42");
            assert_eq!(peer_id, viewer_id);

            // Host approves session
            let accept_msg = ClientMessage::SessionResponse {
                session_id,
                accepted: true,
                reason: None,
                granted_permissions: Some(SessionPermissions::full_control()),
            };
            host_ws
                .send(Message::Text(serde_json::to_string(&accept_msg).unwrap().into()))
                .await
                .unwrap();
        }
        other => panic!("Expected IncomingSession, got: {:?}", other),
    }

    // Viewer receives SessionEstablished
    let established_resp = viewer_ws.next().await.unwrap().unwrap();
    let established_msg: ServerMessage = serde_json::from_str(&established_resp.to_text().unwrap()).unwrap();
    assert!(matches!(established_msg, ServerMessage::SessionEstablished { .. }));

    let _ = shutdown_tx.send(());
}
