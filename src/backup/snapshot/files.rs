use std::ffi::OsString;
use std::path::{Component, Components, Path, PathBuf, Prefix, PrefixComponent};
use std::{fs, io};

use log::{error, trace, warn};

pub struct Files {
    root: PathBuf,
}

impl Files {
    pub fn new(location: PathBuf) -> Files {
        Files { root: location }
    }

    pub fn copy_all<I: IntoIterator>(&self, files: I) -> Result<(), String>
    where
        I::Item: AsRef<Path>,
    {
        if !self.root.exists() {
            fs::create_dir(self.root.to_owned())
                .or(Err("Cannot create folder for files to backup"))?;
        }

        for entry in files {
            let entry = entry.as_ref();
            if entry.is_dir() {
                self.copy_dir_entry(&entry);
            } else if entry.is_file() {
                self.copy_file_entry(&entry);
            } else {
                warn!("Entry inaccessible: {}", &entry.display());
            }
        }

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

        Ok(self.root.join(snapshot_relative_entry))
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

        let result = files.copy_all(&[invalid_file]);
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
