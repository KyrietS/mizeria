mod files;
mod index;
mod timestamp;

use files::Files;
use index::{Index, IndexPreview};
use log::{debug, error, trace, warn};
use std::cmp::Ordering;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::{fs, io};
use timestamp::Timestamp;
use walkdir::WalkDir;

pub struct Snapshot {
    location: PathBuf,
    timestamp: Timestamp,
    index: Index,
    files: Files,
    config: SnapshotConfig,
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
            config: SnapshotConfig::default(),
        })
    }

    #[allow(dead_code)]
    pub fn open(location: &Path) -> Result<Snapshot, String> {
        let snapshot_name = location
            .file_name()
            .ok_or("Invalid snapshot name")?
            .to_string_lossy();
        let timestamp = Timestamp::parse_from(&snapshot_name).ok_or("Failed to parse timestamp")?;
        let index = Index::open(location.join("index.txt"))?;
        let files = Files::new(location.join("files"));

        Ok(Snapshot {
            location: location.to_owned(),
            timestamp,
            index,
            files,
            config: SnapshotConfig::default(),
        })
    }

    pub fn open_preview(location: &Path) -> Option<SnapshotPreview> {
        SnapshotPreview::new(location)
    }

    pub fn as_preview(&self) -> SnapshotPreview {
        SnapshotPreview::new(self.location.as_path()).unwrap()
    }

    pub fn set_base_snapshot(&mut self, base_snapshot: Option<&SnapshotPreview>) {
        let base_index = match base_snapshot {
            Some(snapshot) => {
                let index_preview = IndexPreview::open(snapshot.index.as_path());
                match index_preview {
                    Ok(index_preview) => Some(index_preview),
                    Err(e) => {
                        warn!("Failed to load base index with cause: {}", e);
                        None
                    }
                }
            }
            None => None,
        };

        let base_snapshot_str = match base_index {
            Some(_) => base_snapshot.unwrap().timestamp.to_string(),
            None => String::from("None"),
        };
        debug!("Base snapshot set to: {}", base_snapshot_str);

        self.config.base_index = base_index;
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
        for entry in WalkDir::new(path).follow_links(false) {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    error!("{}", e);
                    continue;
                }
            };

            let entry = entry.path();

            match self.is_entry_already_backed_up(entry) {
                Some(prev_timestamp) => self.index_entry(prev_timestamp, entry),
                None => self.copy_and_index_entry(entry),
            }
        }
    }

    fn is_entry_already_backed_up(&self, entry: &Path) -> Option<Timestamp> {
        let margin = chrono::Duration::minutes(1);
        let prev_timestamp = self.config.base_index.as_ref()?.find(entry)?;
        let prev_timestamp_with_margin = prev_timestamp.clone() - margin;

        let metadata = entry.symlink_metadata().ok()?;
        let modif_system_time = metadata.modified().ok()?;
        let create_system_time = metadata.created().ok()?;
        let modif_timestamp = Timestamp::from(modif_system_time);
        let create_timestamp = Timestamp::from(create_system_time);

        let file_has_changed = modif_timestamp > prev_timestamp_with_margin
            || create_timestamp > prev_timestamp_with_margin;
        trace!(
            "Entry \"{}\" (modif: {}) found in snapshot: {}, has_changed={}",
            entry.display(),
            modif_timestamp,
            prev_timestamp,
            file_has_changed
        );
        if file_has_changed {
            None
        } else {
            Some(prev_timestamp.clone())
        }
    }

    fn copy_and_index_entry(&mut self, entry: &Path) {
        let destination = self.files.copy_entry(entry);
        match destination {
            Ok(destination) => {
                debug!(
                    "Copied: \"{}\" -> \"{}\"",
                    entry.display(),
                    destination.display()
                );
                self.index_entry(self.timestamp.clone(), entry);
            }
            Err(e) => {
                error!("Failed to copy: \"{}\" ({})", entry.display(), e);
            }
        }
    }

    fn index_entry(&mut self, timestamp: Timestamp, entry: &Path) {
        let absolute_path = entry.canonicalize();

        match absolute_path {
            Ok(absolute_path) => {
                trace!("Indexed: {} {}", timestamp, absolute_path.display());
                self.index.push(timestamp, absolute_path);
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
impl Debug for Snapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.timestamp)
    }
}

struct SnapshotConfig {
    base_index: Option<IndexPreview>,
}

impl<'a> SnapshotConfig {
    fn default() -> Self {
        Self { base_index: None }
    }
}

pub struct SnapshotPreview {
    timestamp: Timestamp,
    index: PathBuf,
    #[allow(dead_code)] // will be used in the future
    files: PathBuf,
}

impl SnapshotPreview {
    pub fn new(location: &Path) -> Option<Self> {
        let timestamp = Timestamp::parse_from(location.file_name()?.to_str()?)?;
        let index = location.join("index.txt");
        let files = location.join("files");

        index.exists().then(|| ())?;
        files.exists().then(|| ())?;

        Some(SnapshotPreview {
            timestamp,
            index,
            files,
        })
    }
}

impl PartialEq for SnapshotPreview {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp
    }
}
impl PartialOrd for SnapshotPreview {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.timestamp.partial_cmp(&other.timestamp)
    }
}
impl Eq for SnapshotPreview {}
impl Ord for SnapshotPreview {
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
