use crate::auth::{SaltedHash, SessionPermissions};
use crate::identity::{get_machine_fingerprint, DeviceId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Local persistent configuration for RemoteDesk Free.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub device_id: DeviceId,
    pub device_alias: String,
    pub signaling_server: String,
    pub unattended_access_enabled: bool,
    pub unattended_password_hash: Option<SaltedHash>,
    pub default_permissions: SessionPermissions,
    pub dark_theme: bool,
    pub enable_relay: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        let fingerprint = get_machine_fingerprint();
        let device_id = DeviceId::from_fingerprint(&fingerprint);
        let hostname = std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_else(|_| "My Computer".to_string());

        Self {
            device_id,
            device_alias: hostname,
            signaling_server: "ws://127.0.0.1:8088/ws".to_string(),
            unattended_access_enabled: false,
            unattended_password_hash: None,
            default_permissions: SessionPermissions::default(),
            dark_theme: true,
            enable_relay: true,
        }
    }
}

/// Information about a recently connected remote peer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecentDevice {
    pub device_id: DeviceId,
    pub alias: String,
    pub last_seen: DateTime<Utc>,
    pub is_favorite: bool,
    pub group_tag: String,
}

/// Audit log record for security tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: u64,
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub peer_id: String,
    pub peer_ip: String,
    pub success: bool,
    pub details: String,
}

/// Persistent local database managing configuration, history, and audit records.
#[derive(Debug, Clone)]
pub struct LocalDatabase {
    config_path: PathBuf,
    history_path: PathBuf,
    audit_path: PathBuf,
    inner: Arc<Mutex<DatabaseInner>>,
}

#[derive(Debug)]
struct DatabaseInner {
    config: AppConfig,
    recent_devices: Vec<RecentDevice>,
    audit_logs: Vec<AuditLog>,
}

impl LocalDatabase {
    /// Initializes the database in the target directory (e.g. %APPDATA%/RemoteDeskFree).
    pub fn init(storage_dir: impl AsRef<Path>) -> Result<Self, std::io::Error> {
        let dir = storage_dir.as_ref();
        fs::create_dir_all(dir)?;

        let config_path = dir.join("config.json");
        let history_path = dir.join("recent.json");
        let audit_path = dir.join("audit.json");

        let config = if config_path.exists() {
            let data = fs::read_to_string(&config_path)?;
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            let def = AppConfig::default();
            let json = serde_json::to_string_pretty(&def)?;
            fs::write(&config_path, json)?;
            def
        };

        let recent_devices = if history_path.exists() {
            let data = fs::read_to_string(&history_path)?;
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            Vec::new()
        };

        let audit_logs = if audit_path.exists() {
            let data = fs::read_to_string(&audit_path)?;
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            Vec::new()
        };

        Ok(Self {
            config_path,
            history_path,
            audit_path,
            inner: Arc::new(Mutex::new(DatabaseInner {
                config,
                recent_devices,
                audit_logs,
            })),
        })
    }

    pub fn get_config(&self) -> AppConfig {
        self.inner.lock().unwrap().config.clone()
    }

    pub fn update_config<F>(&self, update_fn: F) -> Result<AppConfig, std::io::Error>
    where
        F: FnOnce(&mut AppConfig),
    {
        let mut inner = self.inner.lock().unwrap();
        update_fn(&mut inner.config);
        let updated = inner.config.clone();
        let json = serde_json::to_string_pretty(&updated)?;
        atomic_save(&self.config_path, &json)?;
        Ok(updated)
    }

    pub fn get_recent_devices(&self) -> Vec<RecentDevice> {
        self.inner.lock().unwrap().recent_devices.clone()
    }

