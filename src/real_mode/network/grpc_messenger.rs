use std::net::AddrParseError;

use async_trait::async_trait;
use tokio::sync::mpsc::Sender;

use super::messenger::{Address, AsyncMessenger, ProcessSendRequest, ProcessSendResponse};
use crate::common::message::Message;

pub mod message_passing {
    tonic::include_proto!("message_passing");
}

use message_passing::message_passing_client::MessagePassingClient;
use message_passing::message_passing_server::{MessagePassing, MessagePassingServer};
use message_passing::{SendMessageRequest, SendMessageResponse};

use tonic::{transport::Server, Request, Response, Status};

#[derive(Debug)]
pub struct MessagePassingService {
    pub request_sender: Sender<ProcessSendRequest>,
}

#[tonic::async_trait]
impl MessagePassing for MessagePassingService {
    async fn send_message(
        &self,
        request: Request<SendMessageRequest>,
    ) -> Result<Response<SendMessageResponse>, Status> {
        let req = request.into_inner();

        let sender_address = Address {
            host: req.sender_host,
            port: req.sender_port as u16,
            process_name: req.sender_process,
        };

        let receiver_address = Address {
            host: req.receiver_host,
            port: req.receiver_port as u16,
            process_name: req.receiver_process,
        };

        let message = Message::new_raw(&req.message_tip, &req.message_data)
            .map_err(|e| Status::new(tonic::Code::Internal, e))?;

        let message_request = ProcessSendRequest {
            sender_address,
            receiver_address,
            message,
        };

        self.request_sender
            .send(message_request)
            .await
            .map_err(|e| Status::new(tonic::Code::Unavailable, e.to_string()))?;

        let reply = SendMessageResponse {
            status: "success".to_string(),
        };

        Ok(Response::new(reply))
    }
}

#[derive(Default)]
pub struct GRpcMessenger {}

#[async_trait]
impl AsyncMessenger for GRpcMessenger {
    async fn send(request: ProcessSendRequest) -> Result<ProcessSendResponse, String> {
        let grpc_request = tonic::Request::new(SendMessageRequest {
            sender_host: request.sender_address.host.clone(),
            sender_port: u32::from(request.sender_address.port),
            sender_process: request.sender_address.process_name.clone(),
            receiver_host: request.receiver_address.host.clone(),
            receiver_port: u32::from(request.receiver_address.port),
            receiver_process: request.receiver_address.process_name.clone(),
            message_tip: request.message.get_tip().clone(),
            message_data: request.message.get_raw_data().to_vec(),
        });

        let receiver_host = request.receiver_address.host.clone();
        let receiver_port = request.receiver_address.port;

        let mut client =
            MessagePassingClient::connect(format!("http://{}:{}", receiver_host, receiver_port))
                .await
                .expect("Can not connect to the receiver");

        let response = client
            .send_message(grpc_request)
            .await
            .expect("Can not send message to the receiver");

        let process_response = ProcessSendResponse {
            status: response.into_inner().status,
        };

        Ok(process_response)
    }

    async fn listen(
        host: &str,
        port: u16,
        send_to: Sender<ProcessSendRequest>,
    ) -> Result<(), String> {
        let addr = format!("{}:{}", host, port)
            .parse()
            .map_err(|err: AddrParseError| err.to_string())?;

        let service = MessagePassingService {
            request_sender: send_to,
        };

        Server::builder()
            .add_service(MessagePassingServer::new(service))
            .serve(addr)
            .await
            .expect("Can not serve");

        Ok(())
    }
}
