extern crate clap;
extern crate duct;
extern crate regex;
extern crate reqwest;

use clap::{App, Arg, ArgMatches};
use regex::Regex;
use std::env;
use std::process;
use std::str;
use std::io;
use duct::cmd;
use std::collections::HashMap;

const DEFAULT_EXIT_ZERO: &str = ":white_check_mark: `{{full_command}}` exited with `0`.
```
{{result}}
```";
const DEFAULT_EXIT_NON_ZERO: &str =
    ":no_entry_sign: `{{full_command}}` exited with `{{exit_status}}`.
```
{{result}}
```";

pub fn get_matches<'a>() -> ArgMatches<'a> {
    App::new("circle-gh-tee")
        .version("0.1.0")
        .author("Yuya Takeyama <sign.of.the.wolf.pentagram@gmail.com>")
        .about("Command to execute command and post its result to GitHub")
        .usage("circle-gh-tee [OPTIONS] -- <COMMAND>...")
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

pub fn parse_command_name_and_args<'a>(matches: &'a ArgMatches) -> (&'a str, Vec<&'a str>) {
    let command_name_and_args = matches.values_of("COMMAND").unwrap().collect::<Vec<_>>();
    let command_name = command_name_and_args[0];
    let args = command_name_and_args[1..].to_vec();
    (command_name, args)
}

pub fn run_command(command_name: &str, args: Vec<&str>) -> Result<process::Output, io::Error> {
    cmd(command_name, args)
        .unchecked()
        .stderr_to_stdout()
        .stdout_capture()
        .run()
}

pub fn build_comment(matches: &ArgMatches, command_result: &CommandResult) -> String {
    if command_result.exit_status == 0 {
        expand_template_variables(
            matches.value_of("exit-zero-template").unwrap(),
            &command_result,
        )
    } else {
        expand_template_variables(
            matches.value_of("exit-non-zero-template").unwrap(),
            &command_result,
        )
    }
}

pub fn expand_template_variables(template: &str, command_result: &CommandResult) -> String {
    String::from(template)
        .replace("{{full_command}}", &command_result.full_command)
        .replace("{{result}}", &command_result.result)
        .replace(
            "{{exit_status}}",
            &String::from(format!("{}", command_result.exit_status)),
        )
}

pub fn post_comment(
    comment: String,
    environment: Environment,
) -> Result<reqwest::Response, String> {
    let http_client = reqwest::Client::new();
    let pull_request_url = match environment.get_pull_request_comment_api_url() {
        Ok(url) => url,
        Err(e) => return Err(e),
    };
    let mut body = HashMap::new();
    body.insert(String::from("body"), comment);
    let mut headers = reqwest::header::Headers::new();
    headers.set_raw(
        String::from("Authorization"),
        format!("token {}", environment.github_access_token),
    );
    http_client
        .post(&pull_request_url)
        .json(&body)
        .headers(headers)
        .send()
        .or_else(|e| Err(format!("Failed to post a comment to GitHub: {}", e)))
}

pub struct Environment {
    pub github_access_token: String,
    pub username: String,
    pub reponame: String,
    pull_request_url: String,
    last_commit_comment: String,
}

impl Environment {
    pub fn load() -> Result<Environment, String> {
        let github_access_token = match env::var("GITHUB_ACCESS_TOKEN") {
            Ok(v) => v,
            Err(e) => return Err(format!("Failed to get GitHub access token: {}", e)),
        };
        let username = match env::var("CIRCLE_PROJECT_USERNAME") {
            Ok(v) => v,
            Err(e) => return Err(format!("Failed to get username: {}", e)),
        };
        let reponame = match env::var("CIRCLE_PROJECT_REPONAME") {
            Ok(v) => v,
            Err(e) => return Err(format!("Failed to get reponame: {}", e)),
        };
        let pull_request_url = env::var("CI_PULL_REQUEST").unwrap_or(String::from(""));
        let last_commit_comment = match process::Command::new("git")
            .arg("--no-pager")
            .arg("log")
            .arg("--pretty=format:\"%s\"")
            .arg("-1")
            .output()
        {
            Ok(v) => String::from(String::from_utf8_lossy(&v.stdout)),
            Err(e) => return Err(format!("Failed to get the last commit log: {}", e)),
        };

        if pull_request_url.len() == 0 && last_commit_comment.len() == 0 {
            Err(String::from("Failed to get the Pull Request number"))
        } else {
            Ok(Environment {
                github_access_token,
                username,
                reponame,
                pull_request_url,
                last_commit_comment,
            })
        }
    }

