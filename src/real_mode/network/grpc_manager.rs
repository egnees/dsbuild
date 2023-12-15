use std::collections::VecDeque;
use std::net::AddrParseError;
use std::sync::{Arc, Mutex};

use super::manager::{Address, NetworkManagerTrait};
use crate::common::message::Message;
use crate::real_mode::events::Event;

pub mod message_passing {
    tonic::include_proto!("message_passing");
}

use message_passing::message_passing_client::MessagePassingClient;
use message_passing::message_passing_server::{MessagePassing, MessagePassingServer};
use message_passing::{SendMessageRequest, SendMessageResponse};

use tonic::{transport::Server, Request, Response, Status};

#[derive(Debug, Default)]
pub struct MessagePassingService {}

#[tonic::async_trait]
impl MessagePassing for MessagePassingService {
    async fn send_message(
        &self,
        request: Request<SendMessageRequest>,
    ) -> Result<Response<SendMessageResponse>, Status> {
        println!("Listener got request to send message: {:?}", request);

        let req = request.into_inner();

        let reply = SendMessageResponse {
            status: "sucess".to_string(),
        };

        Ok(Response::new(reply))
    }
}

pub struct GRpcNetworkManager {
    host: String,
    event_queue: Arc<Mutex<VecDeque<Event>>>,
}

impl GRpcNetworkManager {
    pub fn new(host: String, event_queue: Arc<Mutex<VecDeque<Event>>>) -> Self {
        Self { host, event_queue }
    }
}

impl NetworkManagerTrait for GRpcNetworkManager {
    fn send_message(
        &mut self,
        sender_process: &str,
        msg: &Message,
        to: &Address,
    ) -> Result<(), String> {
        let request = tonic::Request::new(SendMessageRequest {
            to_address: to.host.clone(),
            to_process: to.process_name.clone(),
            from_address: self.host.clone(),
            from_process: sender_process.to_string(),
            message_tip: msg.get_tip().to_string(),
            message_data: msg.get_raw_data().to_vec(),
        });

        let receiver_host = to.host.clone();

        tokio::spawn(async move {
            let mut client = MessagePassingClient::connect(format!("http://{}", receiver_host))
                .await
                .expect("Can not connect to the receiver");

            let response = client
                .send_message(request)
                .await
                .expect("Can not send message to the receiver");

            println!("got response={:?}", response);
        });

        Ok(())
    }

    fn start_listen(&mut self) -> Result<(), String> {
        let addr = self
            .host
            .parse()
            .map_err(|err: AddrParseError| err.to_string())?;
        let service = MessagePassingService::default();

        tokio::spawn(async move {
            Server::builder()
                .add_service(MessagePassingServer::new(service))
                .serve(addr)
                .await
                .expect("Can not create server");
        });

        Ok(())
    }
}
