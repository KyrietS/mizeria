use snapshot::Snapshot;
use std::ffi::OsStr;
use std::path::PathBuf;

pub mod helpers;
mod snapshot;

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
    let snapshot = Snapshot::create(&args.backup)?;
    snapshot.index_files(&args.files)?;
    snapshot.copy_files(&args.files)?;
    println!("Created snapshot: {}", snapshot.name());

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
