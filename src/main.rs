use std::{fs::create_dir, path::PathBuf};

use chrono::{Datelike, Timelike};

struct Args {
    backup: PathBuf,
    files: PathBuf,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let result_code = match run_program(&args) {
        Ok(_) => 0,
        Err(msg) => {
            eprintln!("{}", msg);
            1
        }
    };

    std::process::exit(result_code);
}

fn run_program(args: &Vec<String>) -> Result<(), String> {
    let args = parse_args(&args)?;
    println!("Folder where backup will be stored:");
    println!("  {}", args.backup.to_string_lossy());
    println!("Folder that will be backed up:");
    println!("  {}", args.files.to_string_lossy());

    let snapshot = add_snapshot(&args.backup)?;
    println!("Snapshot created at: {}", snapshot.to_string_lossy());
    Ok(())
}

fn parse_args(args: &Vec<String>) -> Result<Args, &'static str> {
    let mut args_iter = args.iter();
    args_iter
        .next()
        .ok_or("First argument should be path to binary")?;
    let backup = args_iter.next().ok_or("Backup location is not specified")?;
    let files = args_iter.next().ok_or("Target location is not specified")?;

    Ok(Args {
        backup: PathBuf::from(backup),
        files: PathBuf::from(files),
    })
}

fn add_snapshot(backup_path: &PathBuf) -> Result<PathBuf, String> {
    if !backup_path.is_dir() {
        Err("Folder with backup does not exist or is not accessible")?;
    }

    let snapshot_name = get_date_time();
    let snapshot_path = backup_path.join(&snapshot_name);

    if snapshot_path.exists() {
        Err(format!("Snapshot name is already used: {}", snapshot_name))?;
    }

    create_dir(&snapshot_path).or(Err("Cannot create directory for a snapshot"))?;

    Ok(snapshot_path)
}

fn get_date_time() -> String {
    let datetime = chrono::offset::Local::now();
    let date = datetime.date().naive_local();
    let time = datetime.time();

    format!(
        "{}-{:02}-{:02}_{:02}.{:02}",
        date.year(),
        date.month(),
        date.day(),
        time.hour(),
        time.minute()
    )
}

#[cfg(test)]
mod snapshot_tests {
    use super::*;
    use temp_testdir::TempDir;

    #[test]
    fn backup_folder_does_not_exist() {
        let backup = PathBuf::from("nonexistent");

        let snapshot = add_snapshot(&backup);
        assert!(snapshot.is_err());
        assert_eq!(
            snapshot.unwrap_err(),
            "Folder with backup does not exist or is not accessible"
        );
    }

    #[test]
    fn snapshot_path_returned() {
        let tempdir = TempDir::default();
        let current_datetime = get_date_time();
        let expected_snapshot_path = tempdir.join(current_datetime);
        let snapshot = add_snapshot(&tempdir.to_path_buf()).unwrap();
        assert_eq!(snapshot, expected_snapshot_path);
    }

    #[test]
    fn snapshot_dir_created() {
        let tempdir = TempDir::default();
        let snapshot = add_snapshot(&tempdir.to_path_buf()).unwrap();
        assert!(snapshot.is_dir());
    }

    #[test]
    fn snapshot_dir_already_exists() {
        let tempdir = TempDir::default();
        let current_datetime = get_date_time();
        let expected_snapshot_path = tempdir.join(&current_datetime);
        create_dir(&expected_snapshot_path).unwrap();

        let snapshot = add_snapshot(&tempdir.to_path_buf());
        assert!(snapshot.is_err());
        assert_eq!(
            snapshot.unwrap_err(),
            format!("Snapshot name is already used: {}", &current_datetime)
        );
    }
}

#[cfg(test)]
mod args_parser_tests {
    use super::*;

    #[test]
    fn empty_args() {
        let args: Vec<String> = vec![];
        let args = parse_args(&args);
        assert!(args.is_err());
        assert_eq!(
            args.err().unwrap(),
            "First argument should be path to binary"
        );
    }

    #[test]
    fn one_arg() {
        let path_to_binary = String::from("binary");
        let args: Vec<String> = vec![path_to_binary];
        let args = parse_args(&args);
        assert!(args.is_err());
        assert_eq!(args.err().unwrap(), "Backup location is not specified");
    }

    #[test]
    fn two_args() {
        let path_to_binary = String::from("binary");
        let first_argument = String::from("one");
        let args: Vec<String> = vec![path_to_binary, first_argument];
        let args = parse_args(&args);
        assert!(args.is_err());
        assert_eq!(args.err().unwrap(), "Target location is not specified");
    }

    #[test]
    fn three_args() {
        let path_to_binary = String::from("binary");
        let first_argument = String::from("one");
        let second_argument = String::from("two");
        let args: Vec<String> = vec![path_to_binary, first_argument, second_argument];
        let args = parse_args(&args).unwrap();
        assert_eq!(args.backup.to_string_lossy(), "one");
        assert_eq!(args.files.to_string_lossy(), "two");
    }

    #[test]
    fn more_than_3_args_are_ignored() {
        let args = vec![
            "1".to_string(),
            "2".to_string(),
            "3".to_string(),
            "4".to_string(),
        ];
        let args = parse_args(&args).unwrap();
        assert_eq!(args.backup.to_string_lossy(), "2");
        assert_eq!(args.files.to_string_lossy(), "3");
    }
}
