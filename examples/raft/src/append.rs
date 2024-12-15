use dsbuild::Message;
use serde::{Deserialize, Serialize};

use crate::log::LogEntries;

//////////////////////////////////////////////////////////////////////////////////////////

/// Sended from leader to follower
/// If candidate receives append request, it can skip it
/// with optional answering.
/// But if term is greater than candidate's term,
/// it must transfer to follower.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct AppendEntriesRequest {
    /// Leader's term
    pub term: usize,

    /// Id of leader (sender)
    pub leader_id: usize,

    /// Index of log entry immediately preceding new ones (0-indexing)
    pub prev_log_index: i64,

    /// Term of the prev log (-1 if there are no logs)
    pub prev_log_term: i64,

    /// Entries after [`AppendEntriesRequest::prev_log_index`]
    pub entries: LogEntries,

    /// Leader's commit index
    pub leaders_commit: i64,
}

const APPEND_ENTRIES_REQUEST: &str = "append_entries_request";

impl From<Message> for AppendEntriesRequest {
    fn from(msg: Message) -> Self {
        assert_eq!(msg.get_tip(), APPEND_ENTRIES_REQUEST);
        msg.get_data::<AppendEntriesRequest>().unwrap()
    }
}

impl From<AppendEntriesRequest> for Message {
    fn from(request: AppendEntriesRequest) -> Self {
        Message::new(APPEND_ENTRIES_REQUEST, &request).unwrap()
    }
}

//////////////////////////////////////////////////////////////////////////////////////////

/// Sended to leader in response on [`AppendEntriesRequest`]
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct AppendEntriesResponse {
    /// Id of respondent
    pub respondent_id: usize,

    /// Current term of the respondent
    pub term: usize,

    /// True if follower contained logs matching passed
    /// [`AppendEntriesRequest::prev_log_index`] and [`AppendEntriesRequest::prev_log_term`]
    pub success: bool,

    /// Represents last index matched in leader's and responder's logs
    pub match_index: i64,

    /// Commit index of the respondent
    pub commit_index: i64,
}

const APPEND_ENTRIES_RESPONSE: &str = "append_entries_response";

impl From<Message> for AppendEntriesResponse {
    fn from(msg: Message) -> Self {
        assert_eq!(msg.get_tip(), APPEND_ENTRIES_RESPONSE);
        msg.get_data::<AppendEntriesResponse>().unwrap()
    }
}

impl From<AppendEntriesResponse> for Message {
    fn from(response: AppendEntriesResponse) -> Self {
        Message::new(APPEND_ENTRIES_RESPONSE, &response).unwrap()
    }
}
