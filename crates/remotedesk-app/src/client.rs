use remotedesk_core::auth::SessionPermissions;
use remotedesk_core::identity::DeviceId;
use remotedesk_core::protocol::{DirEntryInfo, SessionPacket};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum ClientSessionStatus {
    Idle,
    Connecting,
    WaitingApproval,
    Connected,
    Disconnected,
    Failed(String),
}

/// Statistics for the active remote session.
#[allow(dead_code)]
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct SessionStats {
    pub current_fps: u32,
    pub rtt_ms: u64,
    pub total_bytes_received: u64,
    pub remote_resolution: String,
    pub remote_monitor_count: u32,
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct ViewerSession {
    pub status: Arc<RwLock<ClientSessionStatus>>,
    pub target_id: Arc<RwLock<Option<DeviceId>>>,
    pub session_id: Arc<RwLock<Option<String>>>,
    pub stats: Arc<RwLock<SessionStats>>,
    pub granted_permissions: Arc<RwLock<SessionPermissions>>,
    pub chat_history: Arc<RwLock<Vec<(String, String, i64)>>>, // (sender, msg, timestamp)
    pub remote_files: Arc<RwLock<Vec<DirEntryInfo>>>,
    pub remote_current_dir: Arc<RwLock<String>>,
    // Outbound packet channel to remote host
    pub outbound_tx: Arc<Mutex<Option<mpsc::UnboundedSender<SessionPacket>>>>,
}

impl Default for ViewerSession {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewerSession {
    pub fn new() -> Self {
        Self {
            status: Arc::new(RwLock::new(ClientSessionStatus::Idle)),
            target_id: Arc::new(RwLock::new(None)),
            session_id: Arc::new(RwLock::new(None)),
            stats: Arc::new(RwLock::new(SessionStats::default())),
            granted_permissions: Arc::new(RwLock::new(SessionPermissions::default())),
            chat_history: Arc::new(RwLock::new(Vec::new())),
            remote_files: Arc::new(RwLock::new(Vec::new())),
            remote_current_dir: Arc::new(RwLock::new("C:\\".to_string())),
            outbound_tx: Arc::new(Mutex::new(None)),
        }
    }

    /// Disconnects the active viewer session.
    pub async fn disconnect(&self) {
        let sid = self.session_id.read().await.clone();
        if let Some(_) = sid {
            *self.status.write().await = ClientSessionStatus::Disconnected;
            *self.session_id.write().await = None;
            *self.target_id.write().await = None;
            *self.outbound_tx.lock().await = None;
        }
    }

    /// Sends an input or command packet to the remote host.
    pub async fn send_packet(&self, packet: SessionPacket) -> bool {
        let tx = self.outbound_tx.lock().await.clone();
        if let Some(sender) = tx {
            sender.send(packet).is_ok()
        } else {
            false
        }
    }

    /// Adds a chat message.
    pub async fn send_chat(&self, sender: &str, content: &str) {
        let now = chrono::Utc::now().timestamp_millis();
        self.chat_history
            .write()
            .await
            .push((sender.to_string(), content.to_string(), now));

        self.send_packet(SessionPacket::Chat {
            sender: sender.to_string(),
            message: content.to_string(),
            timestamp: now,
        })
        .await;
    }
}
