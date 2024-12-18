use dsbuild::Message;
use serde::{Deserialize, Serialize};

/// Sended from candidate to follower.
/// Candidate and leader can skip them
/// with optional answering if their term is greater
/// than passed. But if their term is less than passed,
/// they must transfer to the follower state.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct VoteRequest {
    /// Candidate's term
    pub term: usize,

    /// Candidate's id (sender)
    pub candidate_id: usize,

    /// Index of last log (0-indexing)
    pub last_log_index: i64,

    /// Term of last log (-1 if there are no logs)
    pub last_log_term: i64,
}

pub const VOTE_REQUEST: &str = "vote_request";

impl From<Message> for VoteRequest {
    fn from(msg: Message) -> Self {
        assert_eq!(msg.get_tip(), VOTE_REQUEST);
        msg.get_data::<VoteRequest>().unwrap()
    }
}

impl From<VoteRequest> for Message {
    fn from(request: VoteRequest) -> Self {
        Message::new(VOTE_REQUEST, &request).unwrap()
    }
}

//////////////////////////////////////////////////////////////////////////////////////////

/// Sended from follower to candidate.
/// Also can be sended from candidate or leader,
/// but they will not grant their vote.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct VoteResponse {
    /// Id of responder
    pub responder_id: usize,

    /// Term of the responder
    pub term: usize,

    /// Vote granted flag
    pub vote_granted: bool,

    /// Requester needs commit index
    /// to be able to update his log
    pub commit_index: i64,
}

pub const VOTE_RESPONSE: &str = "vote_response";

impl From<Message> for VoteResponse {
    fn from(msg: Message) -> Self {
        assert_eq!(msg.get_tip(), VOTE_RESPONSE);
        msg.get_data::<VoteResponse>().unwrap()
    }
}

impl From<VoteResponse> for Message {
    fn from(response: VoteResponse) -> Self {
        Message::new(VOTE_RESPONSE, &response).unwrap()
    }
}
