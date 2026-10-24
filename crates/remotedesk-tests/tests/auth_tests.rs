use remotedesk_core::auth::{
    BruteForceLimiter, SaltedHash, SessionPermissions, SessionPin, SessionToken,
};

#[test]
fn test_salted_password_hash_security() {
    let raw_pass = "VeryStrong#AdminPassword2026";
    let hash = SaltedHash::create(raw_pass);

    // Verify correct password matches
    assert!(hash.verify(raw_pass));

    // Prove wrong password is unequivocally rejected
    assert!(!hash.verify("WrongPassword"));
    assert!(!hash.verify("VeryStrong#AdminPassword2025"));
    assert!(!hash.verify(""));
    assert!(!hash.verify("verystrong#adminpassword2026")); // case sensitivity
}

#[test]
fn test_session_pin_verification() {
    let pin = SessionPin::generate();
    assert_eq!(pin.as_str().len(), 6);

    // Valid PIN verifies
    assert!(pin.verify(pin.as_str()));

    // Unauthorized PIN attempts must be rejected
    assert!(!pin.verify("000000"));
    assert!(!pin.verify("123456"));
    assert!(!pin.verify("999999"));
    assert!(!pin.verify(""));
}

#[test]
fn test_session_token_validation_and_expiration() {
    // Valid token
    let token = SessionToken::new(300); // 300s lifetime
    assert!(token.is_valid());

    // Expired token must be rejected
    let expired = SessionToken {
        token: "stale_token_abc123".to_string(),
        created_at: chrono::Utc::now() - chrono::Duration::seconds(400),
        expires_at: chrono::Utc::now() - chrono::Duration::seconds(100),
    };
    assert!(!expired.is_valid());
}

#[test]
fn test_unauthorized_access_rejection_and_rate_limiting() {
    // Max 3 failed attempts, 60s window, 120s lockout
    let limiter = BruteForceLimiter::new(3, 60, 120);
    let attacker_ip = "198.51.100.42";

    assert!(!limiter.is_locked(attacker_ip));

    // Attempt 1: Failed
    let locked = limiter.record_failure(attacker_ip);
    assert!(!locked);
    assert!(!limiter.is_locked(attacker_ip));

    // Attempt 2: Failed
    let locked = limiter.record_failure(attacker_ip);
    assert!(!locked);
    assert!(!limiter.is_locked(attacker_ip));

    // Attempt 3: Failed -> Must be locked out!
    let locked = limiter.record_failure(attacker_ip);
    assert!(locked);
    assert!(limiter.is_locked(attacker_ip), "Attacker IP must be locked out after 3 failures");

    // Any further request while locked is rejected
    assert!(limiter.is_locked(attacker_ip));

    // Valid reset unlocks
    limiter.reset(attacker_ip);
    assert!(!limiter.is_locked(attacker_ip));
}

#[test]
fn test_session_permission_scoping() {
    let view_only = SessionPermissions::view_only();
    assert!(view_only.allow_screen_view);
    assert!(!view_only.allow_mouse_keyboard);
    assert!(!view_only.allow_clipboard);
    assert!(!view_only.allow_file_transfer);

    let full = SessionPermissions::full_control();
    assert!(full.allow_screen_view);
    assert!(full.allow_mouse_keyboard);
    assert!(full.allow_clipboard);
    assert!(full.allow_file_transfer);
}
