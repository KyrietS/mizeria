use regex::Regex;
use std::borrow::Borrow;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use walkdir::WalkDir;

pub fn create_snapshot(backup: &TempDir, files: &[&Path]) {
    let mut args = vec![
        String::from("backup"),
        backup.path().to_string_lossy().to_string(),
    ];

    for file in files {
        args.push(file.to_string_lossy().to_string());
    }

    mizeria::run_program(args).expect("program failed");
}

pub fn generate_snapshot_name() -> String {
    use chrono::{Datelike, Timelike};
    let local = chrono::offset::Local::now();
    let date = local.date();
    let time = local.time();
    format!(
        "{}-{:02}-{:02}_{:02}.{:02}",
        date.year(),
        date.month(),
        date.day(),
        time.hour(),
        time.minute()
    )
}

pub fn get_entry_from(folder: &Path) -> PathBuf {
    folder.read_dir().unwrap().next().unwrap().unwrap().path()
}

struct StubSnapshot {
    timestamp: String,
    index: String,
    files: PathBuf,
}

impl StubSnapshot {
    fn open(snapshot: &Path) -> StubSnapshot {
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
}

fn assert_snapshot_exists(snapshot: &Path) {
    // snapshot is a folder
    assert!(snapshot.is_dir(), "snapshot should be a dir");

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

fn get_dir_by_name(path: &Path, dir_name: &str) -> PathBuf {
    for entry in WalkDir::new(path) {
        let entry = entry.unwrap().into_path();
        let entry_name = entry.file_name().unwrap().to_string_lossy().to_string();
        if entry.is_dir() && entry_name == dir_name {
            return entry;
        }
    }
    panic!("Dir cannot be found under specified path");
}

fn get_file_by_name(path: &Path, file_name: &str) -> PathBuf {
    for entry in WalkDir::new(path) {
        let entry = entry.unwrap().into_path();
        let entry_name = entry.file_name().unwrap().to_string_lossy();
        if entry.is_file() && entry_name == file_name {
            return entry;
        }
    }
    panic!("File cannot be found under specified path");
}

#[test]
fn create_snapshot_with_empty_folder() {
    let backup = tempfile::tempdir().unwrap();
    let files = tempfile::tempdir().unwrap();
    let files = files.path().join("dummy_dir");
    fs::create_dir(&files).unwrap();

    create_snapshot(&backup, &[files.as_path()]);

    // backup should have one entry (snapshot)
    assert_eq!(
        backup.path().read_dir().unwrap().count(),
        1,
        "backup folder should have only one entry (the snapshot)"
    );

    let snapshot = get_entry_from(backup.path());
    let snapshot = StubSnapshot::open(snapshot.as_path());
    // assert_snapshot_exists(snapshot.as_path());

    let dummy_dir = get_dir_by_name(snapshot.files.as_path(), "dummy_dir");
    assert_eq!(0, dummy_dir.read_dir().unwrap().count()); // empty dir

    // snapshot should have index.txt with one record
    assert_eq!(
        snapshot.index,
        format!(
            "{} {}\n",
            snapshot.timestamp,
            files.canonicalize().unwrap().display()
        )
    );
}

#[test]
fn create_snapshot_with_one_file() {
    let backup = tempfile::tempdir().unwrap();
    let files = tempfile::tempdir().unwrap();

    // dummy file to backup
    let dummy_file = files.path().join("dummy_file.txt");
    File::create(&dummy_file)
        .unwrap()
        .write_all(b"hello world")
        .unwrap();

    // run program
    let snapshot_name = generate_snapshot_name();
    create_snapshot(&backup, &[files.path()]);

    // snapshot
    let snapshot = get_entry_from(backup.path());
    let snapshot_index = snapshot.join("index.txt");
    let snapshot_index_content = fs::read_to_string(&snapshot_index).unwrap();

    let snapshot_files = snapshot.join("files");
    let snapshot_dummy_file = get_file_by_name(snapshot_files.as_path(), "dummy_file.txt");
    let snapshot_dummy_file_content = fs::read_to_string(&snapshot_dummy_file).unwrap();

    assert!(snapshot_dummy_file.is_file());
    assert_eq!(snapshot_dummy_file_content, "hello world");
    assert_eq!(
        snapshot_index_content,
        format!(
            "{snap} {}\n{snap} {}\n",
            files.path().canonicalize().unwrap().display(),
            dummy_file.canonicalize().unwrap().display(),
            snap = snapshot_name,
        )
    );
}

#[test]
fn create_three_snapshots_one_after_another() {
    let backup = tempfile::tempdir().unwrap();
    let files = tempfile::tempdir().unwrap();

    create_snapshot(&backup, &[files.path()]);
    create_snapshot(&backup, &[files.path()]);
    create_snapshot(&backup, &[files.path()]);

    let backup = backup.path().read_dir().unwrap();
    let snapshots: Vec<fs::DirEntry> = backup.filter_map(Result::ok).collect();

    assert_eq!(snapshots.len(), 3);

    assert_snapshot_exists(snapshots[0].path().borrow());
    assert_snapshot_exists(snapshots[1].path().borrow());
    assert_snapshot_exists(snapshots[2].path().borrow());
}

#[test]
fn create_snapshot_from_two_paths() {
    let backup = tempfile::tempdir().unwrap();
    let path_1 = tempfile::tempdir().unwrap();
    let path_2 = tempfile::tempdir().unwrap();
    let path_2_file = path_2.path().join("dummy_file.txt");
    File::create(&path_2_file)
        .unwrap()
        .write_all(b"hello world")
        .unwrap();

    create_snapshot(&backup, &[path_1.path(), path_2.path()]);

    let snapshot = get_entry_from(backup.path());
    let snapshot = StubSnapshot::open(snapshot.as_path());

    let snapshot_path_1 = get_dir_by_name(
        snapshot.files.as_path(),
        path_1.path().file_name().unwrap().to_str().unwrap(),
    );
    let snapshot_path_2 = get_dir_by_name(
        snapshot.files.as_path(),
        path_2.path().file_name().unwrap().to_str().unwrap(),
    );
    assert_eq!(0, snapshot_path_1.read_dir().unwrap().count());
    assert_eq!(1, snapshot_path_2.read_dir().unwrap().count());

    let snapshot_dummy_file = get_file_by_name(snapshot.files.as_path(), "dummy_file.txt");
    assert_eq!(
        "hello world",
        fs::read_to_string(snapshot_dummy_file).unwrap()
    );

    let expected_index_content = format!(
        "{timestamp} {}\n{timestamp} {}\n{timestamp} {}\n",
        path_1.path().canonicalize().unwrap().display(),
        path_2.path().canonicalize().unwrap().display(),
        path_2_file.as_path().canonicalize().unwrap().display(),
        timestamp = snapshot.timestamp,
    );

    assert_eq!(3, snapshot.index.lines().count());
    assert_eq!(
        snapshot.index.lines().count(),
        expected_index_content.lines().count()
    );
    assert_eq!(snapshot.index, expected_index_content);
}
