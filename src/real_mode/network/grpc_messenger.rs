//! Definition of asynchronous messenger [`GRpcMessenger`] structure.

use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

use async_trait::async_trait;
use tokio::sync::mpsc::Sender;
use tonic::transport::server::TcpIncoming;

use super::defs::*;

use super::messenger::AsyncMessenger;
use crate::common::message::Message;
use crate::common::process::Address;
use crate::real_mode::events::Event;

pub mod message_passing {
    tonic::include_proto!("message_passing");
}

use message_passing::message_passing_client::MessagePassingClient;
use message_passing::message_passing_server::{MessagePassing, MessagePassingServer};
use message_passing::{SendMessageRequest, SendMessageResponse};

use tonic::{transport::Server, Request, Response, Status};

#[derive(Debug)]
pub struct MessagePassingService {
    pub event_sender: Sender<Event>,
}

#[tonic::async_trait]
impl MessagePassing for MessagePassingService {
    async fn send_message(
        &self,
        request: Request<SendMessageRequest>,
    ) -> Result<Response<SendMessageResponse>, Status> {
        let req = request.into_inner();

        let sender_address =
            Address::new(req.sender_host, req.sender_port as u16, req.sender_process);

        let receiver_address = Address::new(
            req.receiver_host,
            req.receiver_port as u16,
            req.receiver_process,
        );

        let message = Message::new_raw(&req.message_tip, &req.message_data)
            .map_err(|e| Status::new(tonic::Code::Internal, e))?;

        let event = Event::MessageReceived {
            msg: message,
            from: sender_address,
            to: receiver_address.process_name,
        };

        self.event_sender
            .send(event)
            .await
            .map_err(|e| Status::new(tonic::Code::Unavailable, e.to_string()))?;

        let reply = SendMessageResponse {
            status: "success".to_owned(),
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
                .map_err(|e| "Can not connect to the receiver: ".to_owned() + &e.to_string())?;

        let response = client
            .send_message(grpc_request)
            .await
            .map_err(|e| "Can not send message to the receiver: ".to_owned() + &e.to_string())?;

        let process_response = ProcessSendResponse {
            status: response.into_inner().status,
        };

        Ok(process_response)
    }

    async fn listen(host: String, port: u16, send_to: Sender<Event>) -> Result<(), String> {
        // Create ip address.
        let ip_addr =
            IpAddr::from_str(&host).map_err(|e| "Invalid host: ".to_owned() + &e.to_string())?;

        // Create socket address.
        let sock_addr = SocketAddr::new(ip_addr, port);

        // Create incoming stream.
        let incoming_stream = TcpIncoming::new(sock_addr, true, None).map_err(|e| {
            "Can not create Tcp incoming stream: ".to_owned() + e.to_string().as_str()
        })?;

        // Create rpc server.
        let service = MessagePassingService {
            event_sender: send_to,
        };
        let server = MessagePassingServer::new(service);

        // Start the server.
        Server::builder()
            .add_service(server)
            .serve_with_incoming(incoming_stream)
            .await
            .map_err(|e| "GRpc messenger server error: ".to_owned() + e.to_string().as_str())?;

        Ok(())
    }
}
