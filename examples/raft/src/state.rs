use dsbuild::{Address, Context, Message};
use log::info;
use serde::{Deserialize, Serialize};

use crate::{
    append::{AppendEntriesRequest, AppendEntriesResponse},
    cmd::Command,
    db::DataBase,
    disk::{append_value, read_all_values, read_last_value, rewrite_file},
    local::{InitializeResponse, LocalResponse, LocalResponseType, ReadValueRequest},
    log::{LogEntries, LogEntry},
    role::{LeaderInfo, Role},
    vote::{VoteRequest, VoteResponse},
};

/// Represents state of raft
pub struct RaftState {
    nodes: Vec<Address>,
    my_id: usize, // id of node in nodes list
    election_timeout: f64,
    heartbeat_timeout: f64,
    dump_state_timeout: f64,
    net_rtt: f64, // round trip time in network

    /// Index of candidate node voted for in current term
    vote_for: Option<usize>,

    /// Current term from article
    current_term: usize,

    /// Appeared logs (not all of them are committed)
    log: LogEntries,

    /// Current role of node
    role: Role,

    /// Index of last commited log (0-indexing)
    commit_index: i64,

    /// Index of log last applied to state machine,
    /// should increase while last_applied < commit_index
    /// (0-indexing)
    last_applied: i64,

    /// Instance of maintaining database
    db: DataBase,
}

//////////////////////////////////////////////////////////////////////////////////////////

pub const ELECTION_TIMER_NAME: &str = "election_timer";
pub const HEARTBEAT_TIMER_NAME: &str = "heartbeat_timer";

const VOTE_FOR_FILENAME: &str = "vote_for.txt";
const CURRENT_TERM_FILENAME: &str = "current_term.txt";

/// File of [`LogEntries`]
const LOG_FILENAME: &str = "log.txt";

const SEQ_NUM_FILENAME: &str = "seq_num.txt";

//////////////////////////////////////////////////////////////////////////////////////////

pub const DUMP_STATE_TIMER_NAME: &str = "dump_state_timer";

//////////////////////////////////////////////////////////////////////////////////////////

impl RaftState {
    pub fn new(nodes: Vec<Address>, my_id: usize, net_rtt: f64) -> Self {
        Self {
            nodes,
            my_id,
            election_timeout: net_rtt * 10.,
            heartbeat_timeout: net_rtt * 2.,
            dump_state_timeout: net_rtt,
            net_rtt,
            vote_for: None,
            current_term: 0,
            log: LogEntries::default(),
            role: Role::Follower(None),
            commit_index: -1,
            last_applied: -1,
            db: Default::default(),
        }
    }

    //////////////////////////////////////////////////////////////////////////////////////////
    // Initialization
    //////////////////////////////////////////////////////////////////////////////////////////

    pub async fn initialize(&mut self, ctx: Context) {
        // first set dump state timer
        self.set_dump_state_timeout(ctx.clone());

        let last_term = read_last_value(CURRENT_TERM_FILENAME, ctx.clone()).await;
        if let Some(last_term) = last_term {
            self.current_term = last_term;
        }

        let last_vote = read_last_value(VOTE_FOR_FILENAME, ctx.clone()).await;
        if let Some(last_vote) = last_vote {
            self.vote_for = last_vote;
        }

        self.log = read_all_values::<LogEntry>(LOG_FILENAME, ctx.clone()).await;

        self.transit_to_follower(None, ctx.clone());

        // finish initialization and response with last seq num
        ctx.send_local(
            InitializeResponse {
                seq_num: read_last_value(SEQ_NUM_FILENAME, ctx.clone())
                    .await
                    .unwrap_or(0),
            }
            .into(),
        );
    }

    //////////////////////////////////////////////////////////////////////////////////////////
    // Working with persistent state
    //////////////////////////////////////////////////////////////////////////////////////////

    async fn change_current_term(&mut self, new_current_term: usize, ctx: Context) {
        append_value(CURRENT_TERM_FILENAME, new_current_term, ctx).await;
        self.current_term = new_current_term;
    }

    async fn change_vote_for(&mut self, new_vote_for: Option<usize>, ctx: Context) {
        append_value(VOTE_FOR_FILENAME, new_vote_for, ctx).await;
        self.vote_for = new_vote_for;
    }

    async fn append_log(&mut self, log_entry: LogEntry, ctx: Context) {
        append_value(LOG_FILENAME, log_entry.clone(), ctx).await;
        self.log.push(log_entry);
    }

