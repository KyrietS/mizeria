use std::path::PathBuf;

struct Args {
    backup_location: PathBuf,
    files_location: PathBuf
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match parse_args(&args) {
        Ok(args) => {
            println!("Folder where backup will be stored:");
            println!("  {}", args.backup_location.to_string_lossy());
            println!("Folder that will be backed up:");
            println!("  {}", args.files_location.to_string_lossy());
        }
        Err(msg) => eprintln!("{}", msg)
    }
}

fn parse_args(args: &Vec<String>) -> Result<Args, &'static str> {
    let mut args_iter = args.iter();
    args_iter.next().ok_or("First argument should be path to binary")?;
    let backup = args_iter.next().ok_or("Backup location is not specified")?;
    let files = args_iter.next().ok_or("Target location is not specified")?;

    Ok(Args{
        backup_location: PathBuf::from(backup),
        files_location: PathBuf::from(files)
    })
}


#[cfg(test)]
mod args_parser_tests {
    use super::*;

    #[test]
    fn empty_args() {
        let args: Vec<String> = vec![];
        let args = parse_args(&args);
        assert!(args.is_err());
        assert_eq!(args.err().unwrap(), "First argument should be path to binary");
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
        assert_eq!(args.backup_location.to_string_lossy(), "one");
        assert_eq!(args.files_location.to_string_lossy(), "two");
    }

    #[test]
    fn more_than_3_args_are_ignored() {
        let args = vec!["1".to_string(), "2".to_string(), "3".to_string(), "4".to_string()];
        let args = parse_args(&args).unwrap();
        assert_eq!(args.backup_location.to_string_lossy(), "2");
        assert_eq!(args.files_location.to_string_lossy(), "3");
    }
}
