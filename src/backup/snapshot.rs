mod files;
mod index;
mod timestamp;

use files::Files;
use index::Index;
use log::{debug, trace, warn};
use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};
use timestamp::Timestamp;
use walkdir::WalkDir;

pub struct Snapshot {
    #[allow(dead_code)]
    location: PathBuf,
    timestamp: Timestamp,
    index: Index,
    files: Files,
}

impl Snapshot {
    pub fn create(root: &Path) -> Result<Snapshot, String> {
        if !root.is_dir() {
            return Err("Folder with backup does not exist or is not accessible".into());
        }
        let mut timestamp = Timestamp::now();

        let location = loop {
            let location = root.join(timestamp.to_string());
            if !location.exists() {
                fs::create_dir(&location).or(Err("Cannot create directory for a snapshot"))?;
                break location;
            }
            timestamp = timestamp.get_next();
        };
        let index = Index::new(location.join("index.txt"));
        let files = Files::new(location.join("files"));

        debug!("Created empty snapshot: {}", &timestamp.to_string());

        Ok(Snapshot {
            location,
            timestamp,
            index,
            files,
        })
    }

    pub fn open(location: &Path) -> Option<Snapshot> {
        let timestamp = Timestamp::parse_from(location.file_name()?.to_str()?)?;
        let index = Index::open(location.join("index.txt")).ok()?;
        let files = Files::new(location.join("files"));

        Some(Snapshot {
            location: location.to_owned(),
            timestamp,
            index,
            files,
        })
    }

    pub fn name(&self) -> String {
        self.timestamp.to_string()
    }

    pub fn index_files(&mut self, files: &[PathBuf]) -> Result<(), String> {
        debug!("Started indexing files");

        for path in files {
            self.add_path_to_index(path)?;
        }

        self.index.save().or(Err("Error while saving index.txt"))?;
        debug!("Finished indexing files");
        Ok(())
    }

    fn add_path_to_index(&mut self, path: &Path) -> Result<(), String> {
        if !path.exists() {
            return Err(format!("Invalid path: {}", path.display()));
        }

        for entry in WalkDir::new(&path) {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    warn!("{}", e);
                    continue;
                }
            };
            let file_path = entry
                .path()
                .canonicalize()
                .or(Err("Cannot resolve file path"))?;
            trace!("Indexed: {}", file_path.display());
            self.index.push(&self.timestamp, file_path);
        }
        Ok(())
    }

    pub fn copy_indexed_files(&self) -> Result<(), String> {
        debug!("Started copying files");

        let index_entries = self.index.entries.iter();
        let entries_to_copy = index_entries.map(|e| e.path.as_path());
        self.files.copy_all(entries_to_copy)?;

        debug!("Finished copying files");
        Ok(())
    }
}

impl PartialEq for Snapshot {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp
    }
}

impl PartialOrd for Snapshot {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.timestamp.partial_cmp(&other.timestamp)
    }
}

impl Eq for Snapshot {}

impl Ord for Snapshot {
    fn cmp(&self, other: &Self) -> Ordering {
        self.timestamp.cmp(&other.timestamp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile;

    #[test]
    fn create_snapshot_in_nonexistent_folder() {
        let result = Snapshot::create(Path::new("nonexistent"));

        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap(),
            "Folder with backup does not exist or is not accessible"
        );
    }

    #[test]
    fn index_invalid_path() {
        let root = tempfile::tempdir().unwrap();
        let mut snapshot = Snapshot::create(root.path()).unwrap();

        let result = snapshot.index_files(&[PathBuf::from("incorrect path")]);
        assert!(result.is_err());
    }

    #[test]
    fn index_empty_folder() {
        let root = tempfile::tempdir().unwrap();
        let files = tempfile::tempdir().unwrap();
        let mut snapshot = Snapshot::create(root.path()).unwrap();

        snapshot.index_files(&[files.path().to_owned()]).unwrap();

        let index_content = fs::read_to_string(snapshot.location.join("index.txt")).unwrap();

        assert_eq!(
            index_content,
            format!(
                "{} {}\n",
                snapshot.name(),
                files.path().canonicalize().unwrap().display()
            )
        )
    }
}
