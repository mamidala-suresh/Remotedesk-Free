use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::str::FromStr;

/// Represents a 9-digit unique Device ID (formatted as `123 456 789`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceId(String);

impl DeviceId {
    /// Creates a DeviceId from raw 9-digit string.
    pub fn new(raw: impl Into<String>) -> Result<Self, String> {
        let clean = raw.into().chars().filter(|c| c.is_ascii_digit()).collect::<String>();
        if clean.len() != 9 {
            return Err(format!("Device ID must be exactly 9 digits, got '{}' (len {})", clean, clean.len()));
        }
        if clean.starts_with('0') {
            return Err("Device ID must not start with 0".to_string());
        }
        Ok(DeviceId(clean))
    }

    /// Generates a randomized 9-digit Device ID.
    pub fn generate_random() -> Self {
        let mut rng = rand::thread_rng();
        let number: u32 = rng.gen_range(100_000_000..=999_999_999);
        DeviceId(number.to_string())
    }

    /// Generates a deterministic Device ID from a machine hardware fingerprint.
    pub fn from_fingerprint(fingerprint: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(fingerprint.as_bytes());
        let hash = hasher.finalize();
        let mut val: u64 = 0;
        for byte in &hash[0..8] {
            val = (val << 8) | (*byte as u64);
        }
        let id_num = 100_000_000 + (val % 900_000_000);
        DeviceId(id_num.to_string())
    }

    /// Returns the raw 9-digit string (e.g. "123456789").
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns human-readable formatted ID with spaces (e.g. "123 456 789").
    pub fn formatted(&self) -> String {
        format!("{} {} {}", &self.0[0..3], &self.0[3..6], &self.0[6..9])
    }

    /// Validates a raw or formatted string.
    pub fn validate(input: &str) -> bool {
        Self::from_str(input).is_ok()
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.formatted())
    }
}

impl FromStr for DeviceId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        DeviceId::new(s)
    }
}

/// Generates a stable hardware fingerprint for the current Windows host.
pub fn get_machine_fingerprint() -> String {
    let hostname = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "UnknownHost".to_string());
    let user = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "UnknownUser".to_string());
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    format!("{}:{}:{}:{}", hostname, user, os, arch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_id_validation() {
        let id = DeviceId::new("123456789").unwrap();
        assert_eq!(id.as_str(), "123456789");
        assert_eq!(id.formatted(), "123 456 789");

        // Parse with spaces or dashes
        let parsed = DeviceId::from_str("123 456 789").unwrap();
        assert_eq!(parsed, id);

        let parsed_dash = DeviceId::from_str("123-456-789").unwrap();
        assert_eq!(parsed_dash, id);

        // Invalid lengths or starting with 0
        assert!(DeviceId::new("012345678").is_err());
        assert!(DeviceId::new("12345").is_err());
        assert!(DeviceId::new("1234567890").is_err());
    }

    #[test]
    fn test_random_device_id() {
        let id = DeviceId::generate_random();
        assert_eq!(id.as_str().len(), 9);
        assert!(!id.as_str().starts_with('0'));
    }

    #[test]
    fn test_deterministic_fingerprint() {
        let fp = "DESKTOP-ABC:Admin:windows:x86_64";
        let id1 = DeviceId::from_fingerprint(fp);
        let id2 = DeviceId::from_fingerprint(fp);
        assert_eq!(id1, id2);
        assert_eq!(id1.as_str().len(), 9);
    }
}
