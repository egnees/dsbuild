//! Definition of asynchronous messenger [`GRpcMessenger`] structure.

use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

use tokio::sync::mpsc::Sender;
use tonic::transport::server::TcpIncoming;

use crate::common::message::{Message, RoutedMessage};
use crate::common::process::Address;

pub mod message_passing {
    tonic::include_proto!("message_passing");
}

use message_passing::message_passing_client::MessagePassingClient;
use message_passing::message_passing_server::{MessagePassing, MessagePassingServer};
use message_passing::{SendMessageRequest, SendMessageResponse};

use tonic::{transport::Server, Request, Response, Status};

use crate::common::tag::Tag;

pub struct ProcessSendRequest {
    /// Address of process, which sends request.
    pub sender_address: Address,
    /// Address of process, which will receive request.
    pub receiver_address: Address,
    /// Passed message.
    pub message: Message,
    /// Optional message tag.
    pub tag: Option<Tag>,
}

/// Used to pass responses on [requests][`ProcessSendRequest`].
#[derive(Clone, Debug, PartialEq)]
pub struct ProcessSendResponse {
    /// Response message from receiver,
    /// which indicates whether request was accepted or not.
    pub status: String,
}

#[derive(Debug)]
pub struct MessagePassingService {
    pub send_to: Sender<RoutedMessage>,
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

        let msg = RoutedMessage {
            msg: message,
            from: sender_address,
            to: receiver_address,
            tag: req.tag,
        };

        self.send_to
            .send(msg)
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

impl GRpcMessenger {
    pub async fn send(request: ProcessSendRequest) -> Result<ProcessSendResponse, String> {
        let grpc_request = tonic::Request::new(SendMessageRequest {
            sender_host: request.sender_address.host.clone(),
            sender_port: u32::from(request.sender_address.port),
            sender_process: request.sender_address.process_name.clone(),
            receiver_host: request.receiver_address.host.clone(),
            receiver_port: u32::from(request.receiver_address.port),
            receiver_process: request.receiver_address.process_name.clone(),
            message_tip: request.message.get_tip().clone(),
            message_data: request.message.get_raw_data().to_vec(),
            tag: request.tag,
        });

        let receiver_host = request.receiver_address.host.clone();
        let receiver_port = request.receiver_address.port;

        let mut client =
            MessagePassingClient::connect(format!("http://{}:{}", receiver_host, receiver_port))
                .await
                .map_err(|e| "can not connect to the receiver: ".to_owned() + &e.to_string())?;

        let response = client
            .send_message(grpc_request)
            .await
            .map_err(|e| "can not send message to the receiver: ".to_owned() + &e.to_string())?;

        let process_response = ProcessSendResponse {
            status: response.into_inner().status,
        };

        Ok(process_response)
    }

    pub async fn listen(
        host: String,
        port: u16,
        send_to: Sender<RoutedMessage>,
    ) -> Result<(), String> {
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
        let service = MessagePassingService { send_to };
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
