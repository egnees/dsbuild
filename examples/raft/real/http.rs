use std::{collections::HashMap, convert::Infallible, net::SocketAddr};

use http_body_util::Full;
use hyper::{body::Bytes, server::conn::http1, service::service_fn, Method, Request, Response};
use log::info;
use raft::{
    cmd::CommandType,
    local::{LocalResponse, LocalResponseType},
};
use tokio::{net::TcpListener, sync::oneshot};

use crate::register::SharedRequestRegister;

//////////////////////////////////////////////////////////////////////////////////////////

fn get_http_param<'a>(
    http_params: &'a HashMap<String, String>,
    param: &str,
) -> Result<&'a String, String> {
    http_params
        .get(param)
        .ok_or(format!("query param {:?} is absent", param))
}

//////////////////////////////////////////////////////////////////////////////////////////

fn get_key_param(http_params: &HashMap<String, String>) -> Result<&String, String> {
    get_http_param(http_params, "key")
}

fn get_value_param(http_params: &HashMap<String, String>) -> Result<&String, String> {
    get_http_param(http_params, "value")
}

fn get_cmp_param(http_params: &HashMap<String, String>) -> Result<&String, String> {
    get_http_param(http_params, "cmp")
}

//////////////////////////////////////////////////////////////////////////////////////////

/// Allows to make command type from http request
pub fn command_type_from_http(
    http_method: Method,
    http_params: &HashMap<String, String>,
) -> Result<CommandType, String> {
    if http_method == Method::POST {
        let key = get_key_param(http_params)?;
        Ok(CommandType::create(key))
    } else if http_method == Method::PUT {
        match http_params.len() {
            2 => {
                let key = get_key_param(http_params)?;
                let value = get_value_param(http_params)?;
                Ok(CommandType::update(key, value))
            }
            3 => {
                let key = get_key_param(http_params)?;
                let cmp = get_cmp_param(http_params)?;
                let value = get_value_param(http_params)?;
                Ok(CommandType::cas(key, cmp, value))
            }
            _ => Err("unsupported method".into()),
        }
    } else if http_method == Method::DELETE {
        let key = get_key_param(http_params)?;
        Ok(CommandType::delete(key))
    } else {
        Err("unsupported method".into())
    }
}

//////////////////////////////////////////////////////////////////////////////////////////

/// Returns pair of key and optional [`min commit index`][`raft::local::ReadValueRequest::min_commit_id`].
pub fn read_request_from_http(
    http_params: &HashMap<String, String>,
) -> Result<(String, Option<i64>), String> {
    let key = get_key_param(http_params)?;
    let commit_index = get_http_param(http_params, "commit_index");
    if let Ok(commit_index) = commit_index {
        let commit_index = commit_index.parse::<i64>().map_err(|e| e.to_string())?;
        Ok((key.clone(), Some(commit_index)))
    } else {
        Ok((key.clone(), None))
    }
}

//////////////////////////////////////////////////////////////////////////////////////////

async fn register_request(
    request: Request<hyper::body::Incoming>,
    request_register: SharedRequestRegister,
) -> Result<oneshot::Receiver<LocalResponse>, String> {
    let params =
        get_uri_params(&request.uri().to_string()).ok_or("can not parse uri params".to_owned())?;
    if request.method() == Method::GET {
        let (key, min_commit_id) = read_request_from_http(&params)?;
        Ok(request_register
            .register_read_request(key, min_commit_id)
            .await)
    } else {
        let command_type = command_type_from_http(request.method().clone(), &params)?;
        Ok(request_register.register_command(command_type).await)
    }
}

//////////////////////////////////////////////////////////////////////////////////////////

fn make_response(status: u16, mut s: String) -> Response<Full<Bytes>> {
    s.push('\n');
    Response::builder()
        .status(status)
        .body(Full::new(Bytes::from(s)))
        .unwrap()
}

//////////////////////////////////////////////////////////////////////////////////////////

/// Allows to serve one http connection
async fn serve_connection(
    request: Request<hyper::body::Incoming>,
    request_register: SharedRequestRegister,
) -> Result<Response<Full<Bytes>>, Infallible> {
    // register request
    let register_result = register_request(request, request_register.clone()).await;
    let response = match register_result {
        Err(err) => make_response(400, err), // bad request
        Ok(receiver) => {
            let response = receiver
                .await
                .map(|r| r.tp)
                .unwrap_or(LocalResponseType::Unavailable());
            match response {
                LocalResponseType::Unavailable() => {
                    make_response(503, "service unavailable".to_owned())
                }
                LocalResponseType::ReadValue(result) => make_response(202, format!("{:?}", result)),
                LocalResponseType::RedirectedTo(to, commit_index) => {
                    let addr = request_register.addr_of(to).await;
                    make_response(
                        302,
                        format!(
                            "to=\"{}:{}\", commit_index={:?}",
                            addr.ip(),
                            addr.port(),
                            commit_index
                        ),
                    )
                }
                LocalResponseType::Command(command_reply) => {
                    make_response(command_reply.status, command_reply.info)
                }
            }
        }
    };
    Ok(response)
}

//////////////////////////////////////////////////////////////////////////////////////////

/// Listener for incoming connections
pub async fn listener(
    addr: SocketAddr,
    request_register: SharedRequestRegister,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listener = TcpListener::bind(addr).await?;

    info!(
        "Listening for incomming http connections on {}:{}",
        addr.ip().to_string(),
        addr.port()
    );

    // run accept cycle
    loop {
        let (stream, _) = listener.accept().await?;

        let io = hyper_util::rt::TokioIo::new(stream);

        // serve new connection
        tokio::task::spawn({
            let register = request_register.clone();
            async move {
                let serve_result = http1::Builder::new()
                    .serve_connection(
                        io,
                        service_fn(|request| {
                            let register = register.clone();
                            async move { serve_connection(request, register).await }
                        }),
                    )
                    .await;

                if let Err(err) = serve_result {
                    eprintln!("Error serving connection: {:?}", err);
                }
            }
        });
    }
}

//////////////////////////////////////////////////////////////////////////////////////////

fn get_uri_params(uri: &str) -> Option<HashMap<String, String>> {
    let pos = uri.find('?')?;
    let s = &uri[pos + 1..];
    let pairs = s.split_terminator('&').map(|t| t.split_once('='));
    if !pairs.clone().all(|x| x.is_some()) {
        None
    } else {
        Some(
            pairs
                .map(|p| p.unwrap())
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .collect::<HashMap<_, _>>(),
        )
    }
}

//////////////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::get_uri_params;

    #[test]
    fn parse_uri_params() {
        let p1 = get_uri_params("/?key=123&value=124").unwrap();
        assert_eq!(p1.len(), 2);
        assert_eq!(p1.get("key"), Some(&"123".to_owned()));
        assert_eq!(p1.get("value"), Some(&"124".to_owned()));
    }
}
