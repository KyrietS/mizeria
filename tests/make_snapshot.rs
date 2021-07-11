use path_absolutize::Absolutize;
use regex::Regex;
use std::fs::{self, File};
use std::io::Write;

mod helpers {
    use tempfile::TempDir;

    pub fn run_program_with_args(backup: &TempDir, files: &TempDir) {
        let args = [
            backup.path().to_string_lossy().to_string(),
            files.path().to_string_lossy().to_string(),
        ];

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
}

#[test]
fn create_snapshot_with_empty_folder() {
    let backup = tempfile::tempdir().unwrap();
    let files = tempfile::tempdir().unwrap();

    println!("{}", backup.path().display());
    println!("{}", files.path().display());

    helpers::run_program_with_args(&backup, &files);

    // snapshot folder should be created
    let backup = backup.path().read_dir().unwrap();
    let snapshots: Vec<_> = backup.collect();
    assert_eq!(
        snapshots.len(),
        1,
        "backup folder should have only one entry (the snapshot)"
    );

    let snapshot = snapshots.first().unwrap().as_ref().unwrap().path();
    assert!(snapshot.is_dir(), "entry in backup should be a folder");

    let re = Regex::new(r"\d{4}-\d{2}-\d{2}_\d{2}\.\d{2}").unwrap();
    let snapshot_name = snapshot.file_name().unwrap().to_string_lossy().to_string();
    assert!(
        re.is_match(snapshot_name.as_str()),
        "snapshot folder name should match the pattern"
    );

    let snapshot_origin = mizeria::helpers::map_origin_to_snapshot_path(files.path(), &snapshot);
    assert!(snapshot_origin.is_dir());
    let is_snapshot_files_empty = snapshot_origin.read_dir().unwrap().count() == 0;
    assert!(is_snapshot_files_empty);

    // snapshot should have index.txt with one record
    let snapshot_index = snapshot.join("index.txt");
    assert!(snapshot_index.is_file());
    let snapshot_index_content = fs::read_to_string(snapshot_index.as_path()).unwrap();
    assert_eq!(
        snapshot_index_content,
        format!(
            "{} {}\n",
            snapshot_name,
            files.path().absolutize().unwrap().display()
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
    let snapshot_name = helpers::generate_snapshot_name();
    helpers::run_program_with_args(&backup, &files);

    // snapshot
    let snapshot = backup
        .path()
        .read_dir()
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let snapshot_index = snapshot.join("index.txt");
    let snapshot_index_content = fs::read_to_string(&snapshot_index).unwrap();
    let snapshot_dummy_file = mizeria::helpers::map_origin_to_snapshot_path(&dummy_file, &snapshot);
    let snapshot_dummy_file_content = fs::read_to_string(&snapshot_dummy_file).unwrap();

    assert!(snapshot_dummy_file.is_file());
    assert_eq!(snapshot_dummy_file_content, "hello world");
    assert_eq!(
        snapshot_index_content,
        format!(
            "{snap} {}\n{snap} {}\n",
            files.path().absolutize().unwrap().display(),
            dummy_file.absolutize().unwrap().display(),
            snap = snapshot_name,
        )
    );
}
