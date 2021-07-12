use path_absolutize::Absolutize;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub mod helpers;

struct Args {
    backup: PathBuf,
    files: PathBuf,
}

pub fn run_program<C: IntoIterator>(args: C) -> Result<(), String>
where
    C::Item: AsRef<OsStr>,
{
    let args: Vec<String> = args
        .into_iter()
        .map(|e: _| e.as_ref().to_string_lossy().to_string())
        .collect();

    let args = parse_args(&args)?;
    println!("Folder where backup will be stored:");
    println!("  {}", args.backup.to_string_lossy());
    println!("Folder that will be backed up:");
    println!("  {}", args.files.to_string_lossy());

    let snapshot = add_snapshot(&args.backup, &args.files)?;
    println!("Snapshot created at: {}", snapshot.to_string_lossy());
    Ok(())
}

fn parse_args(args: &[String]) -> Result<Args, &'static str> {
    let mut args_iter = args.iter();
    let backup = args_iter.next().ok_or("Backup location is not specified")?;
    let files = args_iter.next().ok_or("Target location is not specified")?;

    Ok(Args {
        backup: PathBuf::from(backup),
        files: PathBuf::from(files),
    })
}

fn add_snapshot(backup_path: &Path, files: &Path) -> Result<PathBuf, String> {
    if !backup_path.is_dir() {
        return Err("Folder with backup does not exist or is not accessible".into());
    }

    let mut snapshot_date_time = helpers::SnapshotDateTime::now();
    let snapshot_path = loop {
        let snapshot_name = snapshot_date_time.to_string();
        let snapshot_path = backup_path.join(snapshot_name);
        if !snapshot_path.exists() {
            break snapshot_path;
        }
        snapshot_date_time = snapshot_date_time.get_next();
    };

    fs::create_dir(&snapshot_path).or(Err("Cannot create directory for a snapshot"))?;

    let snapshot_name = snapshot_date_time.to_string();
    make_index(&snapshot_path, files, &snapshot_name)?;
    copy_files(&snapshot_path, files)?;

    Ok(snapshot_path)
}

fn copy_files(snapshot: &Path, files: &Path) -> Result<(), String> {
    let snapshot_files = snapshot.join("files");
    fs::create_dir(snapshot_files).or(Err("Cannot create directory for snapshot files"))?;

    for entry in WalkDir::new(&files) {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Error: {}", e);
                continue;
            }
        };

        let origin = entry.path();
        let snapshot_origin = helpers::map_origin_to_snapshot_path(origin, snapshot);

        if origin.is_dir() {
            let result = fs::create_dir_all(&snapshot_origin);
            if let Err(e) = result {
                eprintln!("Cannot create directory at: {}", snapshot_origin.display());
                eprintln!("{}", e);
            }
        } else if origin.is_file() {
            let result = fs::copy(origin, snapshot_origin);
            if let Err(e) = result {
                eprintln!("Cannot copy file from: {}", origin.display());
                eprintln!("{}", e);
            }
        } else {
            eprintln!("Warning: unknown file type at: {}", origin.display());
        }
    }

    Ok(())
}

fn make_index(snapshot: &Path, files: &Path, snapshot_name: &str) -> Result<(), String> {
    let snapshot_index = snapshot.join("index.txt");
    println!("Saving index to: {}", snapshot_index.display());
    let mut snapshot_index = File::create(snapshot_index).or(Err("Cannot create index file"))?;

    for entry in WalkDir::new(&files) {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Error: {}", e);
                continue;
            }
        };
        let file_path = entry.path().absolutize().unwrap();
        writeln!(snapshot_index, "{} {}", snapshot_name, file_path.display()).unwrap();
    }
    Ok(())
}

#[cfg(test)]
mod index_tests {
    use std::fs;

    use super::*;
    use tempfile::tempdir;

    #[test]
    fn index_empty_folder() {
        let snapshot_name = helpers::SnapshotDateTime::now().to_string();
        let snapshot = tempdir().unwrap();
        let files = tempdir().unwrap();

        make_index(
            &snapshot.path().to_path_buf(),
            &files.path().to_path_buf(),
            &snapshot_name,
        )
        .unwrap();

        let index = snapshot.path().join("index.txt");
        let index_content = fs::read_to_string(&index).unwrap();

        assert_eq!(
            index_content,
            format!(
                "{} {}\n",
                snapshot_name,
                files.path().absolutize().unwrap().display()
            )
        )
    }

