use std::sync::{Arc, Mutex};

use colored::Colorize;

use dsbuild::{Address, Context, Message, Process};

use crate::{client::requests::ClientRequest, server::messages::ServerMessage};

use super::state::{RoutedMessage, State};

#[derive(Clone)]
pub struct Server {
    state: Arc<Mutex<State>>,
}

impl Server {
    pub fn new(server_name: String) -> Self {
        let state = State::new(server_name);
        Self {
            state: Arc::new(Mutex::new(state)),
        }
    }

    fn process_routed_msg(routed: RoutedMessage, state: Arc<Mutex<State>>, ctx: Context) {
        let user = routed.to;
        let msg: Message = routed.msg.into();
        ctx.clone().spawn(async move {
            let routed_messages =
                Self::send_msg_to_user(user, msg, state.clone(), ctx.clone()).await;

            for routed in routed_messages {
                Self::process_routed_msg(routed, state.clone(), ctx.clone());
            }
        });
    }

    async fn send_msg_to_user(
        user: Address,
        msg: Message,
        state: Arc<Mutex<State>>,
        ctx: Context,
    ) -> Vec<RoutedMessage> {
        println!(
            "{} {} {}",
            &msg.get_data::<ServerMessage>().unwrap(),
            "->".green().bold(),
            user.process_name.green().bold().underline()
        );

        let send_result = ctx.send_reliable(msg, user.clone()).await;

        if send_result.is_err() {
            state.lock().unwrap().remove_auth_for_user(user)
        } else {
            Vec::new()
        }
    }
}

impl Process for Server {
    fn on_local_message(&mut self, _: Message, _: Context) -> Result<(), String> {
        unreachable!("server should not get local messages")
    }

    fn on_timer(&mut self, _: String, _: Context) -> Result<(), String> {
        unreachable!("server should not spawn timers")
    }

    fn on_message(&mut self, msg: Message, from: Address, ctx: Context) -> Result<(), String> {
        let client_request = msg.get_data::<ClientRequest>().unwrap();

        println!("{}", client_request);

        let routed_messages = self
            .state
            .lock()
            .unwrap()
            .process_client_request(from, client_request);

        for routed in routed_messages {
            Self::process_routed_msg(routed, self.state.clone(), ctx.clone());
        }

        Ok(())
    }
}
