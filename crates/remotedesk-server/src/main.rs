use remotedesk_server::run_server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8088);

    println!("============================================================");
    println!("  RemoteDesk Free - Signaling, Relay & Admin Server");
    println!("  Version:   1.1.0.1");
    println!("  Company:   Freeware | Suresh Mamidala");
    println!("  Publisher: Suresh Mamidala");
    println!("  Signaling WS:  ws://127.0.0.1:{}/ws", port);
    println!("  Admin Console: http://127.0.0.1:{}/admin", port);
    println!("  Status API:    http://127.0.0.1:{}/api/status", port);
    println!("============================================================");

    if let Err(e) = run_server(port).await {
        eprintln!("[Server Error] {}", e);
    }

    Ok(())
}