    async fn change_seq_num(&mut self, new_seq_num: usize, ctx: Context) {
        append_value(SEQ_NUM_FILENAME, new_seq_num, ctx).await;
    }

    //////////////////////////////////////////////////////////////////////////////////////////
    // Hanlders for external events
    //////////////////////////////////////////////////////////////////////////////////////////

    pub async fn on_command_request(&mut self, command: Command, ctx: Context) {
        // if i am leader, i must increase my match index by one
        let last_log_index = self.last_log_index();
        let is_leader = if let Role::Leader(info) = &mut self.role {
            info.match_index[self.my_id] = last_log_index + 1; // for the folowing append
            true
        } else {
            false
        };

        // update sequence number
        self.change_seq_num(command.id.sequence_number(), ctx.clone())
            .await;

        // leader can append log entry
        if is_leader {
            self.append_log(LogEntry::new(self.current_term, command), ctx.clone())
                .await;
            self.forward_commit_index();
            self.apply_commands(ctx);
        } else {
            // any other node must redirect command to leader (if it is known)
            let response_tp = match &self.role {
                Role::Leader(_) => panic!("impossible, as is_leader=false"),
                Role::Candidate(_) | Role::Follower(None) => LocalResponseType::Unavailable(),
                Role::Follower(Some(leader_id)) => {
                    LocalResponseType::RedirectedTo(*leader_id, None)
                }
            };
            ctx.send_local(LocalResponse::new(command.id, response_tp).into());
        }
    }

    pub async fn on_read_value_request(&mut self, request: ReadValueRequest, ctx: Context) {
        let request_id = request.request_id;
        self.change_seq_num(request_id.sequence_number(), ctx.clone())
            .await;

        let tp = match &self.role {
            Role::Leader(_) => {
                let replica = self.select_fresh_node(request_id.sequence_number());
                if replica == self.my_id {
                    LocalResponseType::ReadValue(self.db.read_value(&request.key))
                } else {
                    LocalResponseType::RedirectedTo(replica, Some(self.commit_index))
                }
            }
            Role::Candidate(_) | Role::Follower(None) => LocalResponseType::Unavailable(),
            Role::Follower(Some(leader_id)) => {
                // follower which knows leader can answer the request
                match request.min_commit_id {
                    None => LocalResponseType::RedirectedTo(*leader_id, None),
                    Some(idx) if idx == self.commit_index => {
                        LocalResponseType::ReadValue(self.db.read_value(&request.key))
                    }
                    Some(_) => LocalResponseType::RedirectedTo(*leader_id, None),
                }
            }
        };
        let response = LocalResponse::new(request_id, tp);
        ctx.send_local(response.into());
    }

    fn select_fresh_node(&self, shift: usize) -> usize {
        let node_cnt = self.nodes.len();
        match &self.role {
            Role::Leader(info) => (0..node_cnt)
                .map(|i| (i + shift) % node_cnt)
                .find(|node| info.commit_index[*node] == self.commit_index)
                .unwrap(),
            _ => panic!("only leader can select fresh node"),
        }
    }

    pub async fn on_election_timeout(&mut self, ctx: Context) {
        // current role is candidate or follower
        match &self.role {
            Role::Leader(_leader_info) => {
                // leader is forbidden by protocol here
                panic!("on_election_timeout: leader can not get election timeout")
            }
            Role::Follower(_) | Role::Candidate(_) => {
                // change current role on candidate
                self.role = Role::Candidate(0);

                // increment current term
                self.change_current_term(self.current_term + 1, ctx.clone())
                    .await;

                // vote for noone
                self.change_vote_for(None, ctx.clone()).await;

                // broadcast vote request
                self.broadcast_vote_request(ctx.clone());

                // reset election timer
                self.set_election_timer(ctx);
            }
        }
    }

    pub async fn on_heartbeat_timeout(&mut self, ctx: Context) {
        // here i need send heartbeat to every node
        assert!(matches!(self.role, Role::Leader(_)));

        // make heartbeat
        // let heartbeat: Message = self.make_heartbeat().into();

        // send heartbeats for all nodes (except of me)
        if let Role::Leader(info) = &self.role {
            self.nodes
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != self.my_id)
                .for_each(|(node, addr)| {
                    let ctx = ctx.clone();
                    let prev_index = info.next_index[node] - 1;
                    let message = self.make_append_request(prev_index).into();
                    // let hb = heartbeat.clone();
                    let addr = addr.clone();
                    let timeout = self.net_rtt;
                    ctx.clone().spawn(async move {
                        let _ = ctx.send_with_ack(message, addr, timeout).await;
                    });
                });
        }

