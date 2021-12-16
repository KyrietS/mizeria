use backup::Backup;
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use env_logger::{Builder, WriteStyle};
use log::LevelFilter;
use std::ffi::OsStr;
use std::io::Write;
use std::path::{Path, PathBuf};

mod backup;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn run_program<C: IntoIterator>(args: C, writer: &mut impl Write) -> Result<()>
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

fn execute_subcommand(matches: ArgMatches, writer: &mut impl Write) -> Result<()> {
    return match matches.subcommand() {
        ("backup", Some(args)) => handle_backup(args, writer),
        ("snapshot", Some(args)) => handle_manage_snapshot(args, writer),
        _ => Ok(()),
    };
}

fn parse_args(args: &[String]) -> ArgMatches {
    App::new("mizeria")
        .version(clap::crate_version!())
        .about("Simple backup software")
        .setting(AppSettings::VersionlessSubcommands)
        .setting(AppSettings::NoBinaryName)
        .subcommand(SubCommand::with_name("backup")
            .about("created backup of specified files")
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
            .arg(
                Arg::with_name("v")
                    .short("v")
                    .multiple(true)
                    .help("Sets the level of verbosity")
                    .long_help(concat!(
                        "Use -v to turn on debug logs showing steps in producing a backup.\n",
                        "Use -vv to see debug and trace logs that show every file being indexed and copied.\n",
                        "By default only warning and error logs are printed."
                    ))
            )
        )
        .subcommand(SubCommand::with_name("snapshot")
            .about("managing snapshots utilities")
            .arg(
                Arg::with_name("SNAPSHOT")
                    .help("A snapshot to be selected")
                    .required(true)
                    .index(1)
            ))
        .get_matches_from(args)
}

fn handle_manage_snapshot(args: &ArgMatches, _writer: &mut impl Write) -> Result<()> {
    let snapshot = args.value_of("SNAPSHOT").unwrap();
    let snapshot = PathBuf::from(snapshot);
    let snapshot_name = snapshot.file_name().ok_or("cannot open snapshot")?;
    let backup_path = snapshot.parent().ok_or("cannot open backup")?;
    set_verbosity(args);

    let backup = Backup::open(backup_path)?;
    backup.check_integrity(snapshot_name)?;

    Ok(())
}

fn handle_backup(args: &ArgMatches, writer: &mut impl Write) -> Result<()> {
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
