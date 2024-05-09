//! Definition of replication logic.

use dsbuild::{Address, Context, Message, Tag};
use serde::{Deserialize, Serialize};

use super::event::ChatEvent;

/// Represents request to replicate event.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplicateEventRequest {
    pub total_seq_num: u64,
    pub event: ChatEvent,
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

impl From<ReplicateEventRequest> for Message {
    fn from(value: ReplicateEventRequest) -> Self {
        Message::borrow_new("replicate_event_request", value).unwrap()
    }
}

impl From<TotalSeqNumRequest> for Message {
    fn from(value: TotalSeqNumRequest) -> Self {
        Message::borrow_new("total_seq_num_request", value).unwrap()
    }
}

impl From<TotalSeqNumMsg> for Message {
    fn from(value: TotalSeqNumMsg) -> Self {
        Message::borrow_new("total_seq_num_msg", value).unwrap()
    }
}

impl From<ReceiveEventsRequest> for Message {
    fn from(value: ReceiveEventsRequest) -> Self {
        Message::borrow_new("receive_events_request", value).unwrap()
    }
}

/// Get total seq number on the replica with specified address.
pub async fn get_replica_total_seq_num(ctx: Context, tag: u64, address: Address) -> Option<u64> {
    ctx.send_recv_with_tag(TotalSeqNumRequest { tag }.into(), tag, address, 5.0)
        .await
        .map(|msg| msg.get_data::<TotalSeqNumMsg>().unwrap().total_seq_num)
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
        .map_or(false, |_| true)
}
