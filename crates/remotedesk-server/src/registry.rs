use chrono::{DateTime, Utc};
use remotedesk_core::auth::SessionPermissions;
use remotedesk_core::identity::DeviceId;
use remotedesk_core::protocol::ServerMessage;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc::UnboundedSender, RwLock};

/// Information about an active online peer connection.
#[derive(Clone)]
pub struct OnlineDevice {
    pub device_id: DeviceId,
    pub peer_addr: String,
    pub os_info: String,
    pub app_version: String,
    pub connected_at: DateTime<Utc>,
    pub sender: UnboundedSender<ServerMessage>,
}

/// Information about an active remote support session between two peers.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ActiveSession {
    pub session_id: String,
    pub initiator_id: DeviceId,
    pub target_id: DeviceId,
    pub started_at: DateTime<Utc>,
    pub permissions: SessionPermissions,
}

/// Central in-memory registry for online devices and active sessions.
#[derive(Clone, Default)]
pub struct DeviceRegistry {
    devices: Arc<RwLock<HashMap<DeviceId, OnlineDevice>>>,
    sessions: Arc<RwLock<HashMap<String, ActiveSession>>>,
    // Maps session_id to data packet relay channels (initiator sender, target sender)
    relays: Arc<RwLock<HashMap<String, (UnboundedSender<String>, UnboundedSender<String>)>>>,
}

impl DeviceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a device. Replaces existing connection if the same ID re-registers.
    pub async fn register(&self, device: OnlineDevice) {
        let mut map = self.devices.write().await;
        map.insert(device.device_id.clone(), device);
    }

    /// Unregisters a device and cleans up sessions it is participating in.
    pub async fn unregister(&self, device_id: &DeviceId) -> Vec<String> {
        let mut map = self.devices.write().await;
        map.remove(device_id);

        let mut affected_sessions = Vec::new();
        let mut sessions = self.sessions.write().await;
        sessions.retain(|id, session| {
            if &session.initiator_id == device_id || &session.target_id == device_id {
                affected_sessions.push(id.clone());
                false
            } else {
                true
            }
        });

        let mut relays = self.relays.write().await;
        for id in &affected_sessions {
            relays.remove(id);
        }

        affected_sessions
    }

    /// Sends a message to a specific registered device.
    pub async fn send_to(&self, device_id: &DeviceId, msg: ServerMessage) -> bool {
        let map = self.devices.read().await;
        if let Some(dev) = map.get(device_id) {
            dev.sender.send(msg).is_ok()
        } else {
            false
        }
    }

    /// Checks if a device is currently online.
    pub async fn is_online(&self, device_id: &DeviceId) -> bool {
        self.devices.read().await.contains_key(device_id)
    }

    /// Creates an active session.
    #[allow(dead_code)]
    pub async fn create_session(
        &self,
        session_id: String,
        initiator_id: DeviceId,
        target_id: DeviceId,
        permissions: SessionPermissions,
    ) {
        let mut map = self.sessions.write().await;
        map.insert(
            session_id.clone(),
            ActiveSession {
                session_id,
                initiator_id,
                target_id,
                started_at: Utc::now(),
                permissions,
            },
        );
    }

    /// Retrieves an active session by ID.
    pub async fn get_session(&self, session_id: &str) -> Option<ActiveSession> {
        self.sessions.read().await.get(session_id).cloned()
    }

    /// Removes a session by ID.
    pub async fn remove_session(&self, session_id: &str) -> Option<ActiveSession> {
        self.relays.write().await.remove(session_id);
        self.sessions.write().await.remove(session_id)
    }

    /// Binds data relay channels for a session.
    #[allow(dead_code)]
    pub async fn bind_relay(
        &self,
        session_id: String,
        initiator_tx: UnboundedSender<String>,
        target_tx: UnboundedSender<String>,
    ) {
        self.relays.write().await.insert(session_id, (initiator_tx, target_tx));
    }

    /// Forwards a relayed data packet to the peer in the session.
    pub async fn relay_packet(&self, session_id: &str, is_initiator: bool, packet_json: String) -> bool {
        let relays = self.relays.read().await;
        if let Some((init_tx, target_tx)) = relays.get(session_id) {
            if is_initiator {
                target_tx.send(packet_json).is_ok()
            } else {
                init_tx.send(packet_json).is_ok()
            }
        } else {
            false
        }
    }

    /// Returns a snapshot of online device statuses for the admin console.
    pub async fn list_devices_summary(&self) -> Vec<serde_json::Value> {
        let map = self.devices.read().await;
        map.values()
            .map(|dev| {
                serde_json::json!({
                    "device_id": dev.device_id.formatted(),
                    "raw_id": dev.device_id.as_str(),
                    "peer_addr": dev.peer_addr,
                    "os_info": dev.os_info,
                    "app_version": dev.app_version,
                    "connected_at": dev.connected_at.to_rfc3339(),
                })
            })
            .collect()
    }

    /// Returns a snapshot of all active sessions for the admin console.
    pub async fn list_sessions(&self) -> Vec<ActiveSession> {
        self.sessions.read().await.values().cloned().collect()
    }
}