    pub fn get_pull_request_comment_api_url(&self) -> Result<String, String> {
        let pull_request_number = match self.get_pull_request_number() {
            Ok(number) => number,
            Err(e) => return Err(e),
        };
        Ok(format!(
            "https://api.github.com/repos/{owner}/{repo}/issues/{number}/comments",
            owner = self.username,
            repo = self.reponame,
            number = pull_request_number,
        ))
    }

    pub fn get_pull_request_number(&self) -> Result<String, String> {
        if self.pull_request_url.len() > 0 {
            self.get_pull_request_number_from_ci_pull_request()
        } else {
            self.get_pull_request_number_from_last_commit_comment()
        }
    }

    fn get_pull_request_number_from_ci_pull_request(&self) -> Result<String, String> {
        let re = match Regex::new(r"/pull/(\d+)$") {
            Ok(r) => r,
            Err(e) => return Err(format!("Failed to create Regex object: {}", e)),
        };
        match re.captures(&self.pull_request_url) {
            Some(matches) => match matches.get(1) {
                Some(matched) => Ok(String::from(matched.as_str())),
                None => Err(format!(
                    "Failed to get Pull Request number from CI_PULL_REQUEST: {}",
                    &self.pull_request_url
                )),
            },
            None => Err(format!(
                "Failed to get Pull Request number from CI_PULL_REQUEST: {}",
                &self.pull_request_url
            )),
        }
    }

    fn get_pull_request_number_from_last_commit_comment(&self) -> Result<String, String> {
        let re = Regex::new(r"^Merge pull request #([0-9]+)").unwrap();
        match re.captures(&self.last_commit_comment) {
            Some(matches) => match matches.get(1) {
                Some(matched) => Ok(String::from(matched.as_str())),
                None => Err(format!(
                    "Failed to get Pull Request number from last commit comment: {}",
                    &self.last_commit_comment,
                )),
            },
            None => Err(format!(
                "Failed to get Pull Request number from last commit comment: {}",
                &self.last_commit_comment,
            )),
        }
    }
}

pub struct CommandResult {
    pub full_command: String,
    pub result: String,
    pub exit_status: i32,
}

impl CommandResult {
    pub fn new(
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
}

#[cfg(test)]
mod tests {
    use super::Environment;

    #[test]
    fn environment_get_pull_request_comment_api_url_from_pull_request_url_env() {
        let environment = Environment {
            github_access_token: String::from("token"),
            username: String::from("user"),
            reponame: String::from("repo"),
            pull_request_url: String::from("https://github.com/user/repo/pull/1234"),
            last_commit_comment: String::new(),
        };
        assert_eq!(
            environment.get_pull_request_comment_api_url().unwrap(),
            "https://api.github.com/repos/user/repo/issues/1234/comments"
        );
    }

    #[test]
    fn environment_get_pull_request_comment_api_url_from_last_commit_comment_env() {
        let environment = Environment {
            github_access_token: String::from("token"),
            username: String::from("user"),
            reponame: String::from("repo"),
            pull_request_url: String::new(),
            last_commit_comment: String::from("Merge pull request #4321 from test/branch"),
        };
        assert_eq!(
            environment.get_pull_request_comment_api_url().unwrap(),
            "https://api.github.com/repos/user/repo/issues/4321/comments"
        );
    }

    #[test]
    fn environment_get_pull_request_comment_api_url_err() {
        let environment = Environment {
            github_access_token: String::from("token"),
            username: String::from("user"),
            reponame: String::from("repo"),
            pull_request_url: String::new(),
            last_commit_comment: String::new(),
        };
        assert_eq!(
            environment.get_pull_request_comment_api_url(),
            Err(String::from(
                "Failed to get Pull Request number from last commit comment: "
            ))
        );
    }
}
