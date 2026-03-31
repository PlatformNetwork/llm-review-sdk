//! Project tree and file utilities.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use super::ENTRY_POINT_FILES;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub relative_path: String,
    pub absolute_path: PathBuf,
    pub size: u64,
    pub content_hash: String,
    pub is_python: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectTree {
    pub name: String,
    pub children: Vec<ProjectTree>,
    pub is_file: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
}

impl ProjectTree {
    pub fn new_dir(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            children: Vec::new(),
            is_file: false,
            path: None,
        }
    }

    pub fn new_file(name: impl Into<String>, path: PathBuf) -> Self {
        Self {
            name: name.into(),
            children: Vec::new(),
            is_file: true,
            path: Some(path),
        }
    }

    pub fn add_child(&mut self, child: ProjectTree) {
        self.children.push(child);
    }
}

pub fn build_tree(path: &Path) -> ProjectTree {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("root")
        .to_string();

    let mut root = ProjectTree::new_dir(&name);

    for entry in WalkDir::new(path)
        .min_depth(1)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let relative = entry.path().strip_prefix(path).unwrap_or(entry.path());
        let parts: Vec<&str> = relative.iter().filter_map(|p| p.to_str()).collect();

        if parts.is_empty() {
            continue;
        }

        add_to_tree(&mut root, &parts, entry.path());
    }

    root
}

fn add_to_tree(tree: &mut ProjectTree, parts: &[&str], full_path: &Path) {
    if parts.is_empty() {
        return;
    }

    let first = parts[0];

    if parts.len() == 1 {
        if let Some(existing) = tree.children.iter_mut().find(|c| c.name == first) {
            existing.path = Some(full_path.to_path_buf());
        } else {
            let is_file = full_path.is_file();
            let node = if is_file {
                ProjectTree::new_file(first, full_path.to_path_buf())
            } else {
                ProjectTree::new_dir(first)
            };
            tree.add_child(node);
        }
    } else {
        if let Some(child) = tree.children.iter_mut().find(|c| c.name == first) {
            add_to_tree(child, &parts[1..], full_path);
        } else {
            let mut new_dir = ProjectTree::new_dir(first);
            add_to_tree(&mut new_dir, &parts[1..], full_path);
            tree.add_child(new_dir);
        }
    }
}

pub fn list_python_files(tree: &ProjectTree) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_python_files(tree, &mut files);
    files
}

fn collect_python_files(tree: &ProjectTree, files: &mut Vec<PathBuf>) {
    if tree.is_file {
        if let Some(ref path) = tree.path {
            if path.extension().map(|e| e == "py").unwrap_or(false) {
                files.push(path.clone());
            }
        }
    } else {
        for child in &tree.children {
            collect_python_files(child, files);
        }
    }
}

pub fn find_entry_file(tree: &ProjectTree) -> Option<PathBuf> {
    for name in ENTRY_POINT_FILES {
        if let Some(path) = find_file_by_name(tree, name) {
            return Some(path);
        }
    }
    None
}

fn find_file_by_name(tree: &ProjectTree, name: &str) -> Option<PathBuf> {
    if tree.is_file && tree.name == name {
        return tree.path.clone();
    }

    for child in &tree.children {
        if let Some(path) = find_file_by_name(child, name) {
            return Some(path);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_list_python_files() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("main.py"), "").unwrap();
        fs::write(temp.path().join("test.txt"), "").unwrap();
        fs::create_dir(temp.path().join("subdir")).unwrap();
        fs::write(temp.path().join("subdir/utils.py"), "").unwrap();

        let tree = build_tree(temp.path());
        let py_files = list_python_files(&tree);

        assert_eq!(py_files.len(), 2);
    }

    #[test]
    fn test_find_entry_file() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("app.py"), "print('app')").unwrap();

        let tree = build_tree(temp.path());
        let entry = find_entry_file(&tree);

        assert!(entry.is_some());
        assert_eq!(entry.unwrap().file_name().unwrap(), "app.py");
    }
}
