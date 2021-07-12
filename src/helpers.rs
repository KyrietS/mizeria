use path_absolutize::Absolutize;
use std::path::{Path, PathBuf};

#[derive(PartialEq, PartialOrd)]
pub struct SnapshotDateTime {
    inner: chrono::DateTime<chrono::Local>,
}

impl SnapshotDateTime {
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
    let snapshot_origin = origin.absolutize().unwrap();

    // remove leading '/'.
    let snapshot_origin = match snapshot_origin.strip_prefix("/") {
        Ok(not_absolute) => not_absolute,
        Err(_) => snapshot_origin.as_ref(),
    };

    // remove ':' from 'C:/folder'.
    let snapshot_origin = snapshot_origin.to_string_lossy().replace(":", "");

    snapshot.join("files").join(&snapshot_origin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_path_when_origin_is_absolute() {
        let origin = Path::new("/my_data/my_folder");
        let snapshot = Path::new("my_backup/files");

        let result = map_origin_to_snapshot_path(origin, snapshot);

        assert_ne!(result.to_str().unwrap(), origin.to_str().unwrap());
    }
}
