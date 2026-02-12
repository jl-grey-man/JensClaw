//! Hardened File Operations Module
//!
//! Provides safe file operations with:
//! - Strict path validation (prevents directory traversal)
//! - Atomic writes (temp → verify → rename)
//! - Comprehensive error handling
//!
//! All paths must be within allowed roots:
//! - /storage/
//! - /mnt/storage/
//! - /tmp/
//! - Configured working directory

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Allowed root directories for file operations
const ALLOWED_ROOTS: &[&str] = &["/storage", "/mnt/storage", "/tmp"];

/// Errors that can occur during file operations
#[derive(Debug)]
pub enum FileOpsError {
    PathNotAllowed(String),
    IoError(std::io::Error),
    VerificationFailed(String),
    InvalidPath(String),
}

impl std::fmt::Display for FileOpsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileOpsError::PathNotAllowed(path) => {
                write!(f, "Access denied: {} is outside allowed directories", path)
            }
            FileOpsError::IoError(e) => write!(f, "IO error: {}", e),
            FileOpsError::VerificationFailed(msg) => write!(f, "Verification failed: {}", msg),
            FileOpsError::InvalidPath(msg) => write!(f, "Invalid path: {}", msg),
        }
    }
}

impl std::error::Error for FileOpsError {}

impl From<std::io::Error> for FileOpsError {
    fn from(error: std::io::Error) -> Self {
        FileOpsError::IoError(error)
    }
}

/// Validate that a path is within allowed directories
///
/// # Arguments
/// * `path` - The path to validate
///
/// # Returns
/// * `Ok(())` if path is allowed
/// * `Err(FileOpsError::PathNotAllowed)` if path is outside allowed roots
///
/// # Examples
/// ```
/// validate_path("/storage/test.txt").unwrap(); // OK
/// validate_path("/etc/passwd").unwrap_err();   // Not allowed
/// validate_path("/storage/../../../etc/passwd").unwrap_err(); // Blocked
/// ```
pub fn validate_path<P: AsRef<Path>>(path: P) -> Result<(), FileOpsError> {
    let path = path.as_ref();

    // Get absolute path
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| FileOpsError::IoError(e))?
            .join(path)
    };

    // Canonicalize to resolve symlinks and normalize
    let canonical = match absolute.canonicalize() {
        Ok(c) => c,
        Err(_) => {
            // Path doesn't exist yet, check parent
            if let Some(parent) = absolute.parent() {
                let canonical_parent = parent.canonicalize()?;
                canonical_parent.join(absolute.file_name().unwrap_or_default())
            } else {
                absolute
            }
        }
    };

    // Check against allowed roots
    let allowed = ALLOWED_ROOTS.iter().any(|root| {
        let root_path = Path::new(root);
        canonical.starts_with(root_path)
    });

    if !allowed {
        return Err(FileOpsError::PathNotAllowed(
            canonical.to_string_lossy().to_string(),
        ));
    }

    Ok(())
}

/// Read file contents with path validation
///
/// # Arguments
/// * `path` - Path to file to read
///
/// # Returns
/// * `Ok(String)` with file contents
/// * `Err(FileOpsError)` if path invalid or read fails
pub fn read_file<P: AsRef<Path>>(path: P) -> Result<String, FileOpsError> {
    validate_path(&path)?;

    let content = fs::read_to_string(&path)?;
    Ok(content)
}

/// Write file atomically with path validation
///
/// Writes to a temp file first, verifies the write, then renames atomically.
/// This prevents partial file corruption if the process crashes.
///
/// # Arguments
/// * `path` - Destination file path
/// * `content` - Content to write
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(FileOpsError)` if path invalid, write fails, or verification fails
///
/// # Example
/// ```
/// write_file("/storage/test.txt", "Hello World").unwrap();
/// ```
pub fn write_file<P: AsRef<Path>>(path: P, content: &str) -> Result<(), FileOpsError> {
    validate_path(&path)?;

    let path = path.as_ref();

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Create temp file in same directory (for atomic rename)
    let temp_path = path.with_extension("tmp");

    // Write to temp file
    {
        let mut file = fs::File::create(&temp_path)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?; // Ensure data is written to disk
    }

    // Verify temp file was written correctly
    let written_content = fs::read_to_string(&temp_path)?;
    if written_content != content {
        fs::remove_file(&temp_path).ok();
        return Err(FileOpsError::VerificationFailed(
            "Written content does not match expected content".to_string(),
        ));
    }

    // Atomic rename
    fs::rename(&temp_path, path)?;

    Ok(())
}

