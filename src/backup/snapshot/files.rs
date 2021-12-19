use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Component, Components, Path, PathBuf, Prefix, PrefixComponent};
use std::{fs, io};

use walkdir::WalkDir;

use crate::result::IntegrityCheckResult;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub struct Files {
    root: PathBuf,
}

impl Files {
    pub fn new(location: PathBuf) -> Files {
        if !location.exists() {
            fs::create_dir(&location).expect("Error creating files dir");
        }
        Files { root: location }
    }

    pub fn check_integrity(
        location: PathBuf,
        indexed_files: Vec<&PathBuf>,
    ) -> IntegrityCheckResult {
        let mut index_set = HashSet::new();
        for indexed_file in indexed_files {
            let local_path = match Files::to_snapshot_path(&location, indexed_file.as_path()) {
                Ok(path) => path,
                Err(err) => return IntegrityCheckResult::UnexpectedError(format!("{}", err)),
            };
            index_set.insert(local_path);
        }

        if !location.exists() || !location.is_dir() {
            return IntegrityCheckResult::FilesFolderDoesntExist;
        }

        for entry in WalkDir::new(location).min_depth(1).follow_links(false) {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => return IntegrityCheckResult::UnexpectedError(format!("{}", e)),
            };

            let entry = entry.path();
            // TODO: in order to check if a file is indexed but not present I should
            // remove entry from hashmap rgith here and after WalkDir finishes
            // I should check if 'index_set' is empty. Every leftovoer is a candidate
            // to 'indexed but not present' integrity error.
            if !index_set.contains(entry) {
                return IntegrityCheckResult::EntryExistsButNotIndexed(entry.to_owned());
            }
        }

        IntegrityCheckResult::Success
    }

    pub fn copy_entry(&self, entry: &Path) -> Result<PathBuf> {
        let entry_meta = entry.symlink_metadata()?;
        let entry_type = entry_meta.file_type();

        return if entry_type.is_dir() {
            self.copy_dir_entry(entry)
        } else if entry_type.is_file() {
            self.copy_file_entry(entry)
        } else if entry_type.is_symlink() {
            #[cfg(windows)]
            {
                Err("Copying symlinks is not supported on Windows.".into())
            }
            #[cfg(unix)]
            {
                self.copy_link_entry(entry)
            }
        } else {
            Err(format!("Unknown entry type: {}", &entry.display()).into())
        };
    }

    fn copy_dir_entry(&self, dir_to_copy: &Path) -> Result<PathBuf> {
        let snapshot_entry = Files::to_snapshot_path(&self.root, dir_to_copy)?;
        fs::create_dir_all(&snapshot_entry)?;
        Ok(snapshot_entry)
    }

    fn copy_file_entry(&self, file_to_copy: &Path) -> Result<PathBuf> {
        let snapshot_entry = Files::to_snapshot_path(&self.root, file_to_copy)?;
        let snapshot_entry_parent = snapshot_entry.parent().ok_or("no parent")?;
        if !snapshot_entry_parent.exists() {
            fs::create_dir_all(snapshot_entry_parent)?;
        }
        fs::copy(file_to_copy, &snapshot_entry)?;
        Ok(snapshot_entry)
    }

    #[cfg(unix)]
    fn copy_link_entry(&self, link_to_copy: &Path) -> Result<PathBuf> {
        let link_parent = link_to_copy.parent().ok_or("no parent")?;
        let link_file_name = link_to_copy.file_name().ok_or("invalid file name")?;

        let snapshot_entry_parent = Files::to_snapshot_path(&self.root, link_parent)?;
        let snapshot_entry = snapshot_entry_parent.join(link_file_name);
        if !snapshot_entry_parent.exists() {
            fs::create_dir_all(snapshot_entry_parent)?;
        }
        let source = link_to_copy.read_link()?;
        std::os::unix::fs::symlink(source, &snapshot_entry)?;
        Ok(snapshot_entry)
    }

    fn to_snapshot_path(root: &Path, entry: &Path) -> io::Result<PathBuf> {
        let absolute_entry = fs::canonicalize(entry)?;
        let snapshot_relative_entry =
            Self::join_components_to_relative_path(absolute_entry.components());

        Ok(root.join(snapshot_relative_entry))
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

    #[test]
    fn copy_files_from_invalid_path() {
        let tempdir = tempfile::tempdir().unwrap();
        let invalid_file = tempdir.path().join("foobar");
        let files = Files {
            root: PathBuf::new(),
        };

        let result = files.copy_entry(&invalid_file);
        assert!(result.is_err());
    }

    #[test]
    #[cfg_attr(unix, ignore)]
    fn join_windows_verbatim_path() {
        let windows_path = Path::new(r"\\?\C:\dir_1\dir_2\file.txt");
        let rel_path = Files::join_components_to_relative_path(windows_path.components());
        assert_eq!(rel_path, Path::new(r"C\dir_1\dir_2\file.txt"));
    }
    #[test]
    #[cfg_attr(unix, ignore)]
    fn join_windows_disk_path() {
        let windows_path = Path::new(r"C:\dir_1\file.txt");
        let rel_path = Files::join_components_to_relative_path(windows_path.components());
        assert_eq!(rel_path, Path::new(r"C\dir_1\file.txt"));
    }

    #[test]
    #[cfg_attr(unix, ignore)]
    fn join_windows_disk_only_path() {
        let windows_path = Path::new(r"C:\");
        let rel_path = Files::join_components_to_relative_path(windows_path.components());
        assert_eq!(rel_path, Path::new(r"C"));

        let windows_verbatim_path = Path::new(r"\\?\C:\");
        let rel_path = Files::join_components_to_relative_path(windows_verbatim_path.components());
        assert_eq!(rel_path, Path::new(r"C"));
    }

    #[test]
    #[cfg_attr(windows, ignore)]
    fn join_unix_path() {
        let unix_path = Path::new("/dir_1/dir_2/file.txt");
        let rel_path = Files::join_components_to_relative_path(unix_path.components());
        assert_eq!(rel_path, Path::new("dir_1/dir_2/file.txt"));
    }

    #[test]
    #[cfg_attr(windows, ignore)]
    fn join_unix_root_path_only() {
        let unix_path = Path::new("/");
        let rel_path = Files::join_components_to_relative_path(unix_path.components());
        assert_eq!(rel_path, Path::new(""));
    }
}
