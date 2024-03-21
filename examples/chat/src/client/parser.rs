use crate::client::requests::ClientRequestKind;

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    BadSyntax(String),
    CommandNotExists(String),
}

fn bad_syntax<T>(info: &str) -> Result<T, ParseError> {
    Err(ParseError::BadSyntax(info.to_string()))
}

fn command_not_exists<T>(info: &str) -> Result<T, ParseError> {
    Err(ParseError::CommandNotExists(info.to_string()))
}

fn validate_param(param: &str) -> Result<String, ParseError> {
    if param.starts_with("'") {
        if param.ends_with("'") {
            Ok(param[1..param.len() - 1].to_string())
        } else {
            bad_syntax("semicolon expected")
        }
    } else {
        assert!(!param.is_empty());
        Ok(param.to_string())
    }
}

/// Parse request from user.
pub fn parse_request(req: &str) -> Result<ClientRequestKind, ParseError> {
    if !req.starts_with("/") {
        bad_syntax("request must start with /")
    } else {
        let keys = req.split(" ").collect::<Vec<&str>>();

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
                        validate_param(keys[1]).map(|msg| ClientRequestKind::SendMessage(msg))
                    }
                }
                "create" => {
                    if keys.len() == 1 {
                        bad_syntax("create: expected 'chat name'")
                    } else {
                        validate_param(keys[1]).map(|chat| ClientRequestKind::Create(chat))
                    }
                }
                "connect" => {
                    if keys.len() == 1 {
                        bad_syntax("connect: expected 'chat name'")
                    } else {
                        validate_param(keys[1]).map(|chat| ClientRequestKind::Connect(chat))
                    }
                }
                "disconnect" => Ok(ClientRequestKind::Disconnect),
                _ => command_not_exists(cmd),
            }
        }
    }
}
