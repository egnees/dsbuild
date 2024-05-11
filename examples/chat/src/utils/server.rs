use dsbuild::Message;

pub fn check_replica_request() -> Message {
    Message::new("check_replica_request", &String::new()).unwrap()
}
