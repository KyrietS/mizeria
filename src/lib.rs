use backup::Backup;
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use env_logger::{Builder, WriteStyle};
use log::LevelFilter;
use result::{IntegrityCheckError, IntegrityCheckResult};
use std::ffi::OsStr;
use std::fmt::Display;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::slice::Iter;

mod backup;
pub mod result;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
type Writer<'a> = &'a mut dyn Write;

pub fn run_program<C: IntoIterator>(args: C, writer: Writer) -> Result<()>
where
    C::Item: AsRef<OsStr>,
{
    let args: Vec<String> = args
        .into_iter()
        .map(|e: _| e.as_ref().to_string_lossy().to_string())
        .collect();

    let matches = parse_args(&args);
    execute_subcommand(matches, writer)
}

fn execute_subcommand(matches: ArgMatches, writer: Writer) -> Result<()> {
    return match matches.subcommand() {
        ("backup", Some(args)) => handle_backup(args, writer),
        ("list", Some(args)) => handle_list_snapshots(args, writer),
        ("snapshot", Some(args)) => handle_manage_snapshot(args, writer),
        _ => Ok(()),
    };
}

fn get_verbosity_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name("v")
        .short("v")
        .multiple(true)
        .help("Sets the level of verbosity")
        .long_help(concat!(
            "Use -v to turn on debug logs showing steps in producing a backup.\n",
            "Use -vv to see debug and trace logs that show every file being indexed and copied.\n",
            "By default only warning and error logs are printed."
        ))
}

fn parse_args(args: &[String]) -> ArgMatches {
    App::new("mizeria")
        .version(clap::crate_version!())
        .about("Simple backup software")
        .setting(AppSettings::VersionlessSubcommands)
        .setting(AppSettings::NoBinaryName)
        .subcommand(SubCommand::with_name("backup")
            .about("Make a backup of your files")
            .arg(
                Arg::with_name("BACKUP")
                    .help("A folder where snapshot will be stored")
                    .required(true)
                    .index(1),
            )
            .arg(
                Arg::with_name("INPUT")
                    .help("Files or folders to be backed up")
                    .required(true)
                    .multiple(true)
                    .index(2),
            )
            .arg(
                Arg::with_name("full")
                    .long("full")
                    .help("Force creating full snapshot")
                    .long_help(concat!(
                        "Every snapshot is incremental by default and is based on the latest\n",
                        "snapshot that can be found in the backup root. Using this option you\n",
                        "can force the program to create full snapshot of your files. It means\n",
                        "that all files will be copied into the snapshot even if they are already\n",
                        "present in other snapshots."
                    ))
            )
            .arg(get_verbosity_arg())
        )
        .subcommand(SubCommand::with_name("list")
            .about("List all snapshots")
            .visible_alias("ls")
            .arg(get_verbosity_arg())
            .arg(
                Arg::with_name("BACKUP")
                    .help("A folder where snapshots are stored. Defaults to current directory")
                    .required(false)
                    .index(1),
            )
            .arg(
                Arg::with_name("short")
                    .long("short")
                    .short("s")
                    .help("Print only basic information about snapshots in a short format")
            )
        )
        .subcommand(SubCommand::with_name("snapshot")
            .about("View or edit snapshots")
            .arg(
                Arg::with_name("SNAPSHOT")
                    .help("A snapshot to be selected")
                    .required(true)
                    .index(1)
            )
            .arg(get_verbosity_arg())
        )
        .get_matches_from(args)
}

fn print_snapshots(writer: Writer, snapshots: Iter<'_, impl Display>) -> Result<()> {
    writeln!(writer, "Available snapshots:")?;
    for (index, snapshot) in snapshots.rev().enumerate() {
        writeln!(writer, "{}. {}", index + 1, snapshot)?;
    }
    Ok(())
}

fn list_all_snapshots(writer: Writer, path: &Path, short_format: bool) -> Result<()> {
    if !path.exists() {
        return Err("Folder with backup doesn't exist or isn't accessible".into());
    }

    if short_format {
        let previews = Backup::get_all_snapshot_previews(path);
        print_snapshots(writer, previews.iter())?;
    } else {
        let snapshots = Backup::get_all_snapshots(path);
        print_snapshots(writer, snapshots.iter())?;
    };

    Ok(())
}

fn handle_list_snapshots(args: &ArgMatches, writer: Writer) -> Result<()> {
    set_verbosity(args);
    let short_format = args.is_present("short");
    let path = args.value_of("BACKUP").unwrap_or(".");
    let path = Path::new(path);
    list_all_snapshots(writer, path, short_format)
}

fn handle_manage_snapshot(args: &ArgMatches, writer: Writer) -> Result<()> {
    set_verbosity(args);
    let snapshot = args.value_of("SNAPSHOT").unwrap();
    let snapshot = PathBuf::from(snapshot);

    let result = perform_integrity_check(snapshot);
    let result_message = match result {
        Ok(()) => format!("Snapshot integrity check completed. No problems found."),
        Err(error) => format!("Snapshot integrity check failed. {}", error),
    };

    writeln!(writer, "{}", result_message)?;

    Ok(())
}

fn perform_integrity_check(snapshot_path: PathBuf) -> IntegrityCheckResult {
    if !snapshot_path.exists() {
        return Err(IntegrityCheckError::SnapshotDoesntExist)?;
    }
    let snapshot_name = snapshot_path
        .file_name()
        .ok_or(IntegrityCheckError::SnapshotDoesntExist)?;
    let backup_path = snapshot_path
        .parent()
        .ok_or(IntegrityCheckError::UnexpectedError(
            "Cannot open backup folder".into(),
        ))?;
    let backup = match Backup::open(backup_path) {
        Ok(backup) => backup,
        Err(error) => Err(IntegrityCheckError::UnexpectedError(format!("{}", error)))?,
    };
    backup.check_integrity(snapshot_name)
}

fn handle_backup(args: &ArgMatches, writer: Writer) -> Result<()> {
    let backup = args.value_of("BACKUP").unwrap();
    let files: Vec<PathBuf> = args
        .values_of("INPUT")
        .unwrap()
        .map(PathBuf::from)
        .collect();

    set_verbosity(args);

    let incremental_snapshot = !args.is_present("full");
    let mut backup = Backup::open(Path::new(backup))?;

    let timestamp = backup.add_snapshot(files.as_slice(), incremental_snapshot)?;
    writeln!(writer, "Created snapshot: {}", timestamp)?;

    Ok(())
}

fn set_verbosity(args: &ArgMatches) {
    let log_level = match args.occurrences_of("v") {
        0 => LevelFilter::Warn,
        1 => LevelFilter::Debug,
        2 => LevelFilter::Trace,
        _ => LevelFilter::Trace,
    };

    init_logger(log_level);
}

fn init_logger(log_level: LevelFilter) {
    let mut builder = Builder::new();
    builder
        .filter(Some("mizeria"), log_level)
        .write_style(WriteStyle::Auto)
        .format_module_path(false)
        .format_timestamp(None)
        .target(env_logger::Target::Stderr)
        .try_init()
        .ok();
}
