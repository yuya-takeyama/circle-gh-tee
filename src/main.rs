extern crate circle_gh_tee;

use std::process;
use circle_gh_tee::run;

fn main() {
    match run() {
        Ok(s) => {
            print!("{}", s.command_result.result);
            if let Some(error_message) = s.error_message {
                eprintln!("circle-gh-tee: error: {}", error_message);
                process::exit(1);
            } else {
                process::exit(s.command_result.exit_status);
            }
        }
        Err(f) => {
            eprintln!("circle-gh-tee: error: {}", f.message);
            process::exit(1);
        }
    }
}
