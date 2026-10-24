use crate::registry::DeviceRegistry;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

/// Handles a connection to the encrypted relay channel.
pub async fn handle_relay_connection(
    ws_stream: WebSocketStream<TcpStream>,
    session_id: String,
    is_initiator: bool,
    registry: Arc<DeviceRegistry>,
) {
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // Outbound writer
    let write_task = tokio::spawn(async move {
        while let Some(packet) = rx.recv().await {
            if ws_sender.send(Message::Text(packet.into())).await.is_err() {
                break;
            }
        }
    });

    // Inbound reader forwards packets to the registry for the session's counterpart
    while let Some(Ok(msg)) = ws_receiver.next().await {
        match msg {
            Message::Text(text) => {
                registry
                    .relay_packet(&session_id, is_initiator, text.to_string())
                    .await;
            }
            Message::Binary(bin) => {
                // For binary data, can pass as base64 or raw binary frame
                let text = String::from_utf8_lossy(&bin).to_string();
                registry.relay_packet(&session_id, is_initiator, text).await;
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    drop(tx);
    write_task.abort();
}
