use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use mizeria::result::IntegrityCheckResult;

struct ProgramOutput {
    buffer: Vec<u8>,
}
impl Write for ProgramOutput {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buffer.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.buffer.flush()
    }
}
impl ProgramOutput {
    fn new() -> Self {
        ProgramOutput { buffer: Vec::new() }
    }
}
impl ToString for ProgramOutput {
    fn to_string(&self) -> String {
        String::from_utf8(self.buffer.clone()).expect("Invalid UTF-8")
    }
}

fn init_logger() {
    let mut builder = env_logger::Builder::new();
    builder.target(env_logger::Target::Stdout).try_init().ok();
}

fn check_snapshot_integrity(snapshot_path: &Path) -> ProgramOutput {
    check_snapshot_integrity_with_args(snapshot_path, &[])
}

fn check_snapshot_integrity_with_args(snapshot_path: &Path, args: &[&str]) -> ProgramOutput {
    let mut program_args = vec![
        String::from("snapshot"),
        snapshot_path.to_string_lossy().to_string(),
    ];
    for arg in args {
        program_args.push(arg.to_string());
    }

    init_logger();

    let mut output = ProgramOutput::new();
    mizeria::run_program(program_args, &mut output).expect("program failed");
    return output;
}

fn expect_result(output: ProgramOutput, result: IntegrityCheckResult) {
    let output = output.to_string();
    let expected_msg = result.to_string();
    assert!(
        output.contains(expected_msg.as_str()),
        "Expected result message: '{}' not found in: '{}'",
        expected_msg,
        output
    );
}

#[test]
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

    let output = check_snapshot_integrity(snapshot.as_path());
    expect_result(output, IntegrityCheckResult::Success);
}

#[test]
fn check_integrity_for_snapshot_with_invalid_name() {
    let backup = tempfile::tempdir().unwrap();
    let backup = backup.path();
    let snapshot_name = "2021-07-33_18.34";
    let snapshot = backup.join(snapshot_name);
    fs::create_dir(&snapshot).unwrap();
    let index = snapshot.join("index.txt");
    let files = snapshot.join("files");
    fs::create_dir(files).unwrap();
    File::create(&index).unwrap();

    let output = check_snapshot_integrity(snapshot.as_path());
    expect_result(output, IntegrityCheckResult::SnapshotNameIsInvalidTimestamp);
}

#[test]
fn check_integrity_for_snapshot_without_index() {
    let backup = tempfile::tempdir().unwrap();
    let backup = backup.path();
    let snapshot_name = "2021-07-15_18.34";
    let snapshot = backup.join(snapshot_name);
    fs::create_dir(&snapshot).unwrap();
    let files = snapshot.join("files");
    fs::create_dir(files).unwrap();

    let output = check_snapshot_integrity(snapshot.as_path());
    expect_result(output, IntegrityCheckResult::IndexFileDoesntExist);
}

#[test]
fn check_integrity_for_snapshot_without_files_folder() {
    let backup = tempfile::tempdir().unwrap();
    let backup = backup.path();
    let snapshot_name = "2021-07-15_18.34";
    let snapshot = backup.join(snapshot_name);
    fs::create_dir(&snapshot).unwrap();
    let index = snapshot.join("index.txt");
    File::create(&index).unwrap();

    let output = check_snapshot_integrity(snapshot.as_path());
    expect_result(output, IntegrityCheckResult::FilesFolderDoesntExist);
}

#[test]
fn check_integrity_for_snapshot_with_file_present_but_not_indexed() {
    let backup = tempfile::tempdir().unwrap();
    let backup = backup.path();
    let snapshot_name = "2021-07-15_18.34";
    let snapshot = backup.join(snapshot_name);
    fs::create_dir(&snapshot).unwrap();
    // empty index
    let index = snapshot.join("index.txt");
    File::create(&index).unwrap();
    let files = snapshot.join("files");
    fs::create_dir(&files).unwrap();
    let my_file = files.join("my_file.txt");
    File::create(&my_file).unwrap();

    let output = check_snapshot_integrity(snapshot.as_path());
    expect_result(
        output,
        IntegrityCheckResult::EntryExistsButNotIndexed(my_file),
    );
}
