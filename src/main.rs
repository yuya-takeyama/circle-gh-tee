extern crate circle_gh_tee;

use std::process;
use circle_gh_tee::*;

fn main() {
    let matches = get_matches();
    let environment = match Environment::load() {
        Ok(env) => env,
        Err(e) => {
            println!("Failed to load environment: {}", e);
            process::exit(1);
        }
    };
    let (command_name, args) = parse_command_name_and_args(&matches);
    let output = run_command(command_name, args.clone());

    match output {
        Ok(res) => {
            let command_result = CommandResult::new(command_name, args, &res);
            let comment = build_comment(&matches, &command_result);
            match post_comment(comment, environment) {
                Ok(_) => {
                    println!("{}", command_result.result);
                    process::exit(command_result.exit_status);
                }
                Err(err) => {
                    println!("{}", command_result.result);
                    println!("Failed to post comment to GitHub: {}", err);
                    process::exit(1);
                }
            }
        }
        Err(e) => {
            println!("Failed: {}", e);
            process::exit(1);
        }
    }
}