        // reset heartbeat timer
        self.set_heartbeat_timer(ctx);
    }

    // i must send answer on every append entries,
    // because leader send append entries as responses on heartbeats
    pub async fn on_append_entries_request(&mut self, request: AppendEntriesRequest, ctx: Context) {
        let heartbeat = if !request.entries.is_empty() {
            info!(
                "APPEND REQUEST:  id={},  term={},  request={:?}",
                self.my_id, self.current_term, request
            );
            false
        } else {
            true
        };

        // transit to follower if received message from future term
        self.check_term_and_mb_become_follower(request.term, ctx.clone())
            .await;

        // if message outdated, i do not accept request
        if request.term != self.current_term {
            let reply = self.make_append_response(None, heartbeat);
            self.send_async_message(reply.into(), request.leader_id, ctx);
            return;
        }

        // i can get request in the same term as mine only from leader,
        // so i can be only follower or candidate, because there can
        // not be multiple leaders with the same term
        match self.role {
            Role::Follower(None) | Role::Candidate(_) => {
                self.transit_to_follower(Some(request.leader_id), ctx.clone())
            }
            Role::Follower(Some(leader)) => assert_eq!(leader, request.leader_id),
            _ => panic!("on_append_entries_request: process got request in incorrect role"),
        }

        // here i can be only in follower role
        self.reset_election_timer(ctx.clone());

        // try to apply logs
        let can_append_entries = self.can_append_entries(&request);
        let match_index = if can_append_entries {
            let match_index = self.update_log(&request, ctx.clone()).await;
            self.update_commit_index_and_apply_commands(&request, ctx.clone());
            Some(match_index)
        } else {
            None
        };

        // reply
        let reply = self.make_append_response(match_index, heartbeat);
        self.send_async_message(reply.into(), request.leader_id, ctx);
    }

    // i send append entries requests sequentially as responses on append requests
    pub async fn on_append_entries_response(
        &mut self,
        response: AppendEntriesResponse,
        ctx: Context,
    ) {
        if !response.heartbeat {
            info!(
                "APPEND RESPONSE:  id={},  term={},  response={:?}",
                self.my_id, self.current_term, response
            );
        }

        // here term can not be greater than my current term
        assert!(response.term <= self.current_term);

        // if commit index is greater we need to increase it
        self.check_commit_index(response.commit_index, ctx.clone())
            .await;

        // do something only if i am leader
        let is_leader = if let Role::Leader(info) = &mut self.role {
            // here it is not important if responder term is less than current leader term
            let respondent_id = response.respondent_id;

            // update commit index if i can
            if response.commit_index > info.commit_index[respondent_id] {
                info.commit_index[respondent_id] = response.commit_index;
            }

            if response.success {
                // may update match index
                if response.match_index > info.match_index[respondent_id] {
                    info.match_index[respondent_id] = response.match_index;
                    info.next_index[respondent_id] = response.match_index + 1;
                }
            } else {
                // if next_index was zero, then we must match
                assert!(info.next_index[respondent_id] > 0);
                info.next_index[respondent_id] -= 1;
            }

            // may send append entries again
            // self.send_append_entries_request(respondent_id, ctx.clone());

            true
        } else {
            false
        };

        // i can try foward commit index if i am leader
        if is_leader {
            self.forward_commit_index();
            self.apply_commands(ctx);
        }
    }

    pub async fn on_vote_request(&mut self, request: VoteRequest, ctx: Context) {
        info!("VOTE_REQUEST: id={}, request={:?}", self.my_id, request);

        // if term in request is greater than current term,
        // i must transit to follower
        self.check_term_and_mb_become_follower(request.term, ctx.clone())
            .await;

        // here i can vote for myself
        let vote_granted = self.can_grant_vote(&request);
        if vote_granted {
            self.change_vote_for(Some(request.candidate_id), ctx.clone())
                .await;
        }

        // send response
        let vote_response = self.make_vote_response(vote_granted);
        self.send_async_message(vote_response.into(), request.candidate_id, ctx);
    }

    pub async fn on_vote_response(&mut self, response: VoteResponse, ctx: Context) {
        info!("VOTE_RESPONSE: id={}, response={:?}", self.my_id, response);

        // if term in request is greater than current term,
        // i must transit to follower
        self.check_term_and_mb_become_follower(response.term, ctx.clone())
            .await;

        // check commit index in case its greater than mine
        self.check_commit_index(response.commit_index, ctx.clone())
            .await;

        // outdated message
        // in can not be greater
        if response.term != self.current_term || !response.vote_granted {
            return;
        }

        // if i am candidate and i received majority of votes,
        // then i transit to leader
        if let Role::Candidate(mut votes_granted) = self.role {
            votes_granted += 1;
            self.role = Role::Candidate(votes_granted);
            if votes_granted > self.nodes.len() / 2 {
                self.transit_to_leader(ctx);
            }
        }
    }

    //////////////////////////////////////////////////////////////////////////////////////////
    // Role transitions
    //////////////////////////////////////////////////////////////////////////////////////////

    /// Change role on follower and set election timer
    fn transit_to_follower(&mut self, leader: Option<usize>, ctx: Context) {
        self.role = Role::Follower(leader);

        // if i was leader
        self.remove_hearbeat_timer(ctx.clone());

        // set election timer
        self.set_election_timer(ctx);
    }

    /// Change role from candidate to leader
    /// when majority of votes are for me in current term
    fn transit_to_leader(&mut self, ctx: Context) {
        info!(
            "TRANSIT_TO_LEADER: process {:?} transit to leader",
            self.my_id
        );

        // create leader info and set match index for myself to mine log size
        let mut info = LeaderInfo::new(self.nodes.len(), self.log.len());
        info.match_index[self.my_id] = self.last_log_index();
        info.commit_index[self.my_id] = self.commit_index;

        self.role = Role::Leader(info);
        self.remove_election_timer(ctx.clone());
        self.set_heartbeat_timer(ctx.clone());

        // forward commit index according to majority rule
        // and apply commands
        self.forward_commit_index();
        self.apply_commands(ctx);

        // as leader we should transfer logs to other replicas
        // which will be done after replicas response on heartbeats
    }

    //////////////////////////////////////////////////////////////////////////////////////////
    // Common methods
    //////////////////////////////////////////////////////////////////////////////////////////

    pub async fn check_term_and_mb_become_follower(&mut self, new_term: usize, ctx: Context) {
        if new_term > self.current_term {
            self.change_current_term(new_term, ctx.clone()).await;
            self.change_vote_for(None, ctx.clone()).await;
            self.transit_to_follower(None, ctx);
        }
    }

    pub fn send_async_message(&self, message: Message, receiver_id: usize, ctx: Context) {
        let receiver = self.nodes[receiver_id].clone();
        let timeout = self.net_rtt;
        ctx.clone().spawn(async move {
            let _ = ctx.send_with_ack(message, receiver, timeout).await;
        });
    }

    //////////////////////////////////////////////////////////////////////////////////////////
    // Leader election utility
    //////////////////////////////////////////////////////////////////////////////////////////

    fn can_grant_vote(&self, vote_request: &VoteRequest) -> bool {
        // outdated  message
        if self.current_term > vote_request.term {
            return false;
        }

        // check if it is good candidate:
        // i must not vote for noone in current term or
        // i voted for him already (which seem impossible)
        let good_candidate = self
            .vote_for
            .map(|val| val == vote_request.candidate_id)
            .unwrap_or(true);
        if !good_candidate {
            return false;
        }

        // i can not vote for the same candidate twice in one term
        assert!(self.vote_for.is_none());

        // candidate's log should be at least up-to-date as mine
        (vote_request.last_log_term, vote_request.last_log_index)
            >= (self.last_log_term(), self.last_log_index())
    }

    //////////////////////////////////////////////////////////////////////////////////////////
    // Append entries utility
    //////////////////////////////////////////////////////////////////////////////////////////

    fn can_append_entries(&self, append_request: &AppendEntriesRequest) -> bool {
        // message not outdated and
        // leader's log must match with mine in corresponding index
        let (log_index, log_term, term) = (
            append_request.prev_log_index,
            append_request.prev_log_term,
            append_request.term,
        );
        self.current_term == term
            && self.last_log_index() >= log_index
            && self.get_log_term(log_index) == log_term
    }

    // returns last matching index
    async fn update_log(&mut self, request: &AppendEntriesRequest, ctx: Context) -> i64 {
        // find number of equal elements
        let mut equals_cnt: usize = 0;
        let prev_index = request.prev_log_index;
        let last_index = self.last_log_index();
        while prev_index + (equals_cnt as i64) < last_index
            && equals_cnt < request.entries.len()
            && self.log[(prev_index + 1) as usize + equals_cnt] == request.entries[equals_cnt]
        {
            equals_cnt += 1;
        }

        // then not all elements matches and we need extend log (with rewriting maybe)
        if equals_cnt != request.entries.len() {
            // remove conflicts
            let mut removed = false;
            while self.log.len() > (prev_index + 1) as usize {
                self.log.pop();
                removed = true;
            }

            self.log.extend_from_slice(&request.entries);
            if removed {
                // rewrite log
                rewrite_file(LOG_FILENAME, self.log.clone(), ctx).await;
            } else {
                // just append in the end
                for value in request
                    .entries
                    .iter()
                    .rev()
                    .take(request.entries.len() - equals_cnt)
                    .rev()
                {
                    append_value(LOG_FILENAME, value, ctx.clone()).await;
                }
            }
            self.last_log_index()
        } else {
            prev_index + (equals_cnt as i64)
        }
    }

    fn update_commit_index_and_apply_commands(
        &mut self,
        request: &AppendEntriesRequest,
        ctx: Context,
    ) {
        if self.commit_index < request.leaders_commit {
            self.update_commit_index(request.leaders_commit);
        }
        self.apply_commands(ctx);
    }

    fn apply_commands(&mut self, ctx: Context) {
        while self.last_applied < self.commit_index {
            self.last_applied += 1;
            let reply = self
                .db
                .apply_command(self.log[self.last_applied as usize].command.clone());

            info!(
                "APPLY_COMMANDS: proc[{}] applied command[{}]={:?}",
                self.my_id, self.last_applied, self.log[self.last_applied as usize].command
            );

            if reply.command_id.responsible_server() == self.my_id {
                let response_id = reply.command_id;
                let response_tp = LocalResponseType::Command(reply);
                let response = LocalResponse::new(response_id, response_tp);
                ctx.send_local(response.into());
            }
        }
    }

    //////////////////////////////////////////////////////////////////////////////////////////

    // i send append entries as responses on heartbeats's responses
    // fn send_append_entries_request(&self, receiver_id: usize, ctx: Context) {
    //     // get index of log entry to send
    //     let next_index = if let Role::Leader(info) = &self.role {
    //         info.next_index[receiver_id]
    //     } else {
    //         panic!("only leader can send append entries requests")
    //     };

    //     // next index must be >= 0
    //     assert!(next_index >= 0 && next_index <= self.last_log_index() + 1);

    //     // create request and send it
    //     let request = self.make_append_request(next_index - 1);
    //     self.send_async_message(request.into(), receiver_id, ctx);
    // }

    // allows to increase commit index on leader according to 'majority' rule
    fn forward_commit_index(&mut self) {
        if let Role::Leader(info) = &mut self.role {
            let new_commit_index = info.commit_index();

            // info may be inconsistent for new leaders
            if new_commit_index > self.commit_index {
                self.update_commit_index(new_commit_index);
            }
        } else {
            panic!("only leader can forward commit index")
        }
    }

    // allows to update commit index
    fn update_commit_index(&mut self, new_commit_index: i64) {
        assert!(new_commit_index >= self.commit_index);

        // commit index could be greater than last
        // log index in cases of consistency delay (???)
        self.commit_index = new_commit_index.min(self.last_log_index());
        if let Role::Leader(info) = &mut self.role {
            info.commit_index[self.my_id] = self.commit_index;
        }
    }

    //////////////////////////////////////////////////////////////////////////////////////////
    // Helpers for requests and responses
    //////////////////////////////////////////////////////////////////////////////////////////

    async fn check_commit_index(&mut self, mb_commit_index: i64, ctx: Context) {
        if self.commit_index < mb_commit_index {
            self.update_commit_index(mb_commit_index);
        }
        self.apply_commands(ctx);
    }

    /// Allows to get term for log on provided index
    fn get_log_term(&self, index: i64) -> i64 {
        if index == -1 {
            -1
        } else {
            self.log[index as usize].term as i64
        }
    }

    /// Returns index of the last log (or -1 if there are no logs)
    fn last_log_index(&self) -> i64 {
        self.log.len() as i64 - 1
    }

    /// Returns term of the last log (or -1 if there are no logs)
    fn last_log_term(&self) -> i64 {
        self.log.last().map(|e| e.term as i64).unwrap_or(-1)
    }

    fn make_vote_request(&self) -> VoteRequest {
        VoteRequest {
            term: self.current_term,
            candidate_id: self.my_id,
            last_log_index: self.log.len() as i64 - 1,
            last_log_term: self.log.last().map(|x| x.term as i64).unwrap_or(-1),
        }
    }

    fn make_vote_response(&self, vote_granted: bool) -> VoteResponse {
        VoteResponse {
            responder_id: self.my_id,
            term: self.current_term,
            vote_granted,
            commit_index: self.commit_index,
        }
    }

    fn make_append_request(&self, prev_log_index: i64) -> AppendEntriesRequest {
        let prev_log_term = if prev_log_index == -1 {
            -1
        } else {
            self.log[prev_log_index as usize].term as i64
        };
        let entries = self.log[(prev_log_index + 1) as usize..].to_vec();
        AppendEntriesRequest {
            term: self.current_term,
            leader_id: self.my_id,
            prev_log_index,
            prev_log_term,
            entries,
            leaders_commit: self.commit_index,
        }
    }

    // fn make_heartbeat(&self) -> AppendEntriesRequest {
    //     AppendEntriesRequest {
    //         term: self.current_term,
    //         leader_id: self.my_id,
    //         prev_log_index: -1,
    //         prev_log_term: -1,
    //         entries: Vec::new(),
    //         leaders_commit: self.commit_index,
    //     }
    // }

    fn make_append_response(
        &self,
        match_index: Option<i64>,
        heartbeat: bool,
    ) -> AppendEntriesResponse {
        AppendEntriesResponse {
            respondent_id: self.my_id,
            term: self.current_term,
            success: match_index.is_some(),
            match_index: match_index.unwrap_or(-1),
            commit_index: self.commit_index,
            heartbeat,
        }
    }

    //////////////////////////////////////////////////////////////////////////////////////////
    // Election utility
    //////////////////////////////////////////////////////////////////////////////////////////

    fn broadcast_vote_request(&self, ctx: Context) {
        let message: Message = self.make_vote_request().into();
        for id in 0..self.nodes.len() {
            self.send_async_message(message.clone(), id, ctx.clone());
        }
    }

    //////////////////////////////////////////////////////////////////////////////////////////
    // Timer utilities
    //////////////////////////////////////////////////////////////////////////////////////////

    fn set_election_timer(&self, ctx: Context) {
        self.remove_election_timer(ctx.clone());
        let ratio = 1. + (self.my_id as f64) / (self.nodes.len() as f64);
        ctx.set_timer(ELECTION_TIMER_NAME, self.election_timeout * ratio);
    }

    fn set_heartbeat_timer(&self, ctx: Context) {
        self.remove_hearbeat_timer(ctx.clone());
        ctx.set_timer(HEARTBEAT_TIMER_NAME, self.heartbeat_timeout);
    }

    fn remove_election_timer(&self, ctx: Context) {
        ctx.cancel_timer(ELECTION_TIMER_NAME);
    }

    fn remove_hearbeat_timer(&self, ctx: Context) {
        ctx.cancel_timer(HEARTBEAT_TIMER_NAME);
    }

    fn reset_election_timer(&self, ctx: Context) {
        self.set_election_timer(ctx);
    }

    //////////////////////////////////////////////////////////////////////////////////////////
    // Debug utility
    //////////////////////////////////////////////////////////////////////////////////////////

    fn state_info(&self) -> StateInfo {
        StateInfo {
            role: self.role.clone(),
            current_term: self.current_term,
            last_applied: self.last_applied,
            commit_index: self.commit_index,
        }
    }

    fn set_dump_state_timeout(&self, ctx: Context) {
        ctx.set_timer(DUMP_STATE_TIMER_NAME, self.dump_state_timeout);
    }

    pub fn on_dump_state_timeout(&self, ctx: Context) {
        let state_info = self.state_info();
        ctx.send_local(state_info.into());
        self.set_dump_state_timeout(ctx);
    }
}

//////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct StateInfo {
    pub role: Role,
    pub current_term: usize,
    pub last_applied: i64,
    pub commit_index: i64,
}

pub const STATE_INFO: &str = "state_info";

impl From<StateInfo> for Message {
    fn from(info: StateInfo) -> Self {
        Message::new(STATE_INFO, &info).unwrap()
    }
}

impl From<Message> for StateInfo {
    fn from(message: Message) -> Self {
        assert_eq!(message.get_tip(), STATE_INFO);
        message.get_data::<StateInfo>().unwrap()
    }
}
