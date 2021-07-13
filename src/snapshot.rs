use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use log::{debug, error, trace, warn};
use path_absolutize::Absolutize;
use walkdir::WalkDir;

#[derive(Debug)]
pub struct Snapshot {
    root: PathBuf,
    location: PathBuf,
    timestamp: Timestamp,
    index: PathBuf,
    files: PathBuf,
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
        let index = location.join("index.txt");
        let files = location.join("files");

        debug!("Created empty snapshot: {}", &timestamp.to_string());

        Ok(Snapshot {
            root: root.to_owned(),
            location,
            timestamp,
            index,
            files,
        })
    }

    pub fn name(&self) -> String {
        self.timestamp.to_string()
    }

    pub fn index_files(&self, files: &Path) -> Result<(), String> {
        debug!("Started indexing files");
        if !files.exists() {
            return Err(format!("Invalid path: {}", files.display()));
        }

        let mut index = File::create(&self.index).or(Err("Cannot create index file"))?;

        for entry in WalkDir::new(&files) {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    warn!("{}", e);
                    continue;
                }
            };
            let file_path = entry.path().absolutize().unwrap();
            writeln!(index, "{} {}", self.name(), file_path.display())
                .expect("Error while writing to index.txt");
            trace!("Indexed: {}", file_path.display());
        }
        debug!("Finished indexing files");
        Ok(())
    }

    pub fn copy_files(&self, files: &Path) -> Result<(), String> {
        debug!("Started copying files");
        if !files.exists() {
            return Err(format!("Invalid path: {}", files.display()));
        }

        fs::create_dir(&self.files).or(Err("Cannot create directory for snapshot files"))?;

        for entry in WalkDir::new(&files) {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    warn!("{}", e);
                    continue;
                }
            };

            let entry = entry.path();

            if entry.is_dir() {
                self.copy_dir_entry(&entry);
            } else if entry.is_file() {
                self.copy_file_entry(&entry)
            }
        }
        debug!("Finished copying files");
        Ok(())
    }

    fn copy_dir_entry(&self, dir_to_copy: &Path) {
        let snapshot_entry = self.to_snapshot_path(&dir_to_copy);
        let result = fs::create_dir_all(&snapshot_entry);
        trace!(
            "Createed dir: \"{}\" -> \"{}\"",
            dir_to_copy.display(),
            snapshot_entry.display()
        );
        if let Err(e) = result {
            error!(
                "Cannot create directory: \"{}\" -> \"{}\"",
                dir_to_copy.display(),
                snapshot_entry.display()
            );
            error!("{}", e);
        }
    }

    fn copy_file_entry(&self, file_to_copy: &Path) {
        let snapshot_entry = self.to_snapshot_path(&file_to_copy);
        let result = fs::copy(file_to_copy, &snapshot_entry);
        trace!(
            "Copied file: \"{}\" -> \"{}\"",
            file_to_copy.display(),
            snapshot_entry.display()
        );
        if let Err(e) = result {
            error!(
                "Cannot copy file: \"{}\" -> \"{}\"",
                file_to_copy.display(),
                snapshot_entry.display()
            );
            error!("{}", e);
        } else {
            warn!("Unknown file type: {}", file_to_copy.display());
        }
    }

    fn to_snapshot_path(&self, entry: &Path) -> PathBuf {
        let snapshot_entry = entry.absolutize().unwrap();

        // remove leading '/'.
        let snapshot_entry = match snapshot_entry.strip_prefix("/") {
            Ok(not_absolute) => not_absolute,
            Err(_) => snapshot_entry.as_ref(),
        };

        // remove ':' from 'C:/folder'.
        let snapshot_entry = snapshot_entry.to_string_lossy().replace(":", "");

        self.files.join(snapshot_entry)
    }
}

#[derive(PartialEq, PartialOrd, Debug)]
pub struct Timestamp {
    inner: chrono::DateTime<chrono::Local>,
}

impl Timestamp {
    pub fn now() -> Self {
        Self {
            inner: chrono::offset::Local::now(),
        }
    }
    pub fn get_next(&self) -> Self {
        let next_date_time = self.inner + chrono::Duration::minutes(1);
        Self {
            inner: next_date_time,
        }
    }
}

impl ToString for Timestamp {
    fn to_string(&self) -> String {
        use chrono::{Datelike, Timelike};
        let date = self.inner.date();
        let time = self.inner.time();
        format!(
            "{}-{:02}-{:02}_{:02}.{:02}",
            date.year(),
            date.month(),
            date.day(),
            time.hour(),
            time.minute()
        )
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use tempfile;

    #[test]
    fn create_snapshot_in_nonexistent_folder() {
        let result = Snapshot::create(Path::new("nonexistent"));

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Folder with backup does not exist or is not accessible"
        );
    }

    #[test]
    fn copy_files_from_invalid_path() {
        let root = tempfile::tempdir().unwrap();
        let snapshot = Snapshot::create(root.path()).unwrap();

        let result = snapshot.copy_files(Path::new("incorrect path"));
        assert!(result.is_err());
    }

    #[test]
    fn index_invalid_path() {
        let root = tempfile::tempdir().unwrap();
        let snapshot = Snapshot::create(root.path()).unwrap();

        let result = snapshot.index_files(Path::new("incorrect path"));
        assert!(result.is_err());
    }

    #[test]
    fn index_empty_folder() {
        let root = tempfile::tempdir().unwrap();
        let files = tempfile::tempdir().unwrap();
        let snapshot = Snapshot::create(root.path()).unwrap();

        snapshot.index_files(files.path()).unwrap();

        let index_content = fs::read_to_string(&snapshot.index).unwrap();

        assert_eq!(
            index_content,
            format!(
                "{} {}\n",
                snapshot.name(),
                files.path().absolutize().unwrap().display()
            )
        )
    }
}
