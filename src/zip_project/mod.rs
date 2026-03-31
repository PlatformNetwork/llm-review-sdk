//! ZIP project extraction and handling for Python submissions.

mod extractor;
mod tree;

pub use extractor::{extract_zip, ExtractedProject, ZipError};
pub use tree::{ProjectTree, FileInfo, list_python_files, build_tree, find_entry_file};

pub const IGNORE_DIRS: &[&str] = &[
    "__pycache__", ".git", "venv", ".venv",
    "node_modules", ".env", ".mypy_cache", ".tox",
    ".pytest_cache", ".eggs",
];

pub const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

pub const MAX_ARCHIVE_SIZE: u64 = 50 * 1024 * 1024;

pub const ENTRY_POINT_FILES: &[&str] = &[
    "main.py", "app.py", "__main__.py", "run.py", "server.py",
];
