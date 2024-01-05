use crate::{
    common::{context::Context, message::Message, process::Process},
    virtual_mode::virtual_system::VirtualSystem,
};

#[test]
fn test_ping_pong_works_in_simulation() {
    #[derive(Clone)]
    struct PingProcess {
        received_messages: Vec<Message>,
        to_ping: String,
        last_pong: u32,
    }

    impl PingProcess {
        fn send_ping(&mut self, ctx: &mut dyn Context) {
            ctx.send_message(
                Message::borrow_new("PING", self.last_pong.to_string())
                    .expect("Can not create message"),
                self.to_ping.clone(),
            );
            ctx.set_timer("PONG_WAIT".to_string(), 0.1);
        }
    }

    impl Process for PingProcess {
        fn on_start(&mut self, ctx: &mut dyn Context) -> Result<(), String> {
            self.send_ping(ctx);
            Ok(())
        }

        fn on_timer(&mut self, name: String, ctx: &mut dyn Context) -> Result<(), String> {
            assert_eq!(name, "PONG_WAIT");

            self.send_ping(ctx);
            Ok(())
        }

        fn on_message(
            &mut self,
            msg: Message,
            _from: String,
            ctx: &mut dyn Context,
        ) -> Result<(), String> {
            assert_eq!(msg.get_tip(), "PONG");
            let pong_seq_num = u32::from_str_radix(
                msg.get_data::<String>()
                    .expect("Can not fetch data")
                    .as_str(),
                10,
            )
            .map_err(|_err| "Protocal failed")?;
            if pong_seq_num == self.last_pong + 1 {
                // Next message in sequence
                self.last_pong += 1;
                self.received_messages.push(msg);
                if self.last_pong < 10 {
                    self.send_ping(ctx);
                } else {
                    ctx.cancel_timer("PONG_WAIT".to_string());
                    ctx.stop_process();
                }
            }

            Ok(())
        }
    }

    // Process which answers pings and send pong
    // Stops after there are no pings in 0.2 seconds
    #[derive(Clone)]
    struct PongProcess {}

    impl Process for PongProcess {
        fn on_start(&mut self, ctx: &mut dyn Context) -> Result<(), String> {
            ctx.set_timer("PINGS_ENDED".to_string(), 0.2);
            Ok(())
        }

        fn on_timer(&mut self, name: String, ctx: &mut dyn Context) -> Result<(), String> {
            assert_eq!(name, "PINGS_ENDED");

            ctx.stop_process();
            Ok(())
        }

        fn on_message(
            &mut self,
            msg: Message,
            from: String,
            ctx: &mut dyn Context,
        ) -> Result<(), String> {
            assert_eq!(msg.get_tip(), "PING");

            let last_pong_seq_num = u32::from_str_radix(
                msg.get_data::<String>()
                    .expect("Can not fetch data")
                    .as_str(),
                10,
            )
            .map_err(|_err| "Protocal failed")?;

            ctx.send_message(
                Message::borrow_new("PONG", (last_pong_seq_num + 1).to_string())
                    .expect("Can not create message"),
                from,
            );

            ctx.set_timer("PINGS_ENDED".to_string(), 0.2);

            Ok(())
        }
    }

    let ping_proc = PingProcess {
        received_messages: Vec::new(),
        to_ping: "pong_proc".to_string(),
        last_pong: 0,
    };

    let pong_proc = PongProcess {};

    let mut simulation = VirtualSystem::new(12345);

    simulation.add_node("node_1");
    simulation.add_process("ping_proc", ping_proc, "node_1");

    simulation.add_node("node_2");
    simulation.add_process("pong_proc", pong_proc, "node_2");

    simulation.network().set_delay(0.05);

    simulation.start("pong_proc", "node_2");
    simulation.start("ping_proc", "node_1");

    simulation.step_until_no_events();

    assert!(simulation.sent_message_count("pong_proc") >= 10);
    assert!(simulation.sent_message_count("ping_proc") >= 10);
}
