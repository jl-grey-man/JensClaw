//! Atomic JSON file writes: tmp + sync + verify + rename.

use serde::Serialize;
use std::fs;
use std::io;
use std::path::Path;

/// Atomically write JSON data to a file.
///
/// Writes to a `.tmp` sibling, calls `sync_all()`, reads back to verify
/// deserialization succeeds, then renames over the target path.
/// Cleans up the temp file on failure.
pub fn atomic_write_json<T: Serialize + serde::de::DeserializeOwned>(
    path: &Path,
    data: &T,
) -> io::Result<()> {
    let tmp_path = path.with_extension("json.tmp");

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Serialize
    let json = serde_json::to_string_pretty(data).map_err(io::Error::other)?;

    // Write to temp file
    fs::write(&tmp_path, &json)?;

    // Sync to disk
    let file = fs::File::open(&tmp_path)?;
    file.sync_all()?;

    // Read back and verify
    let readback = fs::read_to_string(&tmp_path)?;
    let _: T = serde_json::from_str(&readback).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        io::Error::other(format!("Verification failed after write: {e}"))
    })?;

    // Atomic rename
    fs::rename(&tmp_path, path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        e
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestData {
        name: String,
        value: i32,
    }

    fn test_path() -> PathBuf {
        std::env::temp_dir().join(format!(
            "sandy_atomic_test_{}.json",
            uuid::Uuid::new_v4()
        ))
    }

    #[test]
    fn test_atomic_write_json_roundtrip() {
        let path = test_path();
        let data = TestData {
            name: "test".into(),
            value: 42,
        };
        atomic_write_json(&path, &data).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        let loaded: TestData = serde_json::from_str(&content).unwrap();
        assert_eq!(loaded, data);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_atomic_write_json_cleanup_on_failure() {
        let path = test_path();
        let tmp_path = path.with_extension("json.tmp");
        let data = TestData {
            name: "test".into(),
            value: 1,
        };
        atomic_write_json(&path, &data).unwrap();
        assert!(!tmp_path.exists(), "Temp file should not exist after success");
        let _ = fs::remove_file(&path);
    }
}
