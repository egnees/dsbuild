use crate::common::message::Message;

pub struct Address {
    pub host: String,
    pub process_name: String,
}

pub trait NetworkManagerTrait {
    fn send_message(
        &mut self,
        sender_proc: &str,
        msg: &Message,
        to: &Address,
    ) -> Result<(), String>;
    fn start_listen(&mut self) -> Result<(), String>;
}
