//! Definition of replication logic.

use dsbuild::{Address, Context, Message, Tag};
use serde::{Deserialize, Serialize};

use crate::client::requests::ClientRequest;

/// Represents request to replicate event.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplicateRequest {
    pub seq_num: u64,
    pub client_request: ClientRequest,
}

/// Represents request to get receiver total seq number.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TotalSeqNumRequest {
    pub tag: Tag,
}

/// Represents message with the total sequence number on the sender node.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TotalSeqNumMsg {
    pub total_seq_num: u64,
}

/// Represents request from the node to receive events
/// with total sequence number from the specified range [from, to].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReceiveEventsRequest {
    pub from: u64,
    pub to: u64,
}

impl From<ReplicateRequest> for Message {
    fn from(value: ReplicateRequest) -> Self {
        Message::new("replicate_request", &value).unwrap()
    }
}

impl From<TotalSeqNumRequest> for Message {
    fn from(value: TotalSeqNumRequest) -> Self {
        Message::new("total_seq_num_request", &value).unwrap()
    }
}

impl From<TotalSeqNumMsg> for Message {
    fn from(value: TotalSeqNumMsg) -> Self {
        Message::new("total_seq_num_msg", &value).unwrap()
    }
}

impl From<ReceiveEventsRequest> for Message {
    fn from(value: ReceiveEventsRequest) -> Self {
        Message::new("receive_events_request", &value).unwrap()
    }
}

/// Get total seq number on the replica with specified address.
pub async fn get_replica_total_seq_num(ctx: Context, tag: u64, address: Address) -> Option<u64> {
    ctx.send_recv_with_tag(TotalSeqNumRequest { tag }.into(), tag, address, 5.0)
        .await
        .map(|msg| msg.data::<TotalSeqNumMsg>().unwrap().total_seq_num)
        .ok()
}

/// Request events with global sequence number in the range [`from`, `to`].
pub async fn request_replica_events_from_range(
    ctx: Context,
    from: u64,
    to: u64,
    address: Address,
) -> bool {
    ctx.send_with_ack(ReceiveEventsRequest { from, to }.into(), address, 5.0)
        .await
        .is_ok_and(|_| true)
}

pub async fn replicate_client_request(
    ctx: Context,
    replica: Address,
    client_request: ClientRequest,
    id: u64,
) -> bool {
    ctx.send_with_ack(
        ReplicateRequest {
            seq_num: id,
            client_request,
        }
        .into(),
        replica,
        5.0,
    )
    .await
    .is_ok_and(|_| true)
}
