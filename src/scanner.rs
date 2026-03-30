use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
    pub is_dir: bool,
    pub children: Vec<Entry>,
}

pub fn scan(path: &PathBuf) -> Result<Entry, ScanError> {
    scan_path(path)
}

fn scan_path(path: &PathBuf) -> Result<Entry, ScanError> {
    let metadata = std::fs::symlink_metadata(path)?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned());

    if metadata.is_symlink() || !metadata.is_dir() {
        return Ok(Entry {
            path: path.clone(),
            name,
            size: metadata.len(),
            is_dir: false,
            children: vec![],
        });
    }

    let mut children = vec![];
    let mut total_size: u64 = 0;

    match std::fs::read_dir(path) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let child_path = entry.path();
                match scan_path(&child_path) {
                    Ok(child) => {
                        total_size = total_size.saturating_add(child.size);
                        children.push(child);
                    }
                    Err(_) => {}
                }
            }
        }
        Err(_) => {}
    }

    Ok(Entry {
        path: path.clone(),
        name,
        size: total_size,
        is_dir: true,
        children,
    })
}

pub fn sort_by_size(entries: &mut Vec<Entry>) {
    entries.sort_by(|a, b| b.size.cmp(&a.size));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_scan_single_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("foo.txt");
        fs::write(&file, b"hello").unwrap();

        let entry = scan_path(&file).unwrap();
        assert_eq!(entry.size, 5);
        assert!(!entry.is_dir);
    }

    #[test]
    fn test_scan_dir_aggregates_sizes() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), b"hello").unwrap(); // 5
        fs::write(dir.path().join("b.txt"), b"world!!").unwrap(); // 7

        let entry = scan(&dir.path().to_path_buf()).unwrap();
        assert!(entry.is_dir);
        assert_eq!(entry.size, 12);
    }

    #[test]
    fn test_scan_nested_dirs() {
        let dir = tempdir().unwrap();
        let sub = dir.path().join("sub");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("inner.txt"), b"abcdef").unwrap(); // 6
        fs::write(dir.path().join("top.txt"), b"xy").unwrap(); // 2

        let entry = scan(&dir.path().to_path_buf()).unwrap();
        assert_eq!(entry.size, 8);
    }

    #[test]
    fn test_scan_empty_dir() {
        let dir = tempdir().unwrap();
        let entry = scan(&dir.path().to_path_buf()).unwrap();
        assert!(entry.is_dir);
        assert_eq!(entry.size, 0);
        assert!(entry.children.is_empty());
    }

    #[test]
    fn test_sort_by_size_descending() {
        let mut entries = vec![
            Entry { path: PathBuf::from("a"), name: "a".into(), size: 10, is_dir: false, children: vec![] },
            Entry { path: PathBuf::from("b"), name: "b".into(), size: 50, is_dir: false, children: vec![] },
            Entry { path: PathBuf::from("c"), name: "c".into(), size: 5, is_dir: false, children: vec![] },
        ];
        sort_by_size(&mut entries);
        assert_eq!(entries[0].size, 50);
        assert_eq!(entries[1].size, 10);
        assert_eq!(entries[2].size, 5);
    }

    #[test]
    fn test_sort_empty() {
        let mut entries: Vec<Entry> = vec![];
        sort_by_size(&mut entries);
        assert!(entries.is_empty());
    }
}
