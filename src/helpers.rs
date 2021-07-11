use path_absolutize::Absolutize;
use std::path::{Path, PathBuf};

#[derive(PartialEq, PartialOrd)]
pub struct SnapshotDateTime {
    inner: chrono::DateTime<chrono::Local>,
}

impl SnapshotDateTime {
    pub fn now() -> SnapshotDateTime {
        SnapshotDateTime {
            inner: chrono::offset::Local::now(),
        }
    }
}

impl ToString for SnapshotDateTime {
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

pub fn map_origin_to_snapshot_path(origin: &Path, snapshot: &Path) -> PathBuf {
    let origin_prepared = origin
        .absolutize()
        .unwrap()
        .to_string_lossy()
        .replace(":", "");
    snapshot.join("files").join(origin_prepared)
}
