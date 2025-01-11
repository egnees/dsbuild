use std::{future::Future, sync::Arc};

use dsbuild::{Address, Context, Message, Process};
use tokio::sync::Mutex;

use crate::{
    append::{
        AppendEntriesRequest, AppendEntriesResponse, APPEND_ENTRIES_REQUEST,
        APPEND_ENTRIES_RESPONSE,
    },
    cmd::{Command, COMMAND},
    local::{ReadValueRequest, INITIALIZE_REQUEST, READ_VALUE_REQUEST},
    state::{RaftState, DUMP_STATE_TIMER_NAME, ELECTION_TIMER_NAME, HEARTBEAT_TIMER_NAME},
    vote::{VoteRequest, VoteResponse, VOTE_REQUEST, VOTE_RESPONSE},
};

//////////////////////////////////////////////////////////////////////////////////////////

/// Serves as proxy between system and [`RaftState`]
/// Holds lock on state and blocks it during async calls
pub struct RaftProcess {
    state: Arc<Mutex<RaftState>>,
}

//////////////////////////////////////////////////////////////////////////////////////////

impl RaftProcess {
    pub fn new(nodes: Vec<Address>, my_id: usize, net_rtt: f64) -> Self {
        let state = RaftState::new(nodes, my_id, net_rtt);
        Self {
            state: Arc::new(Mutex::new(state)),
        }
    }

    //////////////////////////////////////////////////////////////////////////////////////////

    // Handler base
    fn call_async<
        F: Future<Output = ()> + Send,
        M: FnOnce(Arc<Mutex<RaftState>>, Context) -> F + Send + 'static,
    >(
        &self,
        callee: M,
        ctx: Context,
    ) {
        let state = self.state.clone();
        ctx.clone().spawn(async move {
            callee(state, ctx).await;
        });
    }

    //////////////////////////////////////////////////////////////////////////////////////////
    // Local message handlers
    //////////////////////////////////////////////////////////////////////////////////////////

    fn on_read_value_request(&self, request: ReadValueRequest, ctx: Context) {
        self.call_async(
            move |state, ctx| async move {
                state.lock().await.on_read_value_request(request, ctx).await;
            },
            ctx,
        );
    }

    fn on_command_request(&self, command: Command, ctx: Context) {
        self.call_async(move |state, ctx| async move {
            state.lock().await.on_command_request(command, ctx).await
        }, ctx);
    }

    fn on_initialize_request(&self, ctx: Context) {
        self.call_async(
            move |state, ctx| async move { state.lock().await.initialize(ctx).await },
            ctx,
        );
    }

    //////////////////////////////////////////////////////////////////////////////////////////
    // Timer handlers
    //////////////////////////////////////////////////////////////////////////////////////////

    fn on_election_timeout(&self, ctx: Context) {
        self.call_async(
            move |state, ctx| async move { state.lock().await.on_election_timeout(ctx).await },
            ctx,
        );
    }

    fn on_heartbeat_timeout(&self, ctx: Context) {
        self.call_async(
            move |state, ctx| async move {
                state.lock().await.on_heartbeat_timeout(ctx).await;
            },
            ctx,
        );
    }

    //////////////////////////////////////////////////////////////////////////////////////////
    // Message handlers
    //////////////////////////////////////////////////////////////////////////////////////////

    fn on_append_entries_request(&self, request: AppendEntriesRequest, ctx: Context) {
        self.call_async(
            move |s, c| async move { s.lock().await.on_append_entries_request(request, c).await },
            ctx,
        );
    }

    fn on_append_entries_response(&self, response: AppendEntriesResponse, ctx: Context) {
        self.call_async(
            move |s, c| async move { s.lock().await.on_append_entries_response(response, c).await },
            ctx,
        );
    }

    fn on_vote_request(&self, request: VoteRequest, ctx: Context) {
        self.call_async(
            move |s, c| async move { s.lock().await.on_vote_request(request, c).await },
            ctx,
        );
    }

    fn on_vote_response(&self, response: VoteResponse, ctx: Context) {
        self.call_async(
            move |s, c| async move { s.lock().await.on_vote_response(response, c).await },
            ctx,
        );
    }

    //////////////////////////////////////////////////////////////////////////////////////////
    // Debug utility
    //////////////////////////////////////////////////////////////////////////////////////////

    fn on_dump_state_timeout(&self, ctx: Context) {
        self.call_async(
            |s, c| async move {
                s.lock().await.on_dump_state_timeout(c);
            },
            ctx,
        );
    }
}

//////////////////////////////////////////////////////////////////////////////////////////

impl Process for RaftProcess {
    fn on_local_message(&mut self, msg: Message, ctx: Context) {
        match msg.get_tip().as_str() {
            READ_VALUE_REQUEST => self.on_read_value_request(msg.into(), ctx),
            COMMAND => self.on_command_request(msg.into(), ctx),
            INITIALIZE_REQUEST => self.on_initialize_request(ctx),
            _ => panic!("unsupported local message type"),
        }
    }

    fn on_timer(&mut self, name: String, ctx: Context) {
        match name.as_str() {
            ELECTION_TIMER_NAME => self.on_election_timeout(ctx),
            HEARTBEAT_TIMER_NAME => self.on_heartbeat_timeout(ctx),
            DUMP_STATE_TIMER_NAME => self.on_dump_state_timeout(ctx),
            _ => panic!("unexpected timer name"),
        }
    }

    fn on_message(&mut self, msg: Message, _from: Address, ctx: Context) {
        match msg.get_tip().as_str() {
            APPEND_ENTRIES_REQUEST => self.on_append_entries_request(msg.into(), ctx),
            APPEND_ENTRIES_RESPONSE => self.on_append_entries_response(msg.into(), ctx),
            VOTE_REQUEST => self.on_vote_request(msg.into(), ctx),
            VOTE_RESPONSE => self.on_vote_response(msg.into(), ctx),
            _ => panic!("unexpected message type"),
        }
    }
}
