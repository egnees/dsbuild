use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use dsbuild::Message;
use raft::{
    cmd::{Command, CommandId, CommandType},
    local::{LocalResponse, ReadValueRequest},
};
use tokio::sync::{mpsc, oneshot, Mutex};

//////////////////////////////////////////////////////////////////////////////////////////

pub struct RequestRegister {
    seq_num: usize,
    nodes: Vec<SocketAddr>,
    my_id: usize,
    registry: HashMap<usize, oneshot::Sender<LocalResponse>>,
    process_local_sender: mpsc::Sender<dsbuild::Message>,
}

//////////////////////////////////////////////////////////////////////////////////////////

impl RequestRegister {
    fn next_seq_num(&mut self) -> usize {
        self.seq_num += 1;
        self.seq_num
    }

    pub fn new(
        seq_num: usize,
        nodes: Vec<SocketAddr>,
        my_id: usize,
        process_local_sender: mpsc::Sender<dsbuild::Message>,
    ) -> Self {
        Self {
            seq_num,
            nodes,
            my_id,
            registry: HashMap::default(),
            process_local_sender,
        }
    }

    /// Allows to register local message by its generation function
    async fn register_local_message(
        &mut self,
        message_maker: impl FnOnce(CommandId) -> Message,
    ) -> oneshot::Receiver<LocalResponse> {
        // make sender and receiver for current cmd
        let (sender, receiver) = oneshot::channel();

        // get current command unique sequence number
        let seq_num = self.next_seq_num();

        // insert command sender into storage
        let prev_on_seq_num = self.registry.insert(seq_num, sender);
        assert!(prev_on_seq_num.is_none());

        // make message
        let command_id = CommandId(self.my_id, seq_num);
        let message = message_maker(command_id);

        // send local message to process
        self.process_local_sender.send(message).await.unwrap();

        // return receiver
        receiver
    }

    /// Allows to response on previously registered command or read request
    fn respond(&mut self, response: LocalResponse) {
        let id = response.request_id;
        assert_eq!(id.responsible_server(), self.my_id);

        // send may fail, which means receiver was dropped
        if let Some(sender) = self.registry.remove(&id.sequence_number()) {
            let _ = sender.send(response);
        }
    }
}

//////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
pub struct SharedRequestRegister(Arc<Mutex<RequestRegister>>);

impl SharedRequestRegister {
    /// Allows to register command
    pub async fn register_command(
        &self,
        command_type: CommandType,
    ) -> oneshot::Receiver<LocalResponse> {
        self.0
            .lock()
            .await
            .register_local_message(|command_id| Command::new(command_type, command_id).into())
            .await
    }

    /// Allows to register read request
    pub async fn register_read_request(
        &self,
        key: String,
        min_commit_id: Option<i64>,
    ) -> oneshot::Receiver<LocalResponse> {
        self.0
            .lock()
            .await
            .register_local_message(|command_id| {
                ReadValueRequest {
                    key,
                    request_id: command_id,
                    min_commit_id,
                }
                .into()
            })
            .await
    }

    /// Allows to send local response to registered receiver
    pub async fn respond(&self, response: LocalResponse) {
        self.0.lock().await.respond(response);
    }

    pub fn new(register: RequestRegister) -> Self {
        Self(Arc::new(Mutex::new(register)))
    }

    /// Allows to get nodes in system
    pub async fn addr_of(&self, of: usize) -> SocketAddr {
        self.0.lock().await.nodes[of]
    }
}
