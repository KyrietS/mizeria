use mizeria;
use path_absolutize::Absolutize;
use regex::Regex;
use std::fs::{self, File};
use std::io::Write;
use tempfile::{tempdir, TempDir};

fn run_program_with_args(backup: &TempDir, files: &TempDir) {
    let args = [
        backup.path().to_string_lossy().to_string(),
        files.path().to_string_lossy().to_string(),
    ];

    mizeria::run_program(args).expect("program failed");
}

#[test]
fn create_empty_snapshot() {
    let backup = tempdir().unwrap();
    let files = tempdir().unwrap();

    println!("{}", backup.path().display());
    println!("{}", files.path().display());

    run_program_with_args(&backup, &files);

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

    // snapshot should have empty 'files' directory
    let snapshot_files = snapshot.join("files");
    assert!(snapshot_files.is_dir());
    let is_snapshot_files_empty = snapshot_files.read_dir().unwrap().count() == 0;
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
#[ignore = "copying files not yet implemented"]
fn create_snapshot_with_one_file() {
    let backup = tempdir().unwrap();
    let files = tempdir().unwrap();

    // dummy file to backup
    let dummy_file = files.path().join("dummy_file.txt");
    let dummy_file_absolute = dummy_file
        .absolutize()
        .unwrap()
        .to_string_lossy()
        .to_string();
    File::create(&dummy_file)
        .unwrap()
        .write_all(b"hello world")
        .unwrap();

    // run program
    let snapshot_name = mizeria::get_date_time();
    run_program_with_args(&backup, &files);

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
    let snapshot_files = snapshot.join("files");
    let snapshot_dummy_file = snapshot_files.join("dummy_file.txt");
    let snapshot_dummy_file_content = fs::read_to_string(&snapshot_dummy_file).unwrap();

    assert!(snapshot_dummy_file.is_file());
    assert_eq!(snapshot_dummy_file_content, "hello world");
    assert_eq!(
        snapshot_index_content,
        format!("{} {}", snapshot_name, dummy_file_absolute)
    );
}
