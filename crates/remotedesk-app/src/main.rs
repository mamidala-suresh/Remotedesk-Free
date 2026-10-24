#![windows_subsystem = "windows"]

use std::path::PathBuf;
use tokio::sync::{broadcast, mpsc};

mod agent;
mod client;
mod file_transfer;
mod ui_server;

use agent::{run_agent_loop, AgentState};
use client::ViewerSession;
use remotedesk_core::storage::LocalDatabase;
use ui_server::start_ui_server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize Windows 10/11 Per-Monitor DPI Awareness V2
    remotedesk_capture::init_process_dpi_awareness();

    println!("============================================================");
    println!("  RemoteDesk Free - Desktop Application & Host Agent");
    println!("  Version:   1.1.0.1");
    println!("  Company:   Freeware | Suresh Mamidala");
    println!("  Publisher: Suresh Mamidala");
    println!("============================================================");

    let mut ui_port: u16 = 9090;
    let mut is_daemon = false;
    let mut _auto_connect_id: Option<String> = None;

    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                if i + 1 < args.len() {
                    if let Ok(p) = args[i + 1].parse() {
                        ui_port = p;
                    }
                    i += 1;
                }
            }
            "--daemon" => {
                is_daemon = true;
            }
            "--connect" => {
                if i + 1 < args.len() {
                    _auto_connect_id = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--help" | "-h" => {
                println!("Usage: remotedesk-app [OPTIONS]");
                println!("  --port <PORT>      Specify embedded UI port (default: 9090)");
                println!("  --daemon           Run as unattended background service without GUI window");
                println!("  --connect <ID>     Automatically connect to remote Device ID on startup");
                return Ok(());
            }
            _ => {}
        }
        i += 1;
    }

    // Resolve app data directory (%APPDATA%\RemoteDeskFree or ./data)
    let app_data_dir = std::env::var("APPDATA")
        .map(|p| PathBuf::from(p).join("RemoteDeskFree"))
        .unwrap_or_else(|_| PathBuf::from("./data"));

    println!("[Init] Storing local configuration and audit logs in: {}", app_data_dir.display());
    let db = LocalDatabase::init(&app_data_dir)?;

    let (ui_broadcast_tx, _) = broadcast::channel::<String>(200);
    let agent_state = AgentState::new(db.clone(), ui_broadcast_tx.clone());
    let viewer_session = ViewerSession::new();

    // Auto-start embedded signaling & relay server in the background (zero-setup out of the box)
    tokio::spawn(async move {
        let _ = remotedesk_server::run_server(8088).await;
    });
    // Brief sleep to allow embedded server socket to bind cleanly
    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

    // Channel for shutting down the background agent gracefully
    let (stop_agent_tx, stop_agent_rx) = mpsc::channel::<()>(1);

    // Spawn Background Agent Task
    let agent_clone = agent_state.clone();
    tokio::spawn(async move {
        run_agent_loop(agent_clone, stop_agent_rx).await;
    });

    // Spawn Embedded UI Server Task
    let ui_agent = agent_state.clone();
    let ui_viewer = viewer_session.clone();
    let ui_tx = ui_broadcast_tx.clone();
    tokio::spawn(async move {
        if let Err(e) = start_ui_server(ui_port, ui_agent, ui_viewer, ui_tx).await {
            eprintln!("[UI Server] Error running UI server: {}", e);
        }
    });

    let app_url = format!("http://127.0.0.1:{}", ui_port);
    println!("[UI] Ready at: {}", app_url);

    // Launch Native Desktop Application Window (if not running in --daemon mode)
    if !is_daemon {
        launch_desktop_window(&app_url);
    } else {
        println!("[Daemon] Running in headless background service mode.");
    }

    // Wait for shutdown signal (Ctrl+C)
    tokio::signal::ctrl_c().await?;
    println!("\n[Shutdown] Shutting down RemoteDesk Free...");
    let _ = stop_agent_tx.send(()).await;

    Ok(())
}

/// Launches a chromeless native desktop window via Edge App mode or default browser.
fn launch_desktop_window(url: &str) {
    #[cfg(windows)]
    {
        use std::process::Command;
        let edge_candidates = [
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
            r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
        ];
        for edge_path in &edge_candidates {
            if std::path::Path::new(edge_path).exists() {
                let _ = Command::new(edge_path)
                    .arg(format!("--app={}", url))
                    .arg("--window-size=1280,840")
                    .arg("--high-dpi-support=1")
                    .arg("--enable-features=DpiVirtualization")
                    .spawn();
                return;
            }
        }
        // Fallback to default browser
        let _ = Command::new("cmd").args(&["/C", "start", url]).spawn();
    }

    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}
