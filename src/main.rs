use std::process;

mod cli;

fn main() {
    if let Err(error) = cli::run() {
        eprintln!("error: {error}");
        process::exit(1);
    }
}
