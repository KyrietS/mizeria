use env_logger::{Builder, WriteStyle};
use log::{error, LevelFilter};
use mizeria::run_program;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    init_logger();

    let result_code = match run_program(&args[1..]) {
        Ok(_) => 0,
        Err(msg) => {
            error!("{}", msg);
            1
        }
    };

    std::process::exit(result_code);
}

fn init_logger() {
    let mut builder = Builder::new();
    builder
        .filter(Some("mizeria"), LevelFilter::Warn)
        .write_style(WriteStyle::Auto)
        .format_module_path(false)
        .format_timestamp(None)
        .init();
}
