mod index;
mod timestamp;

use log::{debug, error, trace, warn};
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Component, Components, Path, PathBuf, Prefix, PrefixComponent};
use walkdir::WalkDir;

use index::Index;
use timestamp::Timestamp;

#[allow(dead_code)]
pub struct Snapshot {
    root: PathBuf,     // unused
    location: PathBuf, // unused
    timestamp: Timestamp,
    index: Index,
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
        let index = Index::new(timestamp.clone(), location.join("index.txt"));
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

    pub fn index_files(&mut self, files: &Path) -> Result<(), String> {
        debug!("Started indexing files");
        if !files.exists() {
            return Err(format!("Invalid path: {}", files.display()));
        }

        for entry in WalkDir::new(&files) {
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
        self.index.save().or(Err("Error while saving index.txt"))?;
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
            } else {
                warn!("Entry inaccessible: {}", &entry.display());
            }
        }
        debug!("Finished copying files");
        Ok(())
    }

    fn copy_dir_entry(&self, dir_to_copy: &Path) {
        if let Err(e) = self.try_copy_dir(dir_to_copy) {
            error!("Cannot create directory: {}", dir_to_copy.display());
            error!("{}", e);
        }
    }

    fn try_copy_dir(&self, dir_to_copy: &Path) -> io::Result<()> {
        let snapshot_entry = self.to_snapshot_path(&dir_to_copy)?;
        fs::create_dir_all(&snapshot_entry)?;
        trace!(
            "Createed dir: \"{}\" -> \"{}\"",
            dir_to_copy.display(),
            snapshot_entry.display()
        );
        Ok(())
    }

    fn copy_file_entry(&self, file_to_copy: &Path) {
        if let Err(e) = self.try_copy_file(file_to_copy) {
            error!("Cannot copy file: {}", file_to_copy.display());
            error!("{}", e);
        }
    }

    fn try_copy_file(&self, file_to_copy: &Path) -> io::Result<()> {
        let snapshot_entry = self.to_snapshot_path(&file_to_copy)?;
        fs::copy(file_to_copy, &snapshot_entry)?;
        trace!(
            "Copied file: \"{}\" -> \"{}\"",
            file_to_copy.display(),
            snapshot_entry.display()
        );
        Ok(())
    }

    fn to_snapshot_path(&self, entry: &Path) -> io::Result<PathBuf> {
        let absolute_entry = fs::canonicalize(entry)?;
        let snapshot_relative_entry =
            Self::join_components_to_relative_path(absolute_entry.components());

        Ok(self.files.join(snapshot_relative_entry))
    }

    fn join_components_to_relative_path(components: Components) -> PathBuf {
        let mut path = PathBuf::new();

        for component in components {
            let component_to_join = match component {
                Component::Prefix(prefix) => Some(Self::get_disk_letter_from_prefix(prefix)),
                Component::RootDir => None,
                Component::Normal(comp) => Some(comp.to_owned()),
                _ => None,
            };

            if let Some(ccc) = component_to_join {
                path.push(ccc);
            }
        }

        path
    }

    fn get_disk_letter_from_prefix(prefix: PrefixComponent) -> OsString {
        match prefix.kind() {
            Prefix::Verbatim(prefix) => prefix.to_owned(),
            Prefix::VerbatimDisk(letter) | Prefix::Disk(letter) => {
                OsString::from(String::from_utf8_lossy(&[letter]).as_ref())
            }
            Prefix::DeviceNS(prefix) => prefix.to_owned(),
            Prefix::VerbatimUNC(first, second) | Prefix::UNC(first, second) => {
                PathBuf::from(first).join(second).as_os_str().to_owned()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile;

    #[test]
    #[cfg_attr(unix, ignore)]
    fn join_windows_verbatim_path() {
        let windows_path = Path::new(r"\\?\C:\dir_1\dir_2\file.txt");
        let rel_path = Snapshot::join_components_to_relative_path(windows_path.components());
        assert_eq!(rel_path, Path::new(r"C\dir_1\dir_2\file.txt"));
    }
    #[test]
    #[cfg_attr(unix, ignore)]
    fn join_windows_disk_path() {
        let windows_path = Path::new(r"C:\dir_1\file.txt");
        let rel_path = Snapshot::join_components_to_relative_path(windows_path.components());
        assert_eq!(rel_path, Path::new(r"C\dir_1\file.txt"));
    }

    #[test]
    #[cfg_attr(unix, ignore)]
    fn join_windows_disk_only_path() {
        let windows_path = Path::new(r"C:\");
        let rel_path = Snapshot::join_components_to_relative_path(windows_path.components());
        assert_eq!(rel_path, Path::new(r"C"));

        let windows_verbatim_path = Path::new(r"\\?\C:\");
        let rel_path =
            Snapshot::join_components_to_relative_path(windows_verbatim_path.components());
        assert_eq!(rel_path, Path::new(r"C"));
    }

    #[test]
    #[cfg_attr(windows, ignore)]
    fn join_unix_path() {
        let unix_path = Path::new("/dir_1/dir_2/file.txt");
        let rel_path = Snapshot::join_components_to_relative_path(unix_path.components());
        assert_eq!(rel_path, Path::new("dir_1/dir_2/file.txt"));
    }

    #[test]
    #[cfg_attr(windows, ignore)]
    fn join_unix_root_path_only() {
        let unix_path = Path::new("/");
        let rel_path = Snapshot::join_components_to_relative_path(unix_path.components());
        assert_eq!(rel_path, Path::new(""));
    }

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
    fn copy_files_from_invalid_path() {
        let root = tempfile::tempdir().unwrap();
        let snapshot = Snapshot::create(root.path()).unwrap();

        let result = snapshot.copy_files(Path::new("incorrect path"));
        assert!(result.is_err());
    }

    #[test]
    fn index_invalid_path() {
        let root = tempfile::tempdir().unwrap();
        let mut snapshot = Snapshot::create(root.path()).unwrap();

        let result = snapshot.index_files(Path::new("incorrect path"));
        assert!(result.is_err());
    }

    #[test]
    fn index_empty_folder() {
        let root = tempfile::tempdir().unwrap();
        let files = tempfile::tempdir().unwrap();
        let mut snapshot = Snapshot::create(root.path()).unwrap();

        snapshot.index_files(files.path()).unwrap();

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
