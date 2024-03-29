use std::{
    fs,
    path::{Path, PathBuf},
};

use regex::Regex;
use walkdir::WalkDir;

pub fn get_current_time() -> time::PrimitiveDateTime {
    let time_with_offset =
        time::OffsetDateTime::now_local().unwrap_or(time::OffsetDateTime::now_utc());
    time::PrimitiveDateTime::new(time_with_offset.date(), time_with_offset.time())
}

pub fn generate_snapshot_name() -> String {
    format_snapshot_name(get_current_time())
}

pub fn format_snapshot_name(datetime: time::PrimitiveDateTime) -> String {
    let date = datetime.date();
    let time = datetime.time();
    format!(
        "{}-{:02}-{:02}_{:02}.{:02}",
        date.year(),
        date.month() as u8,
        date.day(),
        time.hour(),
        time.minute()
    )
}

pub fn get_dir_by_name(path: &Path, dir_name: &str) -> Option<PathBuf> {
    for entry in WalkDir::new(path) {
        let entry = entry.unwrap().into_path();
        let entry_name = entry.file_name().unwrap().to_string_lossy().to_string();
        if entry.is_dir() && entry_name == dir_name {
            return Some(entry);
        }
    }
    return None;
}

pub fn get_file_by_name(path: &Path, file_name: &str) -> Option<PathBuf> {
    for entry in WalkDir::new(path) {
        let entry = entry.unwrap().into_path();
        let entry_name = entry.file_name().unwrap().to_string_lossy();
        if entry.is_file() && entry_name == file_name {
            return Some(entry);
        }
    }
    return None;
}

pub fn assert_snapshot_exists(snapshot: &Path) {
    // snapshot is a folder
    assert!(
        snapshot.is_dir(),
        "snapshot \"{}\" should be a dir",
        snapshot.display()
    );

    // snapshot has a valid name
    let re = Regex::new(r"\d{4}-\d{2}-\d{2}_\d{2}\.\d{2}").unwrap();
    let snapshot_name = snapshot.file_name().unwrap().to_string_lossy().to_string();
    assert!(
        re.is_match(snapshot_name.as_str()),
        "snapshot folder name should match the pattern"
    );

    // snapshot has a 'files' folder
    let snapshot_files = snapshot.join("files");
    assert!(snapshot_files.is_dir());

    // snapshot has an 'index.txt' file
    let snapshot_index = snapshot.join("index.txt");
    assert!(snapshot_index.is_file());
}

pub struct StubSnapshot {
    pub timestamp: String,
    pub index: String,
    pub files: PathBuf,
}

impl StubSnapshot {
    pub fn open(snapshot: &Path) -> StubSnapshot {
        assert_snapshot_exists(snapshot);
        let timestamp = snapshot.file_name().unwrap().to_string_lossy().to_string();
        let index = snapshot.join("index.txt");
        let index = fs::read_to_string(index).unwrap();
        let files = snapshot.join("files");
        StubSnapshot {
            timestamp,
            index,
            files,
        }
    }

    pub fn index_contains(&self, timestamp: &str, path: &Path) -> bool {
        let path = path.canonicalize().unwrap();
        let entry = format!("{} {}", timestamp, path.to_string_lossy());
        let lines: Vec<&str> = self.index.lines().collect();
        match lines.contains(&entry.as_str()) {
            true => true,
            false => {
                println!("index: {:?}", lines);
                println!("entry: {:?}", entry);
                false
            }
        }
    }

    pub fn index_contains_all(&self, timestamp: &str, paths: &[&Path]) -> bool {
        paths.iter().all(|p| self.index_contains(timestamp, p))
    }

    pub fn find_file(&self, file_name: &str) -> Option<PathBuf> {
        get_file_by_name(&self.files, file_name)
    }

    pub fn find_dir(&self, dir_name: &str) -> Option<PathBuf> {
        get_dir_by_name(&self.files, dir_name)
    }
}
