//! [@actions/core](https://www.npmjs.com/package/@actions/core) for Rust projects.

mod command;
mod file_command;
mod oidc_utils;
mod path_utils;
pub mod platform;
mod summary;
mod utils;

pub use crate::path_utils::{to_platform_path, to_posix_path, to_win32_path};
pub use crate::summary::{MARKDOWN_SUMMARY, SUMMARY};
use crate::utils::to_command_value;
use command::{issue, issue_command, CommandProperties};
use file_command::{issue_file_command, prepare_key_value_message};
use oidc_utils::OidcClient;
use std::any::Any;
use std::collections::HashMap;
use std::error::Error;
use std::io::Write;
use std::{env, fs, process};
use utils::to_command_properties;

pub struct InputOptions {
    pub required: Option<bool>,
    pub trim_whitespace: Option<bool>,
}

pub enum ExitCode {
    Success = 0,
    Failure = 1,
}

pub struct AnnotationProperties<'a> {
    pub title: Option<&'a str>,
    pub file: Option<&'a str>,
    pub start_line: Option<u32>,
    pub end_line: Option<u32>,
    pub start_column: Option<u32>,
    pub end_column: Option<u32>,
}

pub fn export_variable(name: &str, value: Option<String>) -> Result<(), Box<dyn Error>> {
    let converted_value = to_command_value(value);
    env::set_var(name, converted_value);
    let file_path = env::var("GITHUB_ENV").unwrap_or_default();
    if !file_path.is_empty() {
        issue_file_command("ENV", Some(prepare_key_value_message(name, value)?))?;
    } else {
        issue_command(
            "set-env",
            CommandProperties::from([("name".into(), name.into())]),
            Some(converted_value),
        )?;
    }
    Ok(())
}

pub fn set_secret(secret: &str) -> Result<(), Box<dyn Error>> {
    issue_command("add-mask", CommandProperties::new(), Some(secret))?;
    Ok(())
}

pub fn add_path(input_path: &str) -> Result<(), Box<dyn Error>> {
    let file_path = env::var("GITHUB_PATH").unwrap_or_default();
    if !file_path.is_empty() {
        issue_file_command("PATH", Some(input_path))?;
    } else {
        issue_command("add-path", CommandProperties::new(), Some(input_path))?;
    }
    #[cfg(target_os = "windows")]
    let path_delimiter = ";";
    #[cfg(not(target_os = "windows"))]
    let path_delimiter = ":";
    env::set_var(
        "PATH",
        format!("{}{path_delimiter}{input_path}", env::var("PATH")?),
    );
    Ok(())
}

pub fn get_input(name: &str, options: Option<&InputOptions>) -> Result<String, Box<dyn Error>> {
    let value =
        env::var(format!("INPUT_{}", name.replace(' ', "_").to_uppercase())).unwrap_or_default();
    if let Some(options) = options {
        if options.required.unwrap_or_default() && value.is_empty() {
            return Err(format!("input {name} required").into());
        }
        if options.trim_whitespace.is_some_and(|x| x == false) {
            return Ok(value);
        }
    }
    Ok(value.trim().into())
}

pub fn get_multiline_input(
    name: &str,
    options: Option<&InputOptions>,
) -> Result<Vec<String>, Box<dyn Error>> {
    let inputs = get_input(name, options)?
        .split("\n")
        .map(|x| x.into())
        .collect::<Vec<String>>();
    if options.is_some_and(|options| options.trim_whitespace.is_some_and(|x| x == false)) {
        return Ok(inputs);
    }
    Ok(inputs.iter().map(|x| x.trim().into()).collect())
}

pub fn get_boolean_input(
    name: &str,
    options: Option<&InputOptions>,
) -> Result<bool, Box<dyn Error>> {
    let true_value = vec!["true", "True", "TRUE"];
    let false_value = vec!["false", "False", "FALSE"];
    let value = get_input(name, options)?;
    if true_value.contains(&value.as_str()) {
        return Ok(true);
    }
    if false_value.contains(&value.as_str()) {
        return Ok(false);
    }
    Err(format!("{name} not `true | True | TRUE | false | False | FALSE`").into())
}