/// Verify that a file exists and is readable
///
/// # Arguments
/// * `path` - Path to check
///
/// # Returns
/// * `Ok(true)` if file exists, is readable, and size > 0
/// * `Ok(false)` if file doesn't exist or is empty
/// * `Err(FileOpsError)` if path invalid or other error
pub fn verify_file_exists<P: AsRef<Path>>(path: P) -> Result<bool, FileOpsError> {
    validate_path(&path)?;

    match fs::metadata(&path) {
        Ok(metadata) => Ok(metadata.is_file() && metadata.len() > 0),
        Err(_) => Ok(false),
    }
}

/// List directory contents with path validation
///
/// # Arguments
/// * `path` - Directory path to list
///
/// # Returns
/// * `Ok(Vec<String>)` with file and directory names
/// * `Err(FileOpsError)` if path invalid or not a directory
pub fn list_directory<P: AsRef<Path>>(path: P) -> Result<Vec<String>, FileOpsError> {
    validate_path(&path)?;

    let entries = fs::read_dir(&path)?;
    let mut names = Vec::new();

    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        names.push(name.to_string_lossy().to_string());
    }

    Ok(names)
}

/// Create a job folder with unique ID
///
/// Creates the directory structure: storage/tasks/job_{id}/
///
/// # Arguments
/// * `job_id` - Unique job identifier
/// * `storage_root` - Root storage directory (default: ./storage)
///
/// # Returns
/// * `Ok(PathBuf)` with full path to job folder
/// * `Err(FileOpsError)` if creation fails
pub fn create_job_folder(
    job_id: &str,
    storage_root: Option<&Path>,
) -> Result<PathBuf, FileOpsError> {
    let root = storage_root.unwrap_or_else(|| Path::new("./storage"));
    let job_path = root.join("tasks").join(format!("job_{}", job_id));

    validate_path(&job_path)?;
    fs::create_dir_all(&job_path)?;

    Ok(job_path)
}

/// Get file metadata with path validation
///
/// # Arguments
/// * `path` - File path
///
/// # Returns
/// * `Ok(fs::Metadata)` with file metadata
/// * `Err(FileOpsError)` if path invalid or file doesn't exist
pub fn get_file_metadata<P: AsRef<Path>>(path: P) -> Result<fs::Metadata, FileOpsError> {
    validate_path(&path)?;
    Ok(fs::metadata(&path)?)
}

/// Safely join paths without traversing outside allowed roots
///
/// # Arguments
/// * `base` - Base path (must be allowed)
/// * `relative` - Relative path to join
///
/// # Returns
/// * `Ok(PathBuf)` with joined path if result is still within allowed roots
/// * `Err(FileOpsError)` if joined path would escape allowed roots
pub fn safe_join<P: AsRef<Path>, Q: AsRef<Path>>(
    base: P,
    relative: Q,
) -> Result<PathBuf, FileOpsError> {
    let base = base.as_ref();
    let relative = relative.as_ref();

    // Validate base path
    validate_path(base)?;

    // Join paths
    let joined = base.join(relative);

    // Validate joined path
    validate_path(&joined)?;

    Ok(joined)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_path_allowed() {
        // These should succeed
        assert!(validate_path("/storage/test.txt").is_ok());
        assert!(validate_path("/mnt/storage/file.md").is_ok());
        assert!(validate_path("/tmp/tempfile").is_ok());
    }

    #[test]
    fn test_validate_path_not_allowed() {
        // These should fail
        assert!(validate_path("/etc/passwd").is_err());
        assert!(validate_path("/root/secret").is_err());
        assert!(validate_path("/home/user/.ssh/id_rsa").is_err());
    }

    #[test]
    fn test_validate_path_traversal_attack() {
        // Directory traversal attacks should be blocked
        assert!(validate_path("/storage/../../../etc/passwd").is_err());
        assert!(validate_path("/mnt/storage/../../etc/shadow").is_err());
    }

    #[test]
    fn test_safe_join() {
        let base = Path::new("/storage");

        // Valid join
        let result = safe_join(base, "test/file.txt").unwrap();
        assert_eq!(result, Path::new("/storage/test/file.txt"));

        // Invalid join (escapes root)
        assert!(safe_join(base, "../../../etc/passwd").is_err());
    }
}
