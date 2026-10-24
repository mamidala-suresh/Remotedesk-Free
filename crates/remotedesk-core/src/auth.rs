use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Mutex;

/// One-time session PIN (6 digits).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionPin(String);

impl SessionPin {
    pub fn generate() -> Self {
        let mut rng = rand::thread_rng();
        let pin: u32 = rng.gen_range(100_000..=999_999);
        SessionPin(pin.to_string())
    }

    pub fn new(pin: impl Into<String>) -> Result<Self, &'static str> {
        let clean = pin.into().trim().to_string();
        if clean.len() < 4 || clean.len() > 16 {
            return Err("PIN must be between 4 and 16 characters");
        }
        Ok(SessionPin(clean))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn verify(&self, candidate: &str) -> bool {
        constant_time_eq(self.0.as_bytes(), candidate.trim().as_bytes())
    }
}

/// Salted cryptographic password hash for unattended access.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SaltedHash {
    pub salt_hex: String,
    pub hash_hex: String,
}

impl SaltedHash {
    pub fn create(password: &str) -> Self {
        let mut salt_bytes = [0u8; 32];
        rand::thread_rng().fill(&mut salt_bytes);
        let salt_hex = hex::encode(salt_bytes);

        let hash_hex = Self::compute_hash(password, &salt_bytes);
        SaltedHash { salt_hex, hash_hex }
    }

    pub fn verify(&self, password: &str) -> bool {
        let salt_bytes = match hex::decode(&self.salt_hex) {
            Ok(b) => b,
            Err(_) => return false,
        };
        let computed = Self::compute_hash(password, &salt_bytes);
        constant_time_eq(self.hash_hex.as_bytes(), computed.as_bytes())
    }

    fn compute_hash(password: &str, salt: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(salt);
        hasher.update(password.as_bytes());
        // Run multiple rounds (iterated PBKDF2-style SHA-256)
        let mut result = hasher.finalize();
        for _ in 0..10_000 {
            let mut round = Sha256::new();
            round.update(&result);
            round.update(salt);
            result = round.finalize();
        }
        hex::encode(result)
    }
}

/// Permissions granted for an active remote session.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionPermissions {
    pub allow_screen_view: bool,
    pub allow_mouse_keyboard: bool,
    pub allow_clipboard: bool,
    pub allow_file_transfer: bool,
    pub allow_audio: bool,
}

impl Default for SessionPermissions {
    fn default() -> Self {
        Self {
            allow_screen_view: true,
            allow_mouse_keyboard: true,
            allow_clipboard: true,
            allow_file_transfer: true,
            allow_audio: false,
        }
    }
}

impl SessionPermissions {
    pub fn view_only() -> Self {
        Self {
            allow_screen_view: true,
            allow_mouse_keyboard: false,
            allow_clipboard: false,
            allow_file_transfer: false,
            allow_audio: false,
        }
    }

    pub fn full_control() -> Self {
        Self::default()
    }
}

/// Cryptographically random session authorization token.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionToken {
    pub token: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl SessionToken {
    pub fn new(lifetime_seconds: i64) -> Self {
        let mut random_bytes = [0u8; 32];
        rand::thread_rng().fill(&mut random_bytes);
        let token = hex::encode(random_bytes);
        let now = Utc::now();
        let expires_at = now + Duration::seconds(lifetime_seconds);

        SessionToken {
            token,
            created_at: now,
            expires_at,
        }
    }

    pub fn is_valid(&self) -> bool {
        Utc::now() <= self.expires_at
    }
}

/// Constant-time byte slice comparison to prevent timing side channels.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut res = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        res |= x ^ y;
    }
    res == 0
}

/// In-memory rate limiter to protect against brute-force attacks on connection PINs/passwords.
#[derive(Debug)]
pub struct BruteForceLimiter {
    attempts: Mutex<HashMap<String, Vec<DateTime<Utc>>>>,
    max_attempts: usize,
    window_seconds: i64,
    lockout_seconds: i64,
}

impl BruteForceLimiter {
    pub fn new(max_attempts: usize, window_seconds: i64, lockout_seconds: i64) -> Self {
        Self {
            attempts: Mutex::new(HashMap::new()),
            max_attempts,
            window_seconds,
            lockout_seconds,
        }
    }

    /// Checks if a client identifier is currently locked out.
    pub fn is_locked(&self, id: &str) -> bool {
        let now = Utc::now();
        let mut map = self.attempts.lock().unwrap();
        if let Some(history) = map.get_mut(id) {
            // Keep entries within window + lockout
            history.retain(|t| *t >= now - Duration::seconds(self.window_seconds + self.lockout_seconds));
            if history.len() >= self.max_attempts {
                if let Some(last) = history.last() {
                    if *last >= now - Duration::seconds(self.lockout_seconds) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Records a failed attempt. Returns true if this attempt triggered a lockout.
    pub fn record_failure(&self, id: &str) -> bool {
        let now = Utc::now();
        let mut map = self.attempts.lock().unwrap();
        let history = map.entry(id.to_string()).or_insert_with(Vec::new);
        history.push(now);
        history.len() >= self.max_attempts
    }

    /// Clears failed attempts upon successful authentication.
    pub fn reset(&self, id: &str) {
        let mut map = self.attempts.lock().unwrap();
        map.remove(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_pin_generate_and_verify() {
        let pin = SessionPin::generate();
        assert_eq!(pin.as_str().len(), 6);
        assert!(pin.verify(pin.as_str()));
        assert!(!pin.verify("000000"));
    }

    #[test]
    fn test_salted_hash_verification() {
        let pwd = "SuperSecretPassword123!";
        let hash = SaltedHash::create(pwd);
        assert!(hash.verify(pwd));
        assert!(!hash.verify("WrongPassword"));
        assert!(!hash.verify("supersecretpassword123!"));
    }

    #[test]
    fn test_session_token_expiry() {
        let token = SessionToken::new(3600);
        assert!(token.is_valid());

        let expired_token = SessionToken {
            token: "expired".to_string(),
            created_at: Utc::now() - Duration::seconds(100),
            expires_at: Utc::now() - Duration::seconds(10),
        };
        assert!(!expired_token.is_valid());
    }

    #[test]
    fn test_brute_force_limiter() {
        let limiter = BruteForceLimiter::new(3, 60, 60);
        let id = "192.168.1.100";
        assert!(!limiter.is_locked(id));

        assert!(!limiter.record_failure(id)); // attempt 1
        assert!(!limiter.record_failure(id)); // attempt 2
        assert!(limiter.record_failure(id));  // attempt 3 -> locked

        assert!(limiter.is_locked(id));
        limiter.reset(id);
        assert!(!limiter.is_locked(id));
    }
}
