use dsbuild::{Address, Context, Message};

use crate::{
    append::{AppendEntriesRequest, AppendEntriesResponse},
    db::DataBase,
    disk::{append_value, read_all_values, read_last_value, rewrite_file},
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

const ELECTION_TIMER_NAME: &str = "election_timer";
const HEARTBEAT_TIMER_NAME: &str = "heartbeat_timer";

const VOTE_FOR_FILENAME: &str = "vote_for.txt";
const CURRENT_TERM_FILENAME: &str = "current_term.txt";

/// File of [`LogEntries`]
const LOG_FILENAME: &str = "log.txt";

//////////////////////////////////////////////////////////////////////////////////////////

impl RaftState {
    pub fn new(nodes: Vec<Address>, my_id: usize, net_rtt: f64) -> Self {
        Self {
            nodes,
            my_id,
            election_timeout: net_rtt * 20.,
            heartbeat_timeout: net_rtt * 4.,
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
        let last_term = read_last_value(CURRENT_TERM_FILENAME, ctx.clone()).await;
        if let Some(last_term) = last_term {
            self.current_term = last_term;
        }

        let last_vote = read_last_value(VOTE_FOR_FILENAME, ctx.clone()).await;
        if let Some(last_vote) = last_vote {
            self.vote_for = last_vote;
        }

        self.log = read_all_values::<LogEntry>(LOG_FILENAME, ctx.clone()).await;

        self.transit_to_follower(None, ctx);
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

    //////////////////////////////////////////////////////////////////////////////////////////
    // Hanlders for external events
    //////////////////////////////////////////////////////////////////////////////////////////

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

                // vote for myself
                self.on_vote_request(self.make_vote_request(), ctx).await;

                // election timer still persists
            }
        }
    }

    pub async fn on_heartbeat_timeout(&mut self, ctx: Context) {
        // here i need send heartbeat to every node
        assert!(matches!(self.role, Role::Leader(_)));

        // make heartbeat
        let heartbeat: Message = self.make_heartbeat().into();

        // send heartbeats for all nodes (except of me)
        self.nodes
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != self.my_id)
            .for_each(|(_, addr)| {
                let ctx = ctx.clone();
                let hb = heartbeat.clone();
                let addr = addr.clone();
                let timeout = self.net_rtt;
                ctx.clone().spawn(async move {
                    let _ = ctx.send_with_ack(hb, addr, timeout).await;
                });
            });

        // heatbeat timer still persists
    }

    pub async fn on_append_entries_request(&mut self, request: AppendEntriesRequest, ctx: Context) {
        self.check_term_and_mb_become_follower(request.term, ctx.clone())
            .await;

        if let Role::Follower(None) = self.role {
            self.role = Role::Follower(Some(request.leader_id));
        }
    }

    pub async fn on_append_entries_response(
        &mut self,
        response: AppendEntriesResponse,
        ctx: Context,
    ) {
        self.check_term_and_mb_become_follower(response.term, ctx.clone())
            .await;
    }

    pub async fn on_vote_request(&mut self, request: VoteRequest, ctx: Context) {
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
        let dst = self.nodes[request.candidate_id].clone();
        let timeout = self.net_rtt;
        ctx.clone().spawn(async move {
            let _ = ctx.send_with_ack(vote_response.into(), dst, timeout).await;
        });
    }

    pub async fn on_vote_response(&mut self, response: VoteResponse, ctx: Context) {
        // if term in request is greater than current term,
        // i must transit to follower
        self.check_term_and_mb_become_follower(response.term, ctx.clone())
            .await;

        // outdated message
        // in can not be greater
        if response.term != self.current_term {
            return;
        }

        // if i am candidate and i received majority of votes,
        // then i transit to leader
        if let Role::Candidate(mut votes_granted) = self.role {
            votes_granted += 1;
            self.role = Role::Candidate(votes_granted);
            if votes_granted >= self.nodes.len().div_ceil(2) {
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
        let info = LeaderInfo::new(self.nodes.len(), self.log.len());
        self.role = Role::Leader(info);
        self.remove_election_timer(ctx.clone());
        self.set_heartbeat_timer(ctx);

        // here we should start transfer logs to other replicas
        todo!("transfer logs to other replicas")
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
            >= (self.get_last_log_term(), self.get_last_log_index())
    }

    //////////////////////////////////////////////////////////////////////////////////////////
    // Append entries utility
    //////////////////////////////////////////////////////////////////////////////////////////

    pub fn can_append_entries(&self, append_request: &AppendEntriesRequest) -> bool {
        // message not outdated and
        // leader's log must match with mine in corresponding index
        let (log_index, log_term, term) = (
            append_request.prev_log_index,
            append_request.prev_log_term,
            append_request.term,
        );
        self.current_term == term
            && self.get_last_log_index() >= log_index
            && self.get_log_term(log_index) == log_term
    }

    pub async fn update_log(&mut self, request: &AppendEntriesRequest, ctx: Context) {
        // find number of equal elements
        let mut equals_cnt = 0;
        let prev_index = request.prev_log_index;
        let last_index = self.get_last_log_index();
        while prev_index + equals_cnt <= last_index && equals_cnt < request.entries.len() as i64 {
            equals_cnt += 1;
        }

        // then not all elements matches and we need extend log (with rewriting maybe)
        if equals_cnt != request.entries.len() as i64 {
            // remove conflicts
            let new_len = prev_index + equals_cnt + 1;
            let mut need_rewrite_file = false;
            while self.log.len() as i64 != new_len {
                need_rewrite_file = true;
                self.log.pop();
            }

            // if there is some inconsistency with leader's log,
            // i need solve conflicts by rewriting file with only non-conflict part.
            if need_rewrite_file {
                rewrite_file(LOG_FILENAME, self.log.clone(), ctx.clone()).await;
            }

            // get entries which must be appended
            let mut entries_to_append = request.entries[..equals_cnt as usize].to_vec();

            // append them in file and in ram log
            for entry in entries_to_append.iter() {
                append_value(LOG_FILENAME, entry.clone(), ctx.clone()).await;
            }
            self.log.append(&mut entries_to_append);
        }
    }

    pub fn update_commit_index_and_apply_commands(&mut self, request: &AppendEntriesRequest) {
        if self.commit_index < request.leaders_commit {
            self.commit_index = request.leaders_commit;
        }
        while self.last_applied < self.commit_index {
            self.last_applied += 1;
            let reply = self
                .db
                .apply_command(self.log[self.last_applied as usize].command.clone());
        }
    }

    //////////////////////////////////////////////////////////////////////////////////////////
    // Helpers for requests and responses
    //////////////////////////////////////////////////////////////////////////////////////////

    /// Allows to get term for log on provided index
    fn get_log_term(&self, index: i64) -> i64 {
        if index == -1 {
            -1
        } else {
            self.log[index as usize].term as i64
        }
    }

    /// Returns index of the last log (or -1 if there are no logs)
    fn get_last_log_index(&self) -> i64 {
        self.log.len() as i64 - 1
    }

    /// Returns term of the last log (or -1 if there are no logs)
    fn get_last_log_term(&self) -> i64 {
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
        }
    }

    fn make_append_request(&self, prev_log_index: i64) -> AppendEntriesRequest {
        let prev_log_term = if prev_log_index == -1 {
            -1
        } else {
            self.log[prev_log_index as usize].term as i64
        };
        let entries = self.log[(prev_log_term + 1) as usize..].to_vec();
        AppendEntriesRequest {
            term: self.current_term,
            leader_id: self.my_id,
            prev_log_index,
            prev_log_term,
            entries,
            leaders_commit: self.commit_index,
        }
    }

    fn make_heartbeat(&self) -> AppendEntriesRequest {
        AppendEntriesRequest {
            term: self.current_term,
            leader_id: self.my_id,
            prev_log_index: -1,
            prev_log_term: -1,
            entries: Vec::new(),
            leaders_commit: self.commit_index,
        }
    }

    fn make_append_response(&self, success: bool) -> AppendEntriesResponse {
        AppendEntriesResponse {
            respondent_id: self.my_id,
            term: self.current_term,
            success,
        }
    }

    //////////////////////////////////////////////////////////////////////////////////////////
    // Timer utilities
    //////////////////////////////////////////////////////////////////////////////////////////

    fn set_election_timer(&self, ctx: Context) {
        ctx.set_timer(ELECTION_TIMER_NAME, self.election_timeout);
    }

    fn set_heartbeat_timer(&self, ctx: Context) {
        ctx.set_timer(HEARTBEAT_TIMER_NAME, self.heartbeat_timeout);
    }

    fn remove_election_timer(&self, ctx: Context) {
        ctx.cancel_timer(ELECTION_TIMER_NAME);
    }

    fn remove_hearbeat_timer(&self, ctx: Context) {
        ctx.cancel_timer(HEARTBEAT_TIMER_NAME);
    }
}
