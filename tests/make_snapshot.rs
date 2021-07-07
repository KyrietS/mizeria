use mizeria::run_program;
use regex::Regex;
use temp_testdir::TempDir;

#[test]
fn create_empty_snapshot() {
    let backup_dir = TempDir::default();
    let args = [
        backup_dir.to_string_lossy().to_string(),
        String::from("foo"),
    ];

    run_program(args).expect("program failed");

    let backup = backup_dir.read_dir().unwrap();
    let snapshots: Vec<_> = backup.collect();
    assert_eq!(
        snapshots.len(),
        1,
        "backup folder should have only one entry (the snapshot)"
    );

    let snapshot = snapshots.first().unwrap().as_ref().unwrap();
    let file_type = snapshot.file_type().unwrap();
    assert!(file_type.is_dir(), "entry in backup should be a folder");

    let re = Regex::new(r"\d{4}-\d{2}-\d{2}_\d{2}\.\d{2}").unwrap();
    let snapshot_name = snapshot.file_name().to_string_lossy().to_string();
    assert!(
        re.is_match(snapshot_name.as_str()),
        "snapshot folder name should match the pattern"
    );
}
