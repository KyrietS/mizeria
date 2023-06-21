use std::borrow::Borrow;
use std::fs::{self, create_dir, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

mod utils;
pub use crate::utils::*;

fn init_logger() {
    let mut builder = env_logger::Builder::new();
    builder.target(env_logger::Target::Stdout).try_init().ok();
}

fn create_snapshot(backup: &Path, files: &[&Path]) {
    create_snapshot_with_args(backup, files, &[]);
}

fn create_snapshot_with_args(backup: &Path, files: &[&Path], args: &[&str]) {
    let mut program_args = vec![String::from("backup"), backup.to_string_lossy().to_string()];

    for arg in args {
        program_args.push(arg.to_string());
    }

    for file in files {
        program_args.push(file.to_string_lossy().to_string());
    }
    init_logger(); // comment this line to see logs (warnings and errors) in the test output
    mizeria::run_program(program_args, &mut std::io::sink()).expect("program failed");
}

fn get_entry_from(folder: &Path) -> PathBuf {
    folder.read_dir().unwrap().next().unwrap().unwrap().path()
}

#[test]
fn create_snapshot_with_empty_folder() {
    let backup = tempfile::tempdir().unwrap();
    let files = tempfile::tempdir().unwrap();
    let files = files.path().join("dummy_dir");
    fs::create_dir(&files).unwrap();

    create_snapshot(backup.path(), &[files.as_path()]);

    // backup should have one entry (snapshot)
    assert_eq!(
        backup.path().read_dir().unwrap().count(),
        1,
        "backup folder should have only one entry (the snapshot)"
    );

    let snapshot = get_entry_from(backup.path());
    let snapshot = StubSnapshot::open(snapshot.as_path());
    // utils::assert_snapshot_exists(snapshot.as_path());

    let dummy_dir = utils::get_dir_by_name(snapshot.files.as_path(), "dummy_dir").unwrap();
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
    let snapshot_name = utils::generate_snapshot_name();
    create_snapshot(backup.path(), &[files.path()]);

    // snapshot
    let snapshot = get_entry_from(backup.path());
    let snapshot_index = snapshot.join("index.txt");
    let snapshot_index_content = fs::read_to_string(&snapshot_index).unwrap();

    let snapshot_files = snapshot.join("files");
    let snapshot_dummy_file =
        utils::get_file_by_name(snapshot_files.as_path(), "dummy_file.txt").unwrap();
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

    create_snapshot(backup.path(), &[files.path()]);
    create_snapshot(backup.path(), &[files.path()]);
    create_snapshot(backup.path(), &[files.path()]);

    let backup = backup.path().read_dir().unwrap();
    let snapshots: Vec<fs::DirEntry> = backup.filter_map(Result::ok).collect();

    assert_eq!(snapshots.len(), 3);

    utils::assert_snapshot_exists(snapshots[0].path().borrow());
    utils::assert_snapshot_exists(snapshots[1].path().borrow());
    utils::assert_snapshot_exists(snapshots[2].path().borrow());
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

    create_snapshot(backup.path(), &[path_1.path(), path_2.path()]);

    let snapshot = get_entry_from(backup.path());
    let snapshot = StubSnapshot::open(snapshot.as_path());

    let snapshot_path_1 = utils::get_dir_by_name(
        snapshot.files.as_path(),
        path_1.path().file_name().unwrap().to_str().unwrap(),
    )
    .unwrap();
    let snapshot_path_2 = utils::get_dir_by_name(
        snapshot.files.as_path(),
        path_2.path().file_name().unwrap().to_str().unwrap(),
    )
    .unwrap();
    assert_eq!(0, snapshot_path_1.read_dir().unwrap().count());
    assert_eq!(1, snapshot_path_2.read_dir().unwrap().count());

    let snapshot_dummy_file =
        utils::get_file_by_name(snapshot.files.as_path(), "dummy_file.txt").unwrap();
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
    assert_eq!(snapshot.index, expected_index_content);
}

#[test]
fn create_incremental_snapshot() {
    let backup = tempfile::tempdir().unwrap();
    let backup = backup.path();
    let files = tempfile::tempdir().unwrap();
    let files = files.path();
    let old_file = files.join("old_file.txt");
    let mod_file = files.join("mod_file.txt");
    let new_file = files.join("new_file.txt");

    File::create(&old_file).unwrap();
    File::create(&mod_file).unwrap();
    File::create(&new_file).unwrap();

    // Snapshot from the future. The idea is to pretend that the previous
    // snapshot has a newer version of a file compared to modification
    // time in the filestystem. Program will read this as the latest snapshot
    // and it will create incremental backup based on this.
    let future_datetime = time::OffsetDateTime::now_local().unwrap() + time::Duration::hours(1);
    let previous_snapshot_timestamp = utils::format_snapshot_name(future_datetime);
    let previous_snapshot_path = backup.join(&previous_snapshot_timestamp);
    fs::create_dir(&previous_snapshot_path).unwrap();
    fs::create_dir(previous_snapshot_path.join("files")).unwrap();
    let latest_index = File::create(previous_snapshot_path.join("index.txt")).unwrap();
    write!(
        &latest_index,
        "{future_ts} {}\n{future_ts} {}\n{past_ts} {}\n",
        files.canonicalize().unwrap().display(),
        old_file.canonicalize().unwrap().display(),
        mod_file.canonicalize().unwrap().display(),
        future_ts = previous_snapshot_timestamp,
        past_ts = utils::generate_snapshot_name()
    )
    .unwrap();

    let snapshot_name = utils::generate_snapshot_name();
    create_snapshot(backup, &[files]);
    let snapshot = StubSnapshot::open(backup.join(snapshot_name).as_path());

    let old_file_in_snapshot = utils::get_file_by_name(snapshot.files.as_path(), "old_file.txt");
    let new_file_in_snapshot = utils::get_file_by_name(snapshot.files.as_path(), "new_file.txt");

    assert!(old_file_in_snapshot.is_none()); // old_file.txt is not copied
    assert!(new_file_in_snapshot.is_some()); // new_file.txt is copied

    assert_eq!(4, snapshot.index.lines().count());

    assert!(snapshot.index_contains(
        previous_snapshot_timestamp.as_str(),
        files.canonicalize().unwrap().as_path()
    ));
    assert!(snapshot.index_contains(
        snapshot.timestamp.as_str(),
        mod_file.canonicalize().unwrap().as_path()
    ));
    assert!(snapshot.index_contains(
        snapshot.timestamp.as_str(),
        new_file.canonicalize().unwrap().as_path()
    ));
    assert!(snapshot.index_contains(
        previous_snapshot_timestamp.as_str(),
        old_file.canonicalize().unwrap().as_path()
    ));
}

#[test]
fn incremental_snapshot_with_no_changes() {
    let backup = tempfile::tempdir().unwrap();
    let backup = backup.path();
    let files = tempfile::tempdir().unwrap();
    let dir_to_backup = files.path();
    let file_to_backup = dir_to_backup.join("file_to_backup.txt");

    File::create(&file_to_backup).unwrap();

    let snapshot_timestamp = utils::generate_snapshot_name();
    let snapshot_path = backup.join(&snapshot_timestamp);
    fs::create_dir(&snapshot_path).unwrap();
    fs::create_dir(snapshot_path.join("files")).unwrap();
    let latest_index = File::create(snapshot_path.join("index.txt")).unwrap();
    write!(
        &latest_index,
        "{timestamp} {}\n{timestamp} {}\n",
        dir_to_backup.canonicalize().unwrap().display(),
        file_to_backup.canonicalize().unwrap().display(),
        timestamp = snapshot_timestamp,
    )
    .unwrap();

    let snapshot_name = utils::generate_snapshot_name();
    create_snapshot(backup, &[dir_to_backup]);
    let snapshot = StubSnapshot::open(backup.join(snapshot_name).as_path());

    // files folder is empty (no files were copied)
    assert_eq!(0, snapshot.files.read_dir().unwrap().count());

    // two entries were indexed
    assert_eq!(2, snapshot.index.lines().count());
    assert!(snapshot.index_contains(
        snapshot_timestamp.as_str(),
        dir_to_backup.canonicalize().unwrap().as_path()
    ));
    assert!(snapshot.index_contains(
        snapshot_timestamp.as_str(),
        file_to_backup.canonicalize().unwrap().as_path()
    ));
}

#[test]
fn force_full_snapshot() {
    let backup = tempfile::tempdir().unwrap();
    let backup = backup.path();
    let files = tempfile::tempdir().unwrap();
    let dir_to_backup = files.path();
    let file_to_backup = dir_to_backup.join("file_to_backup.txt");

    File::create(&file_to_backup).unwrap();

    let past_datetime = time::OffsetDateTime::now_local().unwrap() - time::Duration::hours(1);
    let previous_snapshot_timestamp = utils::format_snapshot_name(past_datetime);
    let previous_snapshot_path = backup.join(&previous_snapshot_timestamp);
    fs::create_dir(&previous_snapshot_path).unwrap();
    fs::create_dir(previous_snapshot_path.join("files")).unwrap();
    let latest_index = File::create(previous_snapshot_path.join("index.txt")).unwrap();
    write!(
        &latest_index,
        "{timestamp} {}\n{timestamp} {}\n",
        dir_to_backup.canonicalize().unwrap().display(),
        file_to_backup.canonicalize().unwrap().display(),
        timestamp = previous_snapshot_timestamp,
    )
    .unwrap();

    let new_snapshot_timestamp = utils::generate_snapshot_name();
    create_snapshot_with_args(backup, &[dir_to_backup], &["--full"]);
    let new_snapshot = StubSnapshot::open(backup.join(&new_snapshot_timestamp).as_path());

    assert!(utils::get_file_by_name(new_snapshot.files.as_path(), "file_to_backup.txt").is_some());

    // two entries were indexed
    assert_eq!(2, new_snapshot.index.lines().count());
    assert!(new_snapshot.index_contains(
        new_snapshot_timestamp.as_str(),
        dir_to_backup.canonicalize().unwrap().as_path()
    ));
    assert!(new_snapshot.index_contains(
        new_snapshot_timestamp.as_str(),
        file_to_backup.canonicalize().unwrap().as_path()
    ));
}

#[test]
#[cfg_attr(windows, ignore = "symlinks are not supported on windows")]
fn create_snapshot_with_symlinks() {
    // some dummy targets for the links
    let target_dir = tempfile::tempdir().unwrap();
    let target_dir = target_dir.path();
    let target_file = target_dir.join("dummy_file.txt");
    File::create(&target_file).unwrap();

    let backup = tempfile::tempdir().unwrap();
    let backup = backup.path();

    let files = tempfile::tempdir().unwrap();
    let files = files.path();
    let dir_link = files.join("dir_link");
    let file_link = files.join("file_link.txt");

    // create symlinks
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&target_dir, &dir_link).unwrap();
        std::os::unix::fs::symlink(&target_file, &file_link).unwrap();
    }
    #[cfg(windows)]
    {
        // creating symlinks on windows requires admin privileges
        std::os::windows::fs::symlink_dir(&target_dir, &dir_link).unwrap();
        std::os::windows::fs::symlink_file(&target_file, &file_link).unwrap();
    }

    create_snapshot(backup, &[files]);

    let snapshot = get_entry_from(backup);
    let snapshot = StubSnapshot::open(snapshot.as_path());

    // Assert index.txt
    assert_eq!(3, snapshot.index.lines().count());
    assert!(snapshot.index_contains(
        snapshot.timestamp.as_str(),
        files.canonicalize().unwrap().as_path()
    ));
    assert!(snapshot.index_contains(
        snapshot.timestamp.as_str(),
        dir_link.canonicalize().unwrap().as_path()
    ));
    assert!(snapshot.index_contains(
        snapshot.timestamp.as_str(),
        file_link.canonicalize().unwrap().as_path()
    ));

    // Assert copied files (symlinks)
    fn get_link_by_name(path: &Path, file_name: &str) -> Option<PathBuf> {
        for entry in WalkDir::new(path) {
            let entry = entry.unwrap().into_path();
            let entry_name = entry.file_name().unwrap().to_string_lossy();
            let entry_metadata = entry.symlink_metadata().unwrap();
            if entry_metadata.file_type().is_symlink() && entry_name == file_name {
                return Some(entry);
            }
        }
        return None;
    }

    // links were successfully copied into 'files'
    let snapshot_dir_link = get_link_by_name(snapshot.files.as_path(), "dir_link").unwrap();
    let snapshot_file_link = get_link_by_name(snapshot.files.as_path(), "file_link.txt").unwrap();

    let snapshot_dir_link_target = snapshot_dir_link.read_link().unwrap();
    let snapshot_file_link_target = snapshot_file_link.read_link().unwrap();

    // links point to the same values as original links
    assert_eq!(snapshot_dir_link_target, target_dir);
    assert_eq!(snapshot_file_link_target, target_file);
}

