use std::fs::{self, File};
use std::io::Write;

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

    let mut output = ProgramOutput::new();

    mizeria::run_program(vec!["snapshot", snapshot.to_str().unwrap()], &mut output)
        .expect("program failed");

    assert_eq!(output.to_string(), "Hello!");
}
