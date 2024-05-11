use std::{fmt, time::SystemTime};

use colored::Colorize;

use chrono::{DateTime, Local};

use crate::client::requests::ClientRequestKind;

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    BadSyntax(String),
    CommandNotExists(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let dt: DateTime<Local> = SystemTime::now().into();

        match self {
            Self::BadSyntax(s) => write!(
                f,
                "[{}]\t{}: {}",
                dt.format("%Y-%m-%d %H:%M:%S").to_string().italic(),
                "ERROR".red().bold(),
                s.italic()
            ),
            Self::CommandNotExists(s) => write!(
                f,
                "[{}]\t{}: {} '{}' {}",
                dt.format("%Y-%m-%d %H:%M:%S").to_string().italic(),
                "ERROR".red().bold(),
                "command".italic(),
                s.italic().underline().bold(),
                "not exists".italic()
            ),
        }
    }
}

fn bad_syntax<T>(info: &str) -> Result<T, ParseError> {
    Err(ParseError::BadSyntax(info.to_string()))
}

fn command_not_exists<T>(info: &str) -> Result<T, ParseError> {
    Err(ParseError::CommandNotExists(info.to_string()))
}

fn validate_param(param: &[&str]) -> Result<String, ParseError> {
    if param.len() == 1 {
        if param[0].starts_with('\'') {
            if param[0].len() <= 2 || !param[0].ends_with('\'') {
                bad_syntax("bad parameter")
            } else {
                Ok(param[0][1..param[0].len() - 1].to_string())
            }
        } else {
            Ok(param[0].to_owned())
        }
    } else if !param[0].starts_with('\'') || !param[param.len() - 1].ends_with('\'') {
        bad_syntax("composite parameter must be in the quotation marks")
    } else {
        let params = param.len();

        let first = param[0][1..].to_owned();
        let last = param[params - 1][..param[params - 1].len() - 1].to_owned();
        let mid = param[1..params - 1].join(" ");

        if mid.is_empty() {
            Ok([first, last].join(" "))
        } else {
            Ok([first, mid, last].join(" "))
        }
    }
}

/// Parse request from user.
pub fn parse_request(req: &str) -> Result<ClientRequestKind, ParseError> {
    if !req.starts_with('/') {
        bad_syntax("request must start with /")
    } else {
        let keys = req
            .split(|c| c == ' ' || c == '\n')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();

        if keys.is_empty() {
            Err(ParseError::BadSyntax(
                "request must not be empty".to_string(),
            ))
        } else {
            let cmd = &keys[0][1..];

            match cmd.to_lowercase().as_str() {
                "send" => {
                    if keys.len() == 1 {
                        bad_syntax("send: expected 'message'")
                    } else {
                        validate_param(&keys[1..]).map(ClientRequestKind::SendMessage)
                    }
                }
                "create" => {
                    if keys.len() == 1 {
                        bad_syntax("create: expected 'chat name'")
                    } else {
                        validate_param(&keys[1..]).map(ClientRequestKind::Create)
                    }
                }
                "connect" => {
                    if keys.len() == 1 {
                        bad_syntax("connect: expected 'chat name'")
                    } else {
                        validate_param(&keys[1..]).map(ClientRequestKind::Connect)
                    }
                }
                "disconnect" => {
                    if keys.len() != 1 {
                        bad_syntax("disconnect: argument not expected")
                    } else {
                        Ok(ClientRequestKind::Disconnect)
                    }
                }
                "status" => {
                    if keys.len() != 1 {
                        bad_syntax("status: argument not expected")
                    } else {
                        Ok(ClientRequestKind::Status)
                    }
                }
                _ => command_not_exists(cmd),
            }
        }
    }
}