#[test]
fn create_snapshot_with_overlapping_paths() {
    let backup = tempfile::tempdir().unwrap();
    let backup = backup.path();

    let files = tempfile::tempdir().unwrap();
    let outer_dir = files.path().join("outer_dir");
    let outer_file = outer_dir.join("outer_file.txt");
    let inner_dir = outer_dir.join("inner_dir");
    let inner_file = inner_dir.join("inner_file.txt");
    create_dir(&outer_dir).unwrap();
    create_dir(&inner_dir).unwrap();
    File::create(&outer_file).unwrap();
    File::create(&inner_file).unwrap();

    create_snapshot(backup, &[outer_dir.as_path(), inner_dir.as_path()]);

    let snapshot = get_entry_from(backup);
    let snapshot = StubSnapshot::open(snapshot.as_path());

    // Assert index.txt
    assert_eq!(4, snapshot.index.lines().count());
    assert!(snapshot.index_contains_all(
        snapshot.timestamp.as_str(),
        &[
            outer_dir.canonicalize().unwrap().as_path(),
            outer_file.canonicalize().unwrap().as_path(),
            inner_dir.canonicalize().unwrap().as_path(),
            inner_file.canonicalize().unwrap().as_path()
        ]
    ));

    assert!(snapshot.find_dir("outer_dir").is_some());
    assert!(snapshot.find_dir("inner_dir").is_some());
    assert!(snapshot.find_file("outer_file.txt").is_some());
    assert!(snapshot.find_file("inner_file.txt").is_some());
}

#[test]
fn create_snapshot_from_duplicated_and_nonexistent_paths() {
    let backup = tempfile::tempdir().unwrap();
    let backup = backup.path();
    let path = tempfile::tempdir().unwrap();
    let path = path.path();
    let file = path.join("file.txt");
    File::create(&file).unwrap();
    let nonexistent_path = path.join("foobar");

    create_snapshot(backup, &[path, path, nonexistent_path.as_path(), path]);

    let snapshot = get_entry_from(backup);
    let snapshot = StubSnapshot::open(snapshot.as_path());

    assert_eq!(2, snapshot.index.lines().count());
    assert!(snapshot.index_contains(
        snapshot.timestamp.as_str(),
        path.canonicalize().unwrap().as_path()
    ));
    assert!(snapshot.index_contains(
        snapshot.timestamp.as_str(),
        file.canonicalize().unwrap().as_path()
    ));

    assert!(snapshot.find_file("file.txt").is_some());
}