    pub fn add_or_update_device(&self, mut device: RecentDevice) -> Result<(), std::io::Error> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(pos) = inner.recent_devices.iter().position(|d| d.device_id == device.device_id) {
            let existing = &inner.recent_devices[pos];
            if existing.is_favorite && !device.is_favorite {
                device.is_favorite = true;
                if device.alias.starts_with("Device ") && !existing.alias.is_empty() {
                    device.alias = existing.alias.clone();
                }
                if (device.group_tag == "General" || device.group_tag == "Recent") && !existing.group_tag.is_empty() {
                    device.group_tag = existing.group_tag.clone();
                }
            }
            inner.recent_devices[pos] = device;
        } else {
            inner.recent_devices.insert(0, device);
            if inner.recent_devices.len() > 100 {
                inner.recent_devices.truncate(100);
            }
        }
        let json = serde_json::to_string_pretty(&inner.recent_devices)?;
        atomic_save(&self.history_path, &json)?;
        Ok(())
    }

    pub fn remove_device(&self, device_id: &DeviceId) -> Result<(), std::io::Error> {
        let mut inner = self.inner.lock().unwrap();
        inner.recent_devices.retain(|d| &d.device_id != device_id);
        let json = serde_json::to_string_pretty(&inner.recent_devices)?;
        atomic_save(&self.history_path, &json)?;
        Ok(())
    }

    pub fn clear_recent_history(&self, keep_favorites: bool) -> Result<(), std::io::Error> {
        let mut inner = self.inner.lock().unwrap();
        if keep_favorites {
            inner.recent_devices.retain(|d| d.is_favorite);
        } else {
            inner.recent_devices.clear();
        }
        let json = serde_json::to_string_pretty(&inner.recent_devices)?;
        atomic_save(&self.history_path, &json)?;
        Ok(())
    }

    pub fn log_audit(
        &self,
        event_type: &str,
        peer_id: &str,
        peer_ip: &str,
        success: bool,
        details: &str,
    ) -> Result<(), std::io::Error> {
        let mut inner = self.inner.lock().unwrap();
        let next_id = inner.audit_logs.len() as u64 + 1;
        let entry = AuditLog {
            id: next_id,
            timestamp: Utc::now(),
            event_type: event_type.to_string(),
            peer_id: peer_id.to_string(),
            peer_ip: peer_ip.to_string(),
            success,
            details: details.to_string(),
        };
        inner.audit_logs.push(entry);
        if inner.audit_logs.len() > 1000 {
            inner.audit_logs.drain(0..100);
        }
        let json = serde_json::to_string_pretty(&inner.audit_logs)?;
        atomic_save(&self.audit_path, &json)?;
        Ok(())
    }

    pub fn get_audit_logs(&self) -> Vec<AuditLog> {
        self.inner.lock().unwrap().audit_logs.clone()
    }
}

/// Atomically writes content to path via temporary file replace.
fn atomic_save(path: &Path, content: &str) -> Result<(), std::io::Error> {
    let tmp_path = path.with_extension("tmp");
    {
        let mut file = File::create(&tmp_path)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
    }
    fs::rename(tmp_path, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_database_persistence() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db = LocalDatabase::init(temp_dir.path()).unwrap();

        let cfg = db.get_config();
        assert_eq!(cfg.signaling_server, "ws://127.0.0.1:8088/ws");

        // Update config
        db.update_config(|c| {
            c.unattended_access_enabled = true;
            c.device_alias = "TestMachine".to_string();
        })
        .unwrap();

        assert!(db.get_config().unattended_access_enabled);
        assert_eq!(db.get_config().device_alias, "TestMachine");

        // Add recent device
        let dev = RecentDevice {
            device_id: DeviceId::new("987654321").unwrap(),
            alias: "Work Laptop".to_string(),
            last_seen: Utc::now(),
            is_favorite: true,
            group_tag: "Work".to_string(),
        };
        db.add_or_update_device(dev.clone()).unwrap();
        let devices = db.get_recent_devices();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].device_id, dev.device_id);

        // Audit log
        db.log_audit("LOGIN", "987654321", "127.0.0.1", true, "Session started").unwrap();
        let logs = db.get_audit_logs();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].event_type, "LOGIN");

        // Re-open from same dir to verify disk persistence
        let db2 = LocalDatabase::init(temp_dir.path()).unwrap();
        assert_eq!(db2.get_config().device_alias, "TestMachine");
        assert_eq!(db2.get_recent_devices().len(), 1);
        assert_eq!(db2.get_audit_logs().len(), 1);

        // Add non-favorite device
        let dev2 = RecentDevice {
            device_id: DeviceId::new("123456789").unwrap(),
            alias: "Test PC".to_string(),
            last_seen: Utc::now(),
            is_favorite: false,
            group_tag: "General".to_string(),
        };
        db2.add_or_update_device(dev2.clone()).unwrap();
        assert_eq!(db2.get_recent_devices().len(), 2);

        // Clear non-favorites
        db2.clear_recent_history(true).unwrap();
        let remaining = db2.get_recent_devices();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].device_id, dev.device_id);

        // Remove favorite device
        db2.remove_device(&dev.device_id).unwrap();
        assert_eq!(db2.get_recent_devices().len(), 0);
    }
}