    #[test]
    fn index_folder_with_file() {
        let snapshot_name = helpers::SnapshotDateTime::now().to_string();
        let snapshot = tempdir().unwrap();
        let files = tempdir().unwrap();
        let dummy_file = files.path().join("dummy_file.txt");
        File::create(&dummy_file)
            .unwrap()
            .write_all(b"hello world")
            .unwrap();

        make_index(
            &snapshot.path().to_path_buf(),
            &files.path().to_path_buf(),
            &snapshot_name,
        )
        .unwrap();

        let index = snapshot.path().join("index.txt");
        let index_content = fs::read_to_string(&index).unwrap();

        assert_eq!(
            index_content,
            format!(
                "{snap} {}\n{snap} {}\n",
                files.path().absolutize().unwrap().display(),
                dummy_file.absolutize().unwrap().display(),
                snap = snapshot_name,
            )
        )
    }
}

#[cfg(test)]
mod snapshot_tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn backup_folder_does_not_exist() {
        let backup = PathBuf::from("nonexistent");
        let files = PathBuf::from("nonexistent");

        let snapshot = add_snapshot(&backup, &files);
        assert!(snapshot.is_err());
        assert_eq!(
            snapshot.unwrap_err(),
            "Folder with backup does not exist or is not accessible"
        );
    }

    #[test]
    fn snapshot_path_returned() {
        let backup = tempdir().unwrap();
        let files = tempdir().unwrap();
        let current_datetime = helpers::SnapshotDateTime::now().to_string();
        let expected_snapshot_path = backup.path().join(current_datetime);
        let snapshot =
            add_snapshot(&backup.path().to_path_buf(), &files.path().to_path_buf()).unwrap();
        assert_eq!(snapshot, expected_snapshot_path);
    }

    #[test]
    fn snapshot_dir_created() {
        let backup = tempdir().unwrap();
        let files = tempdir().unwrap();
        println!("{}", backup.path().display());
        let snapshot =
            add_snapshot(&backup.path().to_path_buf(), &files.path().to_path_buf()).unwrap();
        assert!(snapshot.is_dir());
    }

    #[test]
    fn snapshot_dir_already_exists() {
        let backup = tempdir().unwrap();
        let files = tempdir().unwrap();
        let current_datetime = helpers::SnapshotDateTime::now();
        let taken_snapshot_path = backup.path().join(&current_datetime.to_string());
        fs::create_dir(taken_snapshot_path).unwrap();

        let expected_snapshot_path = current_datetime.get_next().to_string();

        let snapshot =
            add_snapshot(&backup.path().to_path_buf(), &files.path().to_path_buf()).unwrap();
        assert!(snapshot.is_dir());
        assert!(snapshot
            .to_str()
            .unwrap()
            .contains(expected_snapshot_path.as_str()));
    }
}

#[cfg(test)]
mod args_parser_tests {
    use super::*;

    #[test]
    fn no_args() {
        let args: Vec<String> = vec![];
        let args = parse_args(&args);
        assert!(args.is_err());
        assert_eq!(args.err().unwrap(), "Backup location is not specified");
    }

    #[test]
    fn one_arg() {
        let first_argument = String::from("one");
        let args: Vec<String> = vec![first_argument];
        let args = parse_args(&args);
        assert!(args.is_err());
        assert_eq!(args.err().unwrap(), "Target location is not specified");
    }

    #[test]
    fn two_args() {
        let first_argument = String::from("one");
        let second_argument = String::from("two");
        let args: Vec<String> = vec![first_argument, second_argument];
        let args = parse_args(&args).unwrap();
        assert_eq!(args.backup.to_string_lossy(), "one");
        assert_eq!(args.files.to_string_lossy(), "two");
    }

    #[test]
    fn more_than_2_args_are_ignored() {
        let args = vec!["1".to_string(), "2".to_string(), "3".to_string()];
        let args = parse_args(&args).unwrap();
        assert_eq!(args.backup.to_string_lossy(), "1");
        assert_eq!(args.files.to_string_lossy(), "2");
    }
}
