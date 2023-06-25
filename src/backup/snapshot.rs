mod files;
mod index;
mod timestamp;

use files::Files;
use index::{Index, IndexPreview};
use log::{debug, error, info, trace, warn};
use std::cmp::Ordering;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::{fs, io};
use timestamp::Timestamp;
use walkdir::WalkDir;

use super::snapshot_utils::get_latest_snapshot_preview;
use super::IntegrityCheckResult;

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

        let timestamp = get_timestamp_for_new_snapshot(root);

        let location = root.join(timestamp.to_string());
        fs::create_dir(&location).or(Err("Cannot create directory for a snapshot"))?;

        let index = Index::new(location.join("index.txt"));
        let files = Files::new(location.join("files"));

        debug!("Created new snapshot: {}", timestamp);
        Ok(Snapshot {
            location,
            timestamp,
            index,
            files,
            config: SnapshotConfig::default(),
        })
    }

    #[allow(dead_code)] // will be used in the future to extract metadate of a snapshot
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

    pub fn to_preview(&self) -> SnapshotPreview {
        SnapshotPreview::new(self.location.as_path()).unwrap()
    }

    pub fn has_valid_name<T: AsRef<str>>(name: T) -> bool {
        Timestamp::is_valid(name.as_ref())
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

    pub fn save_index(&self) -> io::Result<()> {
        self.index.save()
    }

    pub fn add_files_to_snapshot(&mut self, path: &Path) {
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
        let margin = time::Duration::minutes(1);
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
        if self.copy_entry(entry).is_ok() {
            self.index_entry(self.timestamp.clone(), entry);
        }
    }

    fn copy_entry(&mut self, entry: &Path) -> Result<(), ()> {
        let destination = self.files.copy_entry(entry);
        match destination {
            Ok(destination) => {
                debug!(
                    "Copied: \"{}\" -> \"{}\"",
                    entry.display(),
                    destination.display()
                );
                Ok(())
            }
            Err(e) => {
                error!("Failed to copy: \"{}\" ({})", entry.display(), e);
                Err(())
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

fn get_timestamp_for_new_snapshot(root: &Path) -> Timestamp {
    let mut current_timestamp = Timestamp::now();
    debug!("Current timestamp: {}", current_timestamp);
    let timestamp_of_latest_snapshot = get_latest_snapshot_preview(root).map(|s| s.timestamp);

    // If there is a snapshot from the future, then set current_timestamp to its timestamp + 1 minute.
    if let Some(timestamp_of_latest_snapshot) = timestamp_of_latest_snapshot {
        debug!("Latest snapshot: {}", timestamp_of_latest_snapshot);
        if current_timestamp < timestamp_of_latest_snapshot {
            current_timestamp = timestamp_of_latest_snapshot.get_next();
            warn!(
                "Found snapshot from the future: {}. New snapshot will be created with timestamp: {}",
                timestamp_of_latest_snapshot,
                current_timestamp
            );
        }
    }

    loop {
        let location = root.join(current_timestamp.to_string());
        if !location.exists() {
            break;
        }
        current_timestamp = current_timestamp.get_next();
    }

    current_timestamp
}

// -------------------------------------
// Integrity check
// -------------------------------------
impl Snapshot {
    pub fn check_integrity(location: &Path) -> IntegrityCheckResult {
        if !location.exists() {
            return IntegrityCheckResult::SnapshotDoesntExist;
        }
        let snapshot_name = match location.file_name() {
            Some(name) => name.to_string_lossy(),
            None => return IntegrityCheckResult::SnapshotNameHasInvalidTimestamp("..".into()),
        };
        if !Snapshot::has_valid_name(&snapshot_name) {
            return IntegrityCheckResult::SnapshotNameHasInvalidTimestamp(snapshot_name.into());
        }

        let index_integrity_result = Index::check_integrity(location.join("index.txt"));
        match index_integrity_result {
            IntegrityCheckResult::Success => info!("Index integrity check passed"),
            _ => return index_integrity_result,
        }

        let index = match Index::open(location.join("index.txt")) {
            Ok(index) => index,
            Err(err) => return IntegrityCheckResult::UnexpectedError(err),
        };

        warn!("This is just a shallow integrity check of one snapshot!");
        warn!("Deep (full) integrity check for the entire backup is not yet implemented.");
        let entries_from_this_snapshot = index
            .entries
            .iter()
            .filter(|e| e.timestamp.to_string() == snapshot_name)
            .map(|e| &e.path);

        let files_integrity_result =
            Files::check_integrity(location.join("files"), entries_from_this_snapshot);
        match files_integrity_result {
            IntegrityCheckResult::Success => info!("Files integrity check passed"),
            _ => return files_integrity_result,
        }

        IntegrityCheckResult::Success
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

impl SnapshotConfig {
    fn default() -> Self {
        Self { base_index: None }
    }
}
#[derive(Clone)]
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

        index.exists().then_some(())?;
        files.exists().then_some(())?;

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

        snapshot.add_files_to_snapshot(Path::new("incorrect path"));
        let result = snapshot.save_index();
        assert!(result.is_ok());

        let index_content = fs::read_to_string(snapshot.index.location).unwrap();
        assert!(index_content.is_empty());
    }
}
