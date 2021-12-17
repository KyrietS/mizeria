use std::{
    borrow::Borrow,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use log::{debug, warn};
use snapshot::{Snapshot, SnapshotPreview};

use crate::result::IntegrityCheckResult;

mod snapshot;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub struct Backup {
    location: PathBuf,
    snapshots: Vec<SnapshotPreview>,
}

impl Backup {
    pub fn open(path: &Path) -> Result<Backup> {
        let backup_root = path
            .read_dir()
            .or(Err("Folder with backup doesn't exist or isn't accessible"))?;

        let mut snapshots = vec![];
        for entry in backup_root.filter_map(std::result::Result::ok) {
            let entry_file_name = entry.file_name().to_string_lossy().as_ref().to_owned();

            if Snapshot::has_valid_name(&entry_file_name) {
                let snapshot = Snapshot::open_preview(entry.path().borrow());
                match snapshot {
                    Some(snapshot) => snapshots.push(snapshot),
                    None => warn!("Failed to load snapshot: {}", entry_file_name),
                }
            } else {
                warn!(
                    "Found unrecognized entry in backup folder: {}",
                    entry_file_name
                );
            }
        }
        snapshots.sort_unstable();

        Ok(Backup {
            location: path.to_owned(),
            snapshots,
        })
    }

    pub fn check_integrity(&self, snapshot_name: &OsStr) -> IntegrityCheckResult {
        debug!("Integrity check start");
        let snapshot_path = self.location.join(snapshot_name);
        Snapshot::check_integrity(&snapshot_path)
    }

    pub fn add_snapshot(&mut self, files: &[PathBuf], incremental: bool) -> Result<String> {
        debug!("Started backup process");
        let mut new_snapshot = Snapshot::create(self.location.as_path())?;

        self.set_incremental_snapshot(&mut new_snapshot, incremental);
        let filteres_files = Self::validate_input_paths(files);

        for path in filteres_files {
            new_snapshot.add_files_to_snapshot(path);
        }
        new_snapshot.save_index()?;

        debug!("Finished backup process");
        self.snapshots.push(new_snapshot.to_preview());

        Ok(new_snapshot.name())
    }

    fn set_incremental_snapshot(&self, snapshot: &mut Snapshot, incremental: bool) {
        if incremental {
            debug!("Incremental snapshot will be performed");
            let latest_snapshot = self.latest_snapshot();
            snapshot.set_base_snapshot(latest_snapshot);
        } else {
            debug!("Full snapshot will be performed");
        }
    }

    fn latest_snapshot(&self) -> Option<&SnapshotPreview> {
        self.snapshots.last()
    }

    fn validate_input_paths(paths: &[PathBuf]) -> Vec<&PathBuf> {
        let existent_paths = Self::remove_nonexistent_paths(paths);
        let paths_without_duplicates = Self::remove_duplicated_paths(existent_paths);
        Self::remove_overlapping_paths(paths_without_duplicates)
    }

    fn remove_nonexistent_paths(paths: &[PathBuf]) -> Vec<&PathBuf> {
        let mut filtered = vec![];
        for path in paths {
            if path.exists() {
                filtered.push(path);
            } else {
                warn!("Provided path doesn't exist: {}", path.display());
            }
        }
        filtered
    }

    fn remove_duplicated_paths(paths: Vec<&PathBuf>) -> Vec<&PathBuf> {
        let mut filtered: Vec<&PathBuf> = vec![];

        for path in paths {
            let absolute_path = path.canonicalize().unwrap();
            let duplicate = filtered
                .iter()
                .find(|p| p.canonicalize().unwrap() == absolute_path);
            match duplicate {
                Some(duplicate) => warn!(
                    "Path \"{}\" is the same as {}",
                    path.display(),
                    duplicate.display()
                ),
                None => filtered.push(path),
            }
        }
        filtered
    }

    fn remove_overlapping_paths(paths: Vec<&PathBuf>) -> Vec<&PathBuf> {
        let mut filtered = vec![];

        for path in &paths {
            let absolute_path = path.canonicalize().unwrap();
            let prefix_path = paths.iter().find(|p| {
                let p_abs = p.canonicalize().unwrap();
                let paths_are_different = absolute_path != p_abs;
                let path_has_prefix = absolute_path.starts_with(&p_abs);
                path_has_prefix && paths_are_different
            });
            match prefix_path {
                Some(prefix) => warn!(
                    "Path \"{}\" includes \"{}\". Child path will be ignored",
                    prefix.display(),
                    path.display()
                ),
                None => filtered.push(*path),
            }
        }

        filtered
    }
}

#[cfg(test)]
mod tests {
    use std::fs::create_dir_all;

    use super::*;

    #[test]
    fn remove_nonexistent_paths() {
        let tempdir = tempfile::tempdir().unwrap();
        let existent = tempdir.path().to_owned();
        let nonexistent = existent.join("foobar");
        let paths = [existent.clone(), nonexistent];

        let result = Backup::remove_nonexistent_paths(&paths);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], &existent);
    }

    #[test]
    fn remove_duplicated_paths() {
        let path_1 = tempfile::tempdir().unwrap();
        let path_1 = path_1.path().to_owned();
        let path_2 = tempfile::tempdir().unwrap();
        let path_2 = path_2.path().to_owned();
        let path_3 = path_1.clone();
        let path_4 = tempfile::tempdir().unwrap();
        let path_4 = path_4.path().to_owned();
        let paths = vec![&path_1, &path_2, &path_3, &path_4];

        let result = Backup::remove_duplicated_paths(paths);

        assert_eq!(result.len(), 3);
        assert_eq!(result[0], &path_1);
        assert_eq!(result[1], &path_2);
        assert_eq!(result[2], &path_4);
    }

    #[test]
    fn remove_duplicated_paths_presists_order() {
        let path_1 = tempfile::tempdir().unwrap();
        let path_1 = path_1.path().to_owned();
        let path_2 = tempfile::tempdir().unwrap();
        let path_2 = path_2.path().to_owned();
        let path_3 = tempfile::tempdir().unwrap();
        let path_3 = path_3.path().to_owned();
        let paths = vec![&path_1, &path_2, &path_3];

        let result = Backup::remove_duplicated_paths(paths);

        assert_eq!(result.len(), 3);
        assert_eq!(result[0], &path_1);
        assert_eq!(result[1], &path_2);
        assert_eq!(result[2], &path_3);
    }

    #[test]
    fn remove_overlapping_two_same_paths() {
        let tempdir = tempfile::tempdir().unwrap();
        let tempdir = tempdir.path();

        let path_1 = tempdir.join("aaa").join("bbb");
        let path_2 = tempdir.join("aaa").join("bbb");
        create_dir_all(&path_1).unwrap();
        let paths = vec![&path_1, &path_2];

        let filtered = Backup::remove_overlapping_paths(paths);

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0], &path_1);
        assert_eq!(filtered[1], &path_2);
    }

    #[test]
    fn remove_overlapping_paths() {
        let tempdir = tempfile::tempdir().unwrap();
        let tempdir = tempdir.path();

        let path_1 = tempdir.join("aaa");
        let path_2 = tempdir.join("aaa").join("bbb");
        let path_3 = tempdir.join("aaa").join("bbb").join("ccc");
        let path_4 = tempdir.join("xxx");
        create_dir_all(&path_3).unwrap();
        create_dir_all(&path_4).unwrap();
        let paths = vec![&path_1, &path_3, &path_4, &path_2];

        let filtered = Backup::remove_overlapping_paths(paths);

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0], &path_1);
        assert_eq!(filtered[1], &path_4);
    }
}
