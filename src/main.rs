extern crate clap;
extern crate duct;
extern crate regex;
extern crate reqwest;

use clap::{App, Arg, ArgMatches};
use std::str;
use std::io;
use duct::cmd;
use std::process;
use std::env;
use regex::Regex;
use std::collections::HashMap;

struct CommandResult {
    full_command: String,
    result: String,
    exit_status: i32,
}

fn main() {
    let matches = get_matches();
    let (command_name, args) = parse_command_name_and_args(&matches);
    let output = run_command(command_name, args.clone());

    match output {
        Ok(res) => {
            let command_result = build_command_result(command_name, args, &res);
            let comment = build_comment(&command_result);
            match post_comment(comment) {
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

fn parse_command_name_and_args<'a>(matches: &'a ArgMatches) -> (&'a str, Vec<&'a str>) {
    let command_name_and_args = matches.values_of("COMMAND").unwrap().collect::<Vec<_>>();
    let command_name = command_name_and_args[0];
    let args = command_name_and_args[1..].to_vec();
    (command_name, args)
}

fn run_command(command_name: &str, args: Vec<&str>) -> Result<process::Output, io::Error> {
    cmd(command_name, args)
        .unchecked()
        .stderr_to_stdout()
        .stdout_capture()
        .run()
}

fn build_command_result(
    command_name: &str,
    args: Vec<&str>,
    output: &std::process::Output,
) -> CommandResult {
    let full_command = args.into_iter().fold(String::from(command_name), |acc, c| {
        format!("{} {}", acc, c)
    });
    let exit_status = output.status.code().unwrap();
    let result = String::from_utf8_lossy(&output.stdout);
    CommandResult {
        full_command,
        result: String::from(result),
        exit_status,
    }
}

fn build_comment(command_result: &CommandResult) -> String {
    if command_result.exit_status == 0 {
        expand_template_variables(DEFAULT_EXIT_ZERO, &command_result)
    } else {
        expand_template_variables(DEFAULT_EXIT_NON_ZERO, &command_result)
    }
}

fn expand_template_variables(template: &str, command_result: &CommandResult) -> String {
    String::from(template)
        .replace("{{full_command}}", &command_result.full_command)
        .replace("{{result}}", &command_result.result)
        .replace(
            "{{exit_status}}",
            &String::from(format!("{}", command_result.exit_status)),
        )
}

fn post_comment(comment: String) -> reqwest::Result<reqwest::Response> {
    let http_client = reqwest::Client::new();
    let url = format!(
        "https://api.github.com/repos/{owner}/{repo}/issues/{number}/comments",
        owner = env::var("CIRCLE_PROJECT_USERNAME").unwrap(),
        repo = env::var("CIRCLE_PROJECT_REPONAME").unwrap(),
        number = get_pull_request_number().unwrap()
    );
    let mut body = HashMap::new();
    body.insert(String::from("body"), comment);
    let mut headers = reqwest::header::Headers::new();
    headers.set_raw(
        String::from("Authorization"),
        format!("token {}", env::var("GITHUB_ACCESS_TOKEN").unwrap()),
    );
    http_client.post(&url).json(&body).headers(headers).send()
}

fn get_pull_request_number() -> Result<String, String> {
    match env::var("CI_PULL_REQUEST") {
        Ok(url) => {
            let re = Regex::new(r"/pull/(\d+)$").unwrap();
            Ok(String::from(
                re.captures(&url).unwrap().get(1).unwrap().as_str(),
            ))
        }
        Err(_) => Err(String::from("Failed to get CI_PULL_REQUEST")),
    }
}

const DEFAULT_EXIT_ZERO: &str = ":white_check_mark: `{{full_command}}` exited with `0`.
```
{{result}}
```";
const DEFAULT_EXIT_NON_ZERO: &str =
    ":no_entry_sign: `{{full_command}}` exited with `{{exit_status}}`.
```
{{result}}
```";

fn get_matches<'a>() -> ArgMatches<'a> {
    App::new("circle-gh-tee")
        .version("0.1.0")
        .author("Yuya Takeyama <sign.of.the.wolf.pentagram@gmail.com>")
        .about("Command to execute command and post its result to GitHub")
        .arg(
            Arg::with_name("exit-zero-template")
                .long("exit-zero-template")
                .value_name("TEMPLATE")
                .help("Comment template used when exit code is zero")
                .default_value(DEFAULT_EXIT_ZERO)
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("exit-non-zero-template")
                .long("exit-non-zero-template")
                .value_name("TEMPLATE")
                .help("Comment template used when exit code is non-zero")
                .default_value(DEFAULT_EXIT_NON_ZERO)
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("COMMAND")
                .help("Sets the command to run")
                .required(true)
                .multiple(true),
        )
        .get_matches()
}
