use remotedesk_core::auth::SessionPermissions;
use remotedesk_core::identity::DeviceId;
use remotedesk_core::protocol::{
    ClientMessage, DirEntryInfo, FrameFormat, KeyModifiers, MonitorInfo, MouseButton,
    ServerMessage, SessionPacket,
};

#[test]
fn test_device_id_parsing_and_formatting() {
    let raw = "482910231";
    let dev_id = DeviceId::new(raw).expect("Valid 9-digit ID");
    assert_eq!(dev_id.as_str(), "482910231");
    assert_eq!(dev_id.formatted(), "482 910 231");

    // Test with spaces
    let with_spaces = "482 910 231".parse::<DeviceId>().expect("Parsed spaces");
    assert_eq!(with_spaces, dev_id);

    // Test with dashes
    let with_dashes = "482-910-231".parse::<DeviceId>().expect("Parsed dashes");
    assert_eq!(with_dashes, dev_id);

    // Invalid IDs must be rejected
    assert!(DeviceId::new("012345678").is_err(), "Cannot start with 0");
    assert!(DeviceId::new("1234").is_err(), "Too short");
    assert!(DeviceId::new("1234567890").is_err(), "Too long");
    assert!(DeviceId::new("abcdefghi").is_err(), "Non-digits");
}

#[test]
fn test_signaling_messages_roundtrip() {
    let client_id = DeviceId::new("111222333").unwrap();
    let target_id = DeviceId::new("444555666").unwrap();

    let reg_msg = ClientMessage::Register {
        device_id: client_id.clone(),
        token: Some("secret_token_123".to_string()),
        os_info: "Windows 11 Pro 64-bit".to_string(),
        app_version: "1.0.0".to_string(),
    };
    let json = serde_json::to_string(&reg_msg).unwrap();
    let decoded: ClientMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(reg_msg, decoded);

    let connect_req = ClientMessage::ConnectRequest {
        target_id: target_id.clone(),
        pin: Some("123456".to_string()),
        unattended_password: None,
        requested_permissions: SessionPermissions::full_control(),
    };
    let json = serde_json::to_string(&connect_req).unwrap();
    let decoded: ClientMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(connect_req, decoded);

    let incoming = ServerMessage::IncomingSession {
        session_id: "sess_test_123".to_string(),
        peer_id: client_id,
        peer_name: "Support Tech".to_string(),
        peer_ip: "192.168.1.50".to_string(),
        requested_permissions: SessionPermissions::view_only(),
    };
    let json = serde_json::to_string(&incoming).unwrap();
    let decoded: ServerMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(incoming, decoded);
}

#[test]
fn test_session_packets_multiplexing() {
    // 1. Screen Frame Packet
    let frame_packet = SessionPacket::ScreenFrame {
        frame_id: 100,
        timestamp_ms: 1729681200000,
        monitor_idx: 0,
        screen_w: 1920,
        screen_h: 1080,
        tile_x: 0,
        tile_y: 0,
        tile_w: 1920,
        tile_h: 1080,
        is_keyframe: true,
        format: FrameFormat::Jpeg,
        payload_base64: "base64encodedjpegdata".to_string(),
    };
    let json = serde_json::to_string(&frame_packet).unwrap();
    let decoded: SessionPacket = serde_json::from_str(&json).unwrap();
    assert_eq!(frame_packet, decoded);

    // 2. Mouse Move & Click Packets
    let mouse_move = SessionPacket::MouseMove {
        x: 500,
        y: 400,
        monitor_idx: 1,
    };
    assert_eq!(
        mouse_move,
        serde_json::from_str(&serde_json::to_string(&mouse_move).unwrap()).unwrap()
    );

    let mouse_click = SessionPacket::MouseDown {
        button: MouseButton::Right,
        x: 500,
        y: 400,
    };
    assert_eq!(
        mouse_click,
        serde_json::from_str(&serde_json::to_string(&mouse_click).unwrap()).unwrap()
    );

    // 3. Keyboard Packet
    let key_down = SessionPacket::KeyDown {
        key_code: 67, // 'C'
        scan_code: 46,
        modifiers: KeyModifiers {
            shift: false,
            ctrl: true,
            alt: false,
            meta: false,
        },
    };
    assert_eq!(
        key_down,
        serde_json::from_str(&serde_json::to_string(&key_down).unwrap()).unwrap()
    );

    // 4. Chat & Clipboard Packets
    let chat = SessionPacket::Chat {
        sender: "Tech".to_string(),
        message: "Hello, checking desktop!".to_string(),
        timestamp: 123456789,
    };
    assert_eq!(
        chat,
        serde_json::from_str(&serde_json::to_string(&chat).unwrap()).unwrap()
    );

    let clipboard = SessionPacket::ClipboardSync {
        text: "https://example.com/download".to_string(),
    };
    assert_eq!(
        clipboard,
        serde_json::from_str(&serde_json::to_string(&clipboard).unwrap()).unwrap()
    );
}

#[test]
fn test_monitor_info_and_dir_entry() {
    let monitor = MonitorInfo {
        index: 0,
        name: "Primary LG UltraWide".to_string(),
        width: 3440,
        height: 1440,
        is_primary: true,
        dpi_scale: 125,
    };
    let json = serde_json::to_string(&monitor).unwrap();
    let decoded: MonitorInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(monitor, decoded);

    let entry = DirEntryInfo {
        name: "document.pdf".to_string(),
        is_dir: false,
        size_bytes: 1048576,
        modified_timestamp: 1729680000,
    };
    let json = serde_json::to_string(&entry).unwrap();
    let decoded: DirEntryInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(entry, decoded);
}