pub fn set_output(name: &str, value: Option<String>) -> Result<(), Box<dyn Error>> {
    let file_path = env::var("GITHUB_OUTPUT").unwrap_or_default();
    if !file_path.is_empty() {
        issue_file_command("OUTPUT", Some(prepare_key_value_message(name, value)?))?;
    } else {
        println!();
        issue_command(
            "set-output",
            CommandProperties::from([("name".into(), name.into())]),
            Some(to_command_value(value)),
        )?;
    }
    Ok(())
}

pub fn set_command_echo(enabled: bool) -> Result<(), Box<dyn Error>> {
    issue(
        "echo",
        Some(if enabled { "on".into() } else { "off".into() }),
    )?;
    Ok(())
}

pub fn set_failed(message: Box<dyn Error>) -> Result<(), Box<dyn Error>> {
    error(message, None);
    Ok(())
}

pub fn is_debug() -> bool {
    env::var("RUNNER_DEBUG").is_ok_and(|x| x == "1")
}

pub fn debug(message: &str) -> Result<(), Box<dyn Error>> {
    issue_command("debug", CommandProperties::new(), Some(message.into()))?;
    Ok(())
}

pub fn error(
    message: Box<dyn Error>,
    properties: Option<AnnotationProperties>,
) -> Result<(), Box<dyn Error>> {
    let properties = properties.unwrap_or(AnnotationProperties {
        title: None,
        file: None,
        start_line: None,
        end_line: None,
        start_column: None,
        end_column: None,
    });
    issue_command(
        "error",
        to_command_properties(properties),
        Some(format!("{message}")),
    );
    Ok(())
}

pub fn warning(
    message: Box<dyn Error>,
    properties: Option<AnnotationProperties>,
) -> Result<(), Box<dyn Error>> {
    let properties = properties.unwrap_or(AnnotationProperties {
        title: None,
        file: None,
        start_line: None,
        end_line: None,
        start_column: None,
        end_column: None,
    });
    issue_command(
        "warning",
        to_command_properties(properties),
        Some(format!("{message}")),
    );
    Ok(())
}

pub fn notice(
    message: Box<dyn Error>,
    properties: Option<AnnotationProperties>,
) -> Result<(), Box<dyn Error>> {
    let properties = properties.unwrap_or(AnnotationProperties {
        title: None,
        file: None,
        start_line: None,
        end_line: None,
        start_column: None,
        end_column: None,
    });
    issue_command(
        "notice",
        to_command_properties(properties),
        Some(format!("{message}")),
    );
    Ok(())
}

pub fn info(message: &str) {
    println!("{message}");
}

pub fn start_group(name: &str) -> Result<(), Box<dyn Error>> {
    issue("group", Some(name.into()))?;
    Ok(())
}

pub fn end_group() -> Result<(), Box<dyn Error>> {
    issue("endgroup", None)?;
    Ok(())
}

pub fn group<T, F: FnOnce() -> T>(name: &str, f: F) -> Result<T, Box<dyn Error>> {
    start_group(name)?;
    let result = f();
    end_group()?;
    Ok(result)
}

pub fn save_state(name: &str, value: Option<String>) -> Result<(), Box<dyn Error>> {
    let file_path = env::var("GITHUB_STATE").unwrap_or_default();
    if !file_path.is_empty() {
        issue_file_command("STATE", Some(prepare_key_value_message(name, value)?))?;
    } else {
        issue_command(
            "save-state",
            CommandProperties::from([("name".into(), name.into())]),
            Some(to_command_value(value)),
        )?;
    }
    Ok(())
}

pub fn get_state(name: &str) -> Result<String, Box<dyn Error>> {
    Ok(env::var(format!("STATE_{name}")).unwrap_or_default())
}

pub fn get_id_token(audience: Option<String>) -> Result<String, Box<dyn Error>> {
    Ok(OidcClient::get_id_token(audience)?)
}
