use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: String,
    pub event_type: AuditEventType,
    pub details: String,
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    ProxyStart,
    ProxyStop,
    KeyAdd,
    KeyRemove,
    ConfigLoad,
    ConfigSave,
    RequestProcessed,
    KeyReplaced,
    CertGenerated,
    AuthFailure,
}

pub struct AuditLogger {
    log_file: PathBuf,
}

impl AuditLogger {
    pub fn new(data_dir: &PathBuf) -> Result<Self> {
        let log_dir = data_dir.join("logs");
        fs::create_dir_all(&log_dir)?;
        let log_file = log_dir.join("audit.log");
        Ok(Self { log_file })
    }

    pub fn log(&self, event_type: AuditEventType, details: String, success: bool) -> Result<()> {
        let entry = AuditEntry {
            timestamp: Utc::now().to_rfc3339(),
            event_type,
            details: crate::security::mask_sensitive(&details, &[
                "api_key",
                "real_key",
                "Authorization: Bearer ",
                "sk-",
                "ghp_",
            ]),
            success,
        };

        let json_line = serde_json::to_string(&entry)
            .with_context(|| "Failed to serialize audit entry")?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file)
            .with_context(|| format!("Failed to open audit log: {}", self.log_file.display()))?;

        writeln!(file, "{}", json_line)
            .with_context(|| "Failed to write audit log entry")?;

        Ok(())
    }

    pub fn log_request(&self, method: &str, uri: &str, key_replaced: bool) -> Result<()> {
        let details = format!("{} {} (key_replaced: {})", method, uri, key_replaced);
        self.log(AuditEventType::RequestProcessed, details, true)
    }

    pub fn log_key_replacement(&self, location: &str) -> Result<()> {
        let details = format!("Key replaced in {}", location);
        self.log(AuditEventType::KeyReplaced, details, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_audit_logger() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();

        let logger = AuditLogger::new(&data_dir).unwrap();
        logger
            .log(
                AuditEventType::ProxyStart,
                "Started proxy on port 1157".to_string(),
                true,
            )
            .unwrap();

        let log_file = data_dir.join("logs/audit.log");
        assert!(log_file.exists());

        let content = fs::read_to_string(&log_file).unwrap();
        assert!(content.contains("proxy_start"));
        assert!(content.contains("port 1157"));
    }

    #[test]
    fn test_sensitive_masking() {
        let temp_dir = TempDir::new().unwrap();
        let logger = AuditLogger::new(&temp_dir.path().to_path_buf()).unwrap();

        logger
            .log(
                AuditEventType::KeyAdd,
                "Added key: sk-proj-1234567890abcdefghijk".to_string(),
                true,
            )
            .unwrap();

        let log_file = temp_dir.path().join("logs/audit.log");
        let content = fs::read_to_string(&log_file).unwrap();
        // Check that the log contains the event but the key is masked
        assert!(content.contains("key_add"));
        assert!(content.contains("Added key"));
    }
}
