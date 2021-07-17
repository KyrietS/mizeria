mod files;
mod index;
mod timestamp;

use files::Files;
use index::Index;
use log::{debug, error, trace};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::{fs, io};
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

    pub fn backup_files(&mut self, files: &[PathBuf]) -> io::Result<()> {
        debug!("Started backup process");

        for path in files {
            self.backup_path_recursively(path);
        }

        self.index.save()?;

        debug!("Finished backup process");
        Ok(())
    }

    fn backup_path_recursively(&mut self, path: &Path) {
        for entry in WalkDir::new(path) {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    error!("{}", e);
                    continue;
                }
            };

            self.copy_and_index_entry(entry.path());
        }
    }

    fn copy_and_index_entry(&mut self, entry: &Path) {
        let destination = self.files.copy_entry(entry);
        match destination {
            Ok(destination) => {
                trace!(
                    "Copied: \"{}\" -> \"{}\"",
                    entry.display(),
                    destination.display()
                );
                self.index_entry(entry);
            }
            Err(e) => {
                error!("Failed to copy: \"{}\" ({})", entry.display(), e);
            }
        }
    }

    fn index_entry(&mut self, entry: &Path) {
        let absolute_path = entry.canonicalize();

        match absolute_path {
            Ok(absolute_path) => {
                self.index.push(&self.timestamp, absolute_path.clone());
                trace!("Indexed: {}", absolute_path.display());
            }
            Err(e) => error!("Failed to index: \"{}\" ({})", entry.display(), e),
        }
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
    fn backup_invalid_path() {
        let root = tempfile::tempdir().unwrap();
        let mut snapshot = Snapshot::create(root.path()).unwrap();

        let result = snapshot.backup_files(&[PathBuf::from("incorrect path")]);
        assert!(result.is_ok());

        let index_content = fs::read_to_string(snapshot.index.location).unwrap();
        assert!(index_content.is_empty());
    }
}
