use log::{info, trace, warn};
use std::path::Path;

use super::snapshot::{Snapshot, SnapshotPreview};

pub fn get_latest_snapshot_preview(root: &Path) -> Option<SnapshotPreview> {
    let snapshot_previews = load_all_snapshot_previews(root);
    snapshot_previews.last().cloned()
}

pub fn load_all_snapshot_previews(root: &Path) -> Vec<SnapshotPreview> {
    trace!("Loading all snapshot previews at: {:?}", root);
    load_all(root, SnapshotPreview::new)
}

pub fn load_all_snapshots(root: &Path) -> Vec<Snapshot> {
    trace!("Loading all snapshots at: {:?}", root);
    load_all(root, Snapshot::open)
}

fn load_all<F, T>(backup_root: &Path, get_snapshot: F) -> Vec<T>
where
    F: Fn(&Path) -> Option<T>,
    T: Ord,
{
    let backup_root = match backup_root.read_dir() {
        Ok(backup_root) => backup_root,
        Err(_) => return vec![],
    };

    let mut snapshots = vec![];
    for entry in backup_root.filter_map(std::result::Result::ok) {
        let entry_file_name = entry.file_name().to_string_lossy().as_ref().to_owned();

        info!("Loading snapshot: {}", entry_file_name);
        let preview = match get_snapshot(entry.path().as_path()) {
            Some(preview) => preview,
            None => {
                warn!(
                    "Found unrecognized entry in backup folder: \"{}\"",
                    entry_file_name
                );
                continue;
            }
        };
        snapshots.push(preview);
    }
    snapshots.sort_unstable();
    snapshots
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile;

    #[test]
    fn return_empty_vec_when_directory_is_empty() {
        let root = tempfile::tempdir().unwrap();
        let snapshots = load_all_snapshot_previews(root.path());

        assert_eq!(snapshots.len(), 0);
    }

    #[test]
    fn return_empty_vec_when_directory_is_invalid() {
        let root = tempfile::tempdir().unwrap();
        let invalid_path = root.path().join("Blah Blah Blah");
        let snapshots = load_all_snapshot_previews(invalid_path.as_path());

        assert_eq!(snapshots.len(), 0);
    }

    #[test]
    fn return_empty_vec_when_directory_has_some_foreign_entries() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path();
        std::fs::File::create(root.join("some_file")).unwrap();
        std::fs::create_dir(root.join("some_dir")).unwrap();

        let snapshots = load_all_snapshot_previews(root);

        assert_eq!(snapshots.len(), 0);
    }

    #[test]
    fn return_snapshot_preview_from_directory() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path();

        let snapshot_1 = root.join("2023-06-25_19.49");
        std::fs::create_dir(&snapshot_1).unwrap();
        std::fs::create_dir(snapshot_1.join("files")).unwrap();
        std::fs::File::create(snapshot_1.join("index.txt")).unwrap();

        std::fs::create_dir(root.join("some_dir")).unwrap();

        let snapshot_2 = root.join("2023-06-26_19.49");
        std::fs::create_dir(&snapshot_2).unwrap();
        std::fs::create_dir(snapshot_2.join("files")).unwrap();
        std::fs::File::create(snapshot_2.join("index.txt")).unwrap();

        let snapshots = load_all_snapshot_previews(root);

        assert_eq!(snapshots.len(), 2);
        assert!(snapshots[0] < snapshots[1]);
    }
}
