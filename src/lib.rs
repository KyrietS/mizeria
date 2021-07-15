use clap::{App, AppSettings, Arg};
use env_logger::{Builder, WriteStyle};
use log::LevelFilter;
use snapshot::Snapshot;
use std::ffi::OsStr;
use std::path::PathBuf;

pub mod helpers;
mod snapshot;

struct Args {
    backup: PathBuf,
    files: PathBuf,
    log_level: LevelFilter,
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
    init_logger(args.log_level);

    let mut snapshot = Snapshot::create(&args.backup)?;
    snapshot.index_files(&args.files)?;
    snapshot.copy_files(&args.files)?;
    println!("Created snapshot: {}", snapshot.name());

    Ok(())
}

fn parse_args(args: &[String]) -> Result<Args, &'static str> {
    let matches = App::new("mizeria")
        .version(clap::crate_version!())
        .about("Simple backup software")
        .setting(AppSettings::NoBinaryName)
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
        .get_matches_from(args);

    let backup = matches.value_of("BACKUP").unwrap();
    let files = matches.value_of("INPUT").unwrap(); // TODO: change to values_of
    let log_level = match matches.occurrences_of("v") {
        0 => LevelFilter::Warn,
        1 => LevelFilter::Debug,
        2 => LevelFilter::Trace,
        _ => LevelFilter::Trace,
    };

    Ok(Args {
        backup: PathBuf::from(backup),
        files: PathBuf::from(files),
        log_level,
    })
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
