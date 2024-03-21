use dsbuild::Address;

mod client;
mod server;

use client::{parser::parse_request, process::Client};

use crate::client::parser::ParseError;

fn main() {
    let _ = Client::new("127", Address::new_ref("127", 12, "34"));

    assert_eq!(
        parse_request("123"),
        Err(ParseError::BadSyntax("123".to_string()))
    );
}
