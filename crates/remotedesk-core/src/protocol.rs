use crate::auth::SessionPermissions;
use crate::identity::DeviceId;
use serde::{Deserialize, Serialize};

/// Supported image formats for frame streaming.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameFormat {
    Jpeg,
    Png,
    RawRgba,
}

/// Mouse buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Active keyboard modifier flags.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyModifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool, // Windows/Command key
}

/// Information about a connected display monitor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonitorInfo {
    pub index: u32,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub is_primary: bool,
    pub dpi_scale: u32,
}

/// Information about a remote file or folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirEntryInfo {
    pub name: String,
    pub is_dir: bool,
    pub size_bytes: u64,
    pub modified_timestamp: i64,
}

/// Messages sent from Client to Signaling Server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "payload")]
pub enum ClientMessage {
    Register {
        device_id: DeviceId,
        token: Option<String>,
        os_info: String,
        app_version: String,
    },
    ConnectRequest {
        target_id: DeviceId,
        pin: Option<String>,
        unattended_password: Option<String>,
        requested_permissions: SessionPermissions,
    },
    SessionResponse {
        session_id: String,
        accepted: bool,
        reason: Option<String>,
        granted_permissions: Option<SessionPermissions>,
    },
    Disconnect {
        session_id: String,
        reason: Option<String>,
    },
    IceCandidate {
        session_id: String,
        candidate: String,
    },
    SessionOffer {
        session_id: String,
        sdp: String,
    },
    SessionAnswer {
        session_id: String,
        sdp: String,
    },
    Ping,
}

/// Messages sent from Signaling Server to Client.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "payload")]
pub enum ServerMessage {
    Registered {
        success: bool,
        message: String,
        assigned_id: DeviceId,
    },
    IncomingSession {
        session_id: String,
        peer_id: DeviceId,
        peer_name: String,
        peer_ip: String,
        requested_permissions: SessionPermissions,
    },
    SessionEstablished {
        session_id: String,
        peer_id: DeviceId,
        permissions: SessionPermissions,
        relay_url: Option<String>,
    },
    SessionRejected {
        session_id: String,
        reason: String,
    },
    SessionClosed {
        session_id: String,
        reason: String,
    },
    PeerIceCandidate {
        session_id: String,
        candidate: String,
    },
    PeerOffer {
        session_id: String,
        sdp: String,
    },
    PeerAnswer {
        session_id: String,
        sdp: String,
    },
    Pong,
    Error {
        code: String,
        message: String,
    },
}

/// Data channel packet types for live session multiplexing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "channel", content = "data")]
pub enum SessionPacket {
    // Video streaming
    ScreenFrame {
        frame_id: u64,
        timestamp_ms: u64,
        monitor_idx: u32,
        screen_w: u32,
        screen_h: u32,
        tile_x: u32,
        tile_y: u32,
        tile_w: u32,
        tile_h: u32,
        is_keyframe: bool,
        format: FrameFormat,
        payload_base64: String,
    },
    RequestKeyframe,
    // Input control
    MouseMove {
        x: u32,
        y: u32,
        monitor_idx: u32,
    },
    MouseDown {
        button: MouseButton,
        x: u32,
        y: u32,
    },
    MouseUp {
        button: MouseButton,
        x: u32,
        y: u32,
    },
    MouseWheel {
        delta_x: i32,
        delta_y: i32,
    },
    KeyDown {
        key_code: u32,
        scan_code: u32,
        modifiers: KeyModifiers,
    },
    KeyUp {
        key_code: u32,
        scan_code: u32,
        modifiers: KeyModifiers,
    },
    TextInput {
        text: String,
    },
    // Dynamic cursor shape synchronization
    CursorShape {
        shape: String,
    },
    // Clipboard sync
    ClipboardSync {
        text: String,
    },
    // Session Chat
    Chat {
        sender: String,
        message: String,
        timestamp: i64,
    },
    // Stream configuration
    QualityConfig {
        max_fps: u32,
        jpeg_quality: u8,
        scale_percent: u8,
    },
    MonitorSelect {
        monitor_idx: u32,
    },
    MonitorListResponse {
        monitors: Vec<MonitorInfo>,
    },
    // File Transfer
    FileOffer {
        transfer_id: String,
        filename: String,
        size_bytes: u64,
        sha256: String,
        is_dir: bool,
    },
    FileAccept {
        transfer_id: String,
        destination_path: String,
    },
    FileReject {
        transfer_id: String,
        reason: String,
    },
    FileChunk {
        transfer_id: String,
        chunk_idx: u64,
        total_chunks: u64,
        data_base64: String,
    },
    FileAck {
        transfer_id: String,
        chunk_idx: u64,
    },
    FileFinished {
        transfer_id: String,
        success: bool,
    },
    FileListRequest {
        path: String,
    },
    FileListResponse {
        path: String,
        entries: Vec<DirEntryInfo>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signaling_serde() {
        let msg = ClientMessage::ConnectRequest {
            target_id: DeviceId::new("123456789").unwrap(),
            pin: Some("482910".to_string()),
            unattended_password: None,
            requested_permissions: SessionPermissions::full_control(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ClientMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, parsed);
    }

    #[test]
    fn test_session_packet_serde() {
        let packet = SessionPacket::ScreenFrame {
            frame_id: 42,
            timestamp_ms: 1234567,
            monitor_idx: 0,
            screen_w: 1920,
            screen_h: 1080,
            tile_x: 0,
            tile_y: 0,
            tile_w: 1920,
            tile_h: 1080,
            is_keyframe: true,
            format: FrameFormat::Jpeg,
            payload_base64: "dGVzdGZyYW1l".to_string(),
        };

        let json = serde_json::to_string(&packet).unwrap();
        let decoded: SessionPacket = serde_json::from_str(&json).unwrap();
        assert_eq!(packet, decoded);

        let req_keyframe = SessionPacket::RequestKeyframe;
        let rk_json = serde_json::to_string(&req_keyframe).unwrap();
        let rk_decoded: SessionPacket = serde_json::from_str(&rk_json).unwrap();
        assert_eq!(req_keyframe, rk_decoded);
    }
}
