use std::fs::{self, File};

#[test]
#[ignore]
fn check_integrity_for_empty_snapshot() {
    let backup = tempfile::tempdir().unwrap();
    let backup = backup.path();
    let snapshot_name = "2021-07-15_18.34";
    let snapshot = backup.join(snapshot_name);
    fs::create_dir(&snapshot).unwrap();
    let index = snapshot.join("index.txt");
    let files = snapshot.join("files");
    fs::create_dir(files).unwrap();
    File::create(&index).unwrap();

    mizeria::run_program(vec!["snapshot", snapshot.to_str().unwrap()]).expect("program failes");
}
