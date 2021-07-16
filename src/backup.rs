use std::{
    borrow::Borrow,
    path::{Path, PathBuf},
};

use snapshot::Snapshot;

mod snapshot;

pub struct Backup {
    location: PathBuf,
    snapshots: Vec<Snapshot>,
}

impl Backup {
    pub fn open(path: &Path) -> Result<Self, String> {
        let backup_root = path
            .read_dir()
            .or(Err("Folder with backup doesn't exist or isn't accessible"))?;

        let mut snapshots = vec![];
        for entry in backup_root.filter_map(Result::ok) {
            let snapshot = Snapshot::open(entry.path().borrow());
            match snapshot {
                Some(snapshot) => snapshots.push(snapshot),
                None => continue,
            }
        }
        snapshots.sort_unstable();

        Ok(Backup {
            location: path.to_owned(),
            snapshots,
        })
    }

    #[allow(dead_code)]
    pub fn latest_snapshot(&self) -> Option<&Snapshot> {
        self.snapshots.last()
    }

    pub fn add_snapshot(&mut self, files: &[PathBuf]) -> Result<(), String> {
        let mut snapshot = Snapshot::create(self.location.as_path())?;
        snapshot.index_files(files)?;
        snapshot.copy_files(files)?;
        println!("Created snapshot: {}", snapshot.name());

        self.snapshots.push(snapshot);
        Ok(())
    }
}
