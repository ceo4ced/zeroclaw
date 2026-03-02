//! File-based persistence for billing state.
//!
//! Stores prepaid balance and daily usage to JSON files in the workspace
//! directory. Simple and atomic (write-to-temp + rename).
//!
//! # Storage Layout
//!
//! ```text
//! <workspace_dir>/billing/
//!   └── <user_id>.json   — balance and usage state
//! ```

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Persisted billing state for a single user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedBalance {
    pub user_id: String,
    pub balance_cents: i64,
    /// ISO 8601 timestamp of last update.
    pub updated_at: String,
}

/// Get the billing directory path for a workspace.
fn billing_dir(workspace_dir: &Path) -> PathBuf {
    workspace_dir.join("billing")
}

/// Get the balance file path for a user.
fn balance_file(workspace_dir: &Path, user_id: &str) -> PathBuf {
    billing_dir(workspace_dir).join(format!("{user_id}.json"))
}

/// Save balance to disk. Creates the billing directory if needed.
///
/// Uses write-to-temp + rename for atomicity on most filesystems.
pub fn save_balance(workspace_dir: &Path, user_id: &str, balance_cents: i64) -> anyhow::Result<()> {
    let dir = billing_dir(workspace_dir);
    std::fs::create_dir_all(&dir)?;

    let state = PersistedBalance {
        user_id: user_id.to_string(),
        balance_cents,
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    let json = serde_json::to_string_pretty(&state)?;
    let target = balance_file(workspace_dir, user_id);
    let tmp = target.with_extension("json.tmp");

    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, &target)?;

    Ok(())
}

/// Load persisted balance from disk.
///
/// Returns an error if the file doesn't exist or can't be parsed.
pub fn load_balance(workspace_dir: &Path, user_id: &str) -> anyhow::Result<PersistedBalance> {
    let path = balance_file(workspace_dir, user_id);
    let contents = std::fs::read_to_string(&path)?;
    let state: PersistedBalance = serde_json::from_str(&contents)?;
    Ok(state)
}

/// Check if a persisted balance exists for a user.
pub fn has_balance(workspace_dir: &Path, user_id: &str) -> bool {
    balance_file(workspace_dir, user_id).exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn save_and_load_roundtrip() {
        let tmp = TempDir::new().unwrap();
        save_balance(tmp.path(), "user_a", 1500).unwrap();

        let loaded = load_balance(tmp.path(), "user_a").unwrap();
        assert_eq!(loaded.user_id, "user_a");
        assert_eq!(loaded.balance_cents, 1500);
        assert!(!loaded.updated_at.is_empty());
    }

    #[test]
    fn save_negative_balance() {
        let tmp = TempDir::new().unwrap();
        save_balance(tmp.path(), "user_a", -150).unwrap();

        let loaded = load_balance(tmp.path(), "user_a").unwrap();
        assert_eq!(loaded.balance_cents, -150);
    }

    #[test]
    fn load_nonexistent_returns_error() {
        let tmp = TempDir::new().unwrap();
        let result = load_balance(tmp.path(), "nobody");
        assert!(result.is_err());
    }

    #[test]
    fn has_balance_check() {
        let tmp = TempDir::new().unwrap();
        assert!(!has_balance(tmp.path(), "user_a"));

        save_balance(tmp.path(), "user_a", 100).unwrap();
        assert!(has_balance(tmp.path(), "user_a"));
    }

    #[test]
    fn save_overwrites_atomically() {
        let tmp = TempDir::new().unwrap();
        save_balance(tmp.path(), "user_a", 1000).unwrap();
        save_balance(tmp.path(), "user_a", 500).unwrap();

        let loaded = load_balance(tmp.path(), "user_a").unwrap();
        assert_eq!(loaded.balance_cents, 500);
    }

    #[test]
    fn creates_billing_directory() {
        let tmp = TempDir::new().unwrap();
        let dir = billing_dir(tmp.path());
        assert!(!dir.exists());

        save_balance(tmp.path(), "user_a", 100).unwrap();
        assert!(dir.exists());
    }
}
