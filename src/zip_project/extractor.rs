//! ZIP extraction with security validation.

use sha2::{Digest, Sha256};
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use zip::ZipArchive;

use super::tree::{self, FileInfo, ProjectTree};
use super::{IGNORE_DIRS, MAX_ARCHIVE_SIZE, MAX_FILE_SIZE};

#[derive(Debug, thiserror::Error)]
pub enum ZipError {
    #[error("Invalid ZIP archive: {0}")]
    InvalidArchive(String),

    #[error("Extraction failed: {0}")]
    ExtractionFailed(String),

    #[error("Path traversal attempt blocked: {0}")]
    PathTraversalAttempt(String),

    #[error("File too large: {0} bytes (max: {1})")]
    FileTooLarge(u64, u64),

    #[error("Archive too large: {0} bytes (max: {1})")]
    ArchiveTooLarge(u64, u64),

    #[error("Invalid encoding in file path")]
    InvalidEncoding,

    #[error("IO error: {0}")]
    IoError(String),
}

#[derive(Debug, Clone)]
pub struct ExtractedProject {
    pub root_path: PathBuf,
    pub files: Vec<FileInfo>,
    pub structure: ProjectTree,
}

fn should_ignore(path: &str) -> bool {
    let path_lower = path.to_lowercase();

    for dir in IGNORE_DIRS {
        if path_lower.contains(&dir.to_lowercase()) {
            return true;
        }
    }

    let parts: Vec<&str> = path.split('/').collect();
    parts.iter().any(|p| p.starts_with('.'))
}

fn validate_path(entry_name: &str, dest: &Path) -> Result<PathBuf, ZipError> {
    if entry_name.contains("..") {
        return Err(ZipError::PathTraversalAttempt(entry_name.to_string()));
    }

    if entry_name.starts_with('/') {
        return Err(ZipError::PathTraversalAttempt(entry_name.to_string()));
    }

    if entry_name.len() > 255 {
        return Err(ZipError::InvalidEncoding);
    }

    let full_path = dest.join(entry_name);

    let canonical_dest = dest.canonicalize().unwrap_or_else(|_| dest.to_path_buf());
    let canonical_full = full_path
        .parent()
        .and_then(|p| p.canonicalize().ok())
        .unwrap_or_else(|| full_path.clone());

    if !canonical_full.starts_with(&canonical_dest) {
        return Err(ZipError::PathTraversalAttempt(entry_name.to_string()));
    }

    Ok(full_path)
}

pub fn extract_zip(zip_bytes: &[u8], dest: &Path) -> Result<ExtractedProject, ZipError> {
    if zip_bytes.len() as u64 > MAX_ARCHIVE_SIZE {
        return Err(ZipError::ArchiveTooLarge(
            zip_bytes.len() as u64,
            MAX_ARCHIVE_SIZE,
        ));
    }

    std::fs::create_dir_all(dest).map_err(|e| ZipError::IoError(e.to_string()))?;

    let reader = Cursor::new(zip_bytes);
    let mut archive =
        ZipArchive::new(reader).map_err(|e| ZipError::InvalidArchive(e.to_string()))?;

    let mut files = Vec::new();

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| ZipError::ExtractionFailed(e.to_string()))?;
        let entry_name = file.name().to_string();

        if should_ignore(&entry_name) {
            continue;
        }

        if file.is_dir() {
            continue;
        }

        if file.size() > MAX_FILE_SIZE {
            return Err(ZipError::FileTooLarge(file.size(), MAX_FILE_SIZE));
        }

        let full_path = validate_path(&entry_name, dest)?;

        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ZipError::IoError(e.to_string()))?;
        }

        let mut content = Vec::with_capacity(file.size() as usize);
        file.read_to_end(&mut content)
            .map_err(|e| ZipError::IoError(e.to_string()))?;

        std::fs::write(&full_path, &content).map_err(|e| ZipError::IoError(e.to_string()))?;

        let mut hasher = Sha256::new();
        hasher.update(&content);
        let hash = hasher.finalize();
        let content_hash = format!("{:x}", hash);

        let relative_path = entry_name;
        let is_python = relative_path.ends_with(".py");

        files.push(FileInfo {
            relative_path,
            absolute_path: full_path,
            size: content.len() as u64,
            content_hash,
            is_python,
        });
    }

    let structure = tree::build_tree(dest);

    Ok(ExtractedProject {
        root_path: dest.to_path_buf(),
        files,
        structure,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_valid_zip_extraction() {
        let temp = TempDir::new().unwrap();
        let zip_data = create_test_zip("test.py", b"print('hello')");

        let result = extract_zip(&zip_data, temp.path());
        assert!(result.is_ok());

        let project = result.unwrap();
        assert!(!project.files.is_empty());
        assert!(project.files[0].is_python);
    }

    #[test]
    fn test_path_traversal_blocked() {
        let temp = TempDir::new().unwrap();
        let zip_data = create_test_zip("../../../etc/passwd", b"malicious");

        let result = extract_zip(&zip_data, temp.path());
        assert!(result.is_ok());
        assert!(result.unwrap().files.is_empty());
    }

    #[test]
    fn test_hidden_file_ignored() {
        let temp = TempDir::new().unwrap();
        let zip_data = create_test_zip(".hidden/file.py", b"hidden content");

        let result = extract_zip(&zip_data, temp.path());
        assert!(result.is_ok());
        assert!(result.unwrap().files.is_empty());
    }

    #[test]
    fn test_ignore_pycache() {
        let temp = TempDir::new().unwrap();
        let zip_data = create_test_zip("__pycache__/test.pyc", b"cached");

        let result = extract_zip(&zip_data, temp.path());
        assert!(result.is_ok());

        let project = result.unwrap();
        assert!(project.files.is_empty());
    }

    fn create_test_zip(filename: &str, content: &[u8]) -> Vec<u8> {
        use std::io::Seek;
        use zip::write::FileOptions;
        use zip::ZipWriter;

        let mut buf = Vec::new();
        {
            let mut cursor = Cursor::new(&mut buf);
            let mut zip = ZipWriter::new(&mut cursor);
            let options = FileOptions::default();

            zip.start_file(filename, options).unwrap();
            zip.write_all(content).unwrap();
            zip.finish().unwrap();
        }

        buf
    }
}
