use log::error;
use mizeria::run_program;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    let result_code = match run_program(&args[1..], &mut std::io::stdout()) {
        Ok(_) => 0,
        Err(msg) => {
            if let Some(source) = msg.source() {
                error!("{}", source);
            }
            error!("{}", msg);
            1
        }
    };

    std::process::exit(result_code);
}
