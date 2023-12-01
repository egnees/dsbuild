mod virtual_context;
mod process_wrapper;

pub mod virtual_system;

#[cfg(test)]
mod tests {
    use crate::{common::{process::Process, context::Context, message::Message}, virtual_mode::virtual_system::VirtualSystem};


    #[test]
    fn test_ping_pong_works_in_simulation() {
        #[derive(Clone)]
        struct PingProcess {
            received_messages: Vec<Message>,
            to_ping: String,
            last_pong: u32,
        }

        impl PingProcess {
            fn send_ping(&mut self, ctx: &mut impl Context) {
                ctx.send_message(
                    Message { tip: "PING".to_string(), data: self.last_pong.to_string() }, 
                    self.to_ping.clone());
                ctx.set_timer("PONG_WAIT".to_string(), 0.1);
            }
        }

        impl Process for PingProcess {
            fn on_start(&mut self, ctx: &mut impl Context) -> Result<(), String> {
                self.send_ping(ctx);
                Ok(())
            }

            fn on_timer(&mut self, name: String, ctx: &mut impl Context) -> Result<(), String> {
                assert_eq!(name, "PONG_WAIT");

                self.send_ping(ctx);
                Ok(())
            }

            fn on_message(&mut self, msg: Message, _from: String, ctx: &mut impl Context) -> Result<(), String> {
                assert_eq!(msg.tip, "PONG");
                let pong_seq_num = u32::from_str_radix(msg.data.as_str(), 10)
                                                        .map_err(|_err| "Protocal failed")?;
                if pong_seq_num == self.last_pong + 1 {
                    // Next message in sequence
                    self.last_pong += 1;
                    self.received_messages.push(msg);
                    if self.last_pong < 10 {
                        self.send_ping(ctx);
                    } else {
                        ctx.cancel_timer("PONG_WAIT".to_string());
                        ctx.stop_process(false);
                    }
                }

                Ok(())
            }
        }

        // Process which answers pings and send pong
        // Stops after there are no pings in 0.2 seconds
        #[derive(Clone)]
        struct PongProcess {
        }

        impl Process for PongProcess {
            fn on_start(&mut self, ctx: &mut impl Context) -> Result<(), String> {
                ctx.set_timer("PINGS_ENDED".to_string(), 0.2);
                Ok(())
            }

            fn on_timer(&mut self, name: String, ctx: &mut impl Context) -> Result<(), String> {
                assert_eq!(name, "PINGS_ENDED");

                ctx.stop_process(false);
                Ok(())
            }

            fn on_message(&mut self, msg: Message, from: String, ctx: &mut impl Context) -> Result<(), String> {
                assert_eq!(msg.tip, "PING");

                let last_pong_seq_num = u32::from_str_radix(msg.data.as_str(), 10)
                                                        .map_err(|_err| "Protocal failed")?;
                
                ctx.send_message(Message { tip: "PONG".to_string(), data: (last_pong_seq_num + 1).to_string() }, from);
                
                ctx.set_timer("PINGS_ENDED".to_string(), 0.2);

                Ok(())
            }
        }

        let ping_proc_boxed = Box::new(PingProcess {
            received_messages: Vec::new(),
            to_ping: "pong_proc".to_string(),
            last_pong: 0,
        });

        let ping_proc: &'static mut PingProcess = Box::leak(ping_proc_boxed);

        let pong_proc_boxed = Box::new(PongProcess {
        });

        let pong_proc: &'static mut PongProcess = Box::leak(pong_proc_boxed);

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
}