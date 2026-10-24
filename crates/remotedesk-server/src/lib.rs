pub mod admin;
pub mod registry;
pub mod relay;
pub mod signaling;

use admin::handle_admin_api;
use registry::DeviceRegistry;
use signaling::handle_signaling_connection;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};

pub async fn run_server(port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr).await?;
    let registry = Arc::new(DeviceRegistry::new());

    println!("[Server] Embedded signaling & relay running on ws://0.0.0.0:{}/ws", port);

    while let Ok((mut stream, peer_addr)) = listener.accept().await {
        let registry = Arc::clone(&registry);

        tokio::spawn(async move {
            let mut buf = [0u8; 2048];
            let n = match stream.peek(&mut buf).await {
                Ok(n) if n > 0 => n,
                _ => return,
            };

            let header_str = String::from_utf8_lossy(&buf[..n]);
            let is_ws_upgrade = header_str.to_lowercase().contains("upgrade: websocket");

            if is_ws_upgrade {
                let mut requested_path = String::new();
                let callback = |req: &Request, resp: Response| {
                    requested_path = req.uri().path().to_string();
                    Ok(resp)
                };

                if let Ok(ws_stream) = accept_hdr_async(stream, callback).await {
                    if requested_path.starts_with("/relay") {
                        let session_id = "default".to_string();
                        relay::handle_relay_connection(ws_stream, session_id, true, registry).await;
                    } else {
                        handle_signaling_connection(ws_stream, peer_addr, registry).await;
                    }
                }
            } else {
                let mut full_buf = vec![0u8; 4096];
                if let Ok(bytes_read) = stream.read(&mut full_buf).await {
                    let req_str = String::from_utf8_lossy(&full_buf[..bytes_read]);
                    let path = req_str
                        .lines()
                        .next()
                        .and_then(|line| line.split_whitespace().nth(1))
                        .unwrap_or("/");

                    if let Some((content_type, body)) = handle_admin_api(path, &registry).await {
                        let response = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: {}; charset=utf-8\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n{}",
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
