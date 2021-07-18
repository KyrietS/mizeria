use std::{
    borrow::Borrow,
    path::{Path, PathBuf},
};

use log::debug;
use snapshot::{Snapshot, SnapshotPreview};

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
            let snapshot = Snapshot::open_preview(entry.path().borrow());

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

    fn latest_snapshot(&self) -> Option<&SnapshotPreview> {
        self.snapshots.last()
    }

    pub fn add_snapshot(&mut self, files: &[PathBuf], incremental: bool) -> Result<()> {
        let mut new_snapshot = Snapshot::create(self.location.as_path())?;

        if incremental {
            debug!("Incremental snapshot will be performed");
            let latest_snapshot = self.latest_snapshot();
            new_snapshot.set_base_snapshot(latest_snapshot);
        } else {
            debug!("Full snapshot will be performed");
        }

        new_snapshot.backup_files(files)?;

        println!("Created snapshot: {}", new_snapshot.name());
        self.snapshots.push(new_snapshot.as_preview());

        Ok(())
    }
}
