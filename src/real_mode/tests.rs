use std::{
    collections::VecDeque,
    net::SocketAddr,
    ops::DerefMut,
    str::FromStr,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use crate::{
    common::{context::Context, message::Message, process::Process},
    real_mode::real_system::RealSystem,
};

use super::{
    events::Event,
    network_manager::NetworkManager,
    process_runner::{ProcessRunner, RunConfig},
    timer_manager::TimerManager,
};

#[test]
fn test_timer_manager() {
    let event_queue = Arc::new(Mutex::new(VecDeque::new()));

    let mut timer_manager = TimerManager::new(event_queue.clone());

    // set_timer works
    timer_manager.set_timer("timer_1", 0.1, false);

    thread::sleep(Duration::from_secs_f64(0.2));

    assert_eq!(event_queue.lock().unwrap().len(), 1);

    let first_event = event_queue
        .lock()
        .unwrap()
        .front()
        .expect("data race detected")
        .clone();
    assert_eq!(
        first_event,
        Event::TimerFired {
            name: "timer_1".to_string()
        }
    );

    // cancel_timer works
    event_queue.lock().unwrap().clear();

    timer_manager.set_timer("timer_2", 0.3, false);

    thread::sleep(Duration::from_secs_f64(0.1));

    timer_manager.cancel_timer("timer_2");

    thread::sleep(Duration::from_secs_f64(0.3));

    assert_eq!(event_queue.lock().unwrap().len(), 0);

    // cancel_all_timers works
    event_queue.lock().unwrap().clear();

    timer_manager.set_timer("timer_1", 0.3, false);
    timer_manager.set_timer("timer_2", 0.3, false);
    timer_manager.set_timer("timer_3", 0.3, false);

    thread::sleep(Duration::from_secs_f64(0.1));

    timer_manager.cancel_all_timers();

    thread::sleep(Duration::from_secs_f64(0.3));

    assert_eq!(event_queue.lock().unwrap().len(), 0);

    // override works
    event_queue.lock().unwrap().clear();

    timer_manager.set_timer("timer_1", 1.0, false);
    timer_manager.set_timer("timer_1", 0.1, true);
    timer_manager.set_timer("timer_1", 2.0, false);

    thread::sleep(Duration::from_secs_f64(0.2));

    assert_eq!(event_queue.lock().unwrap().len(), 1);
}

#[test]
fn test_process_runner_basic() {
    #[derive(Clone)]
    struct TwoTimersProcess {
        on_timer_1_cnt: u32,
        on_timer_2_cnt: u32,
        on_start_cnt: u32,
    }

    impl Process for TwoTimersProcess {
        fn on_start(&mut self, ctx: &mut impl Context) -> Result<(), String> {
            self.on_start_cnt += 1;
            ctx.set_timer("timer_1".to_string(), 0.25);
            ctx.set_timer("timer_2".to_string(), 0.15);
            Ok(())
        }
        fn on_timer(&mut self, name: String, ctx: &mut impl Context) -> Result<(), String> {
            if name == "timer_1" {
                self.on_timer_1_cnt += 1;
                if self.on_timer_1_cnt == 2 {
                    ctx.cancel_timer("timer_2".to_string());
                }
                if self.on_timer_1_cnt <= 3 {
                    ctx.set_timer("timer_1".to_string(), 0.25);
                } else {
                    ctx.stop_process(false);
                }
            } else if name == "timer_2" {
                self.on_timer_2_cnt += 1;
                ctx.set_timer("timer_2".to_string(), 0.15);
            }
            Ok(())
        }
        fn on_message(
            &mut self,
            _msg: Message,
            _from: String,
            _ctx: &mut impl Context,
        ) -> Result<(), String> {
            Ok(())
        }
    }

    let mut proc = TwoTimersProcess {
        on_timer_1_cnt: 0,
        on_timer_2_cnt: 0,
        on_start_cnt: 0,
    };

    let config = RunConfig {
        host: "localhost:10099".to_string(),
    };
    let mut process_runner = ProcessRunner::new(config).expect("Can not create process runner");

    let result = process_runner.run(&mut proc);

    assert!(result.is_ok());

    assert_eq!(proc.on_timer_1_cnt, 4);
    assert_eq!(proc.on_timer_2_cnt, 3);
    assert_eq!(proc.on_start_cnt, 1);
}

#[test]
fn test_process_runner_defer_stop() {
    #[derive(Clone)]
    struct OneTimerProcess {
        timer_cnt: u32,
    }

    impl Process for OneTimerProcess {
        fn on_start(&mut self, ctx: &mut impl Context) -> Result<(), String> {
            ctx.set_timer("timer".to_string(), 0.05);
            ctx.stop_process(false);
            Ok(())
        }
        fn on_timer(&mut self, name: String, ctx: &mut impl Context) -> Result<(), String> {
            assert!(name == "timer");
            self.timer_cnt += 1;
            if self.timer_cnt <= 3 {
                ctx.set_timer("timer".to_string(), 0.05);
            }
            Ok(())
        }
        fn on_message(
            &mut self,
            _msg: Message,
            _from: String,
            _ctx: &mut impl Context,
        ) -> Result<(), String> {
            Ok(())
        }
    }

    let mut proc = OneTimerProcess { timer_cnt: 0 };

    let config = RunConfig {
        host: "localhost:10095".to_string(),
    };

    let mut runner = ProcessRunner::new(config).expect("Can not create process runner");
    let result = runner.run(&mut proc);

    assert!(result.is_ok());

    assert_eq!(proc.timer_cnt, 4);
}

#[test]
fn test_process_runner_err_stop() {
    #[derive(Clone)]
    struct OneTimerProcess {
        timer_cnt: u32,
    }

    impl Process for OneTimerProcess {
        fn on_start(&mut self, ctx: &mut impl Context) -> Result<(), String> {
            ctx.set_timer("timer".to_string(), 0.05);
            Ok(())
        }
        fn on_timer(&mut self, name: String, ctx: &mut impl Context) -> Result<(), String> {
            assert!(name == "timer");
            self.timer_cnt += 1;
            ctx.set_timer("timer".to_string(), 0.05);
            if self.timer_cnt <= 3 {
                Ok(())
            } else {
                Err("Error".to_string())
            }
        }
        fn on_message(
            &mut self,
            _msg: Message,
            _from: String,
            _ctx: &mut impl Context,
        ) -> Result<(), String> {
            Ok(())
        }
    }

    let mut proc = OneTimerProcess { timer_cnt: 0 };

    let mut runner = ProcessRunner::new(RunConfig {
        host: "localhost:10093".to_string(),
    })
    .expect("Can not create process runner");
    let result = runner.run(&mut proc);

    assert_eq!(proc.timer_cnt, 4);
    assert_eq!(result, Err("Error".to_string()));
}

#[test]
#[should_panic(expected = "Trying to run ProcessRunner twice")]
fn test_process_runner_runs_once() {
    #[derive(Clone)]
    struct StopOnStartProcess {
        started: bool,
    }
    impl Process for StopOnStartProcess {
        fn on_start(&mut self, ctx: &mut impl Context) -> Result<(), String> {
            self.started = true;
            ctx.stop_process(false);
            Ok(())
        }
        fn on_timer(&mut self, _name: String, _ctx: &mut impl Context) -> Result<(), String> {
            Err("No timers in the test".to_string())
        }
        fn on_message(
            &mut self,
            _msg: Message,
            _from: String,
            _ctx: &mut impl Context,
        ) -> Result<(), String> {
            Ok(())
        }
    }

    let mut proc = StopOnStartProcess { started: false };

    let mut runner = ProcessRunner::new(RunConfig {
        host: "localhost:10089".to_string(),
    })
    .expect("Can not create process runner");
    let result = runner.run(&mut proc);

    assert!(result.is_ok());

    assert!(proc.started);

    // check compiler allows to mutate process after runner stop
    proc.started = false;
    assert_eq!(proc.started, false);

    // must fail on assert
    let _ = runner.run(&mut proc);
}

#[test]
fn test_real_system_runs_process() {
    #[derive(Clone)]
    struct TwoTimersProcess {
        on_timer_1_cnt: u32,
        on_timer_2_cnt: u32,
        on_start_cnt: u32,
    }

    impl Process for TwoTimersProcess {
        fn on_start(&mut self, ctx: &mut impl Context) -> Result<(), String> {
            self.on_start_cnt += 1;
            ctx.set_timer("timer_1".to_string(), 0.25);
            ctx.set_timer("timer_2".to_string(), 0.15);
            Ok(())
        }
        fn on_timer(&mut self, name: String, ctx: &mut impl Context) -> Result<(), String> {
            if name == "timer_1" {
                self.on_timer_1_cnt += 1;
                if self.on_timer_1_cnt == 2 {
                    ctx.cancel_timer("timer_2".to_string());
                }
                if self.on_timer_1_cnt <= 3 {
                    ctx.set_timer("timer_1".to_string(), 0.25);
                } else {
                    ctx.stop_process(false);
                }
            } else if name == "timer_2" {
                self.on_timer_2_cnt += 1;
                ctx.set_timer("timer_2".to_string(), 0.15);
            }
            Ok(())
        }
        fn on_message(
            &mut self,
            _msg: Message,
            _from: String,
            _ctx: &mut impl Context,
        ) -> Result<(), String> {
            Ok(())
        }
    }

    let mut proc = TwoTimersProcess {
        on_timer_1_cnt: 0,
        on_timer_2_cnt: 0,
        on_start_cnt: 0,
    };

    let system = RealSystem::new();
    let result = system.run_process(&mut proc, "localhost:10091");

    assert_eq!(result, Ok(()));

    assert_eq!(proc.on_timer_1_cnt, 4);
    assert_eq!(proc.on_timer_2_cnt, 3);
    assert_eq!(proc.on_start_cnt, 1);

    proc.on_timer_1_cnt = 0;
    proc.on_timer_2_cnt = 0;
    proc.on_start_cnt = 0;

    let result_2 = system.run_process(&mut proc, "localhost:10092");

    assert_eq!(result_2, Ok(()));

    assert_eq!(proc.on_timer_1_cnt, 4);
    assert_eq!(proc.on_timer_2_cnt, 3);
    assert_eq!(proc.on_start_cnt, 1);
}

#[test]
fn test_network_manager() {
    let event_queue = Arc::new(Mutex::new(VecDeque::new()));

    let host_1 = SocketAddr::from_str("127.0.0.1:10110")
        .expect("Can not create SocketAddr from 127.0.0.1:10110")
        .to_string();
    let host_2 = SocketAddr::from_str("127.0.0.1:10111")
        .expect("Can not create SocketAddr from 127.0.0.1:10111")
        .to_string();

    let mut network_manager_1 =
        NetworkManager::new(event_queue.clone(), 100, host_1.clone(), 0.1).unwrap();

    let mut network_manager_2 =
        NetworkManager::new(event_queue.clone(), 100, host_2.clone(), 0.1).unwrap();

    network_manager_1.start_listen().unwrap();
    network_manager_2.start_listen().unwrap();

    let first_msg =
        Message::borrow_new("1", format!("hello from {host_1})")).expect("Can not create message");

    network_manager_1.send_message(host_2.clone(), first_msg.clone());

    let second_msg =
        Message::borrow_new("2", format!("hello from {host_2})")).expect("Can not create message");

    network_manager_2.send_message(host_1.clone(), second_msg.clone());

    thread::sleep(Duration::from_secs_f64(0.2));

    network_manager_1
        .stop_listen()
        .expect("Network manager 1 can not stop listening");
    network_manager_2
        .stop_listen()
        .expect("Network manager 2 can not stop listening");

    assert_eq!(event_queue.lock().unwrap().len(), 2);

    let first_event = event_queue
        .lock()
        .unwrap()
        .pop_front()
        .expect("Data race detected in the test");
    let second_event = event_queue
        .lock()
        .unwrap()
        .pop_front()
        .expect("Data race detected in the test");

    let event_first = Event::MessageReceived {
        msg: first_msg.clone(),
        from: host_1.clone(),
    };
    let event_second = Event::MessageReceived {
        msg: second_msg.clone(),
        from: host_2.clone(),
    };

    assert!(
        (first_event == event_first && second_event == event_second)
            || (first_event == event_second && second_event == event_first)
    );
}

#[test]
fn test_ping_pong_works() {
    println!("Starting test");

    // Process which send pings
    #[derive(Clone)]
    struct PingProcess {
        received_messages: Vec<Message>,
        to_ping: String,
        last_pong: u32,
    }

    impl PingProcess {
        fn send_ping(&mut self, ctx: &mut impl Context) {
            ctx.send_message(
                Message::new("PING", &self.last_pong.to_string()).expect("Can not create"),
                self.to_ping.clone(),
            );
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

        fn on_message(
            &mut self,
            msg: Message,
            _from: String,
            ctx: &mut impl Context,
        ) -> Result<(), String> {
            assert_eq!(msg.get_tip(), "PONG");
            let pong_seq_num = u32::from_str(
                msg.fetch_data::<String>()
                    .expect("Can not fetch data")
                    .as_str(),
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
                    ctx.stop_process(false);
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
        fn on_start(&mut self, ctx: &mut impl Context) -> Result<(), String> {
            ctx.set_timer("PINGS_ENDED".to_string(), 0.2);
            Ok(())
        }

        fn on_timer(&mut self, name: String, ctx: &mut impl Context) -> Result<(), String> {
            assert_eq!(name, "PINGS_ENDED");
            ctx.stop_process(false);
            Ok(())
        }

        fn on_message(
            &mut self,
            msg: Message,
            from: String,
            ctx: &mut impl Context,
        ) -> Result<(), String> {
            assert_eq!(msg.get_tip(), "PING");
            let last_pong_seq_num = u32::from_str(
                msg.fetch_data::<String>()
                    .expect("Can not fetch data")
                    .as_str(),
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

    let host_1 = "localhost:10127".to_string();
    let host_2 = "localhost:10128".to_string();

    let process_1 = Arc::new(Mutex::new(PingProcess {
        received_messages: vec![],
        to_ping: host_2.clone(),
        last_pong: 0,
    }));

    let process_2 = Arc::new(Mutex::new(PongProcess {}));

    let process_1_copy = process_1.clone();
    let first_proc_thread = thread::spawn(move || {
        let mut proc_1_lock = process_1_copy.lock().unwrap();
        let system = RealSystem::new();
        system
            .run_process(proc_1_lock.deref_mut(), host_1.as_str())
            .expect("Can not run process_1");
    });

    let process_2_copy = process_2.clone();
    let second_proc_thread = thread::spawn(move || {
        let mut proc_2_lock = process_2_copy.lock().unwrap();
        let system = RealSystem::new();
        system
            .run_process(proc_2_lock.deref_mut(), host_2.as_str())
            .expect("Can not run process_2");
    });

    first_proc_thread.join().expect("First proc failed to run");
    second_proc_thread
        .join()
        .expect("Second proc failed to run");

    assert_eq!(process_1.lock().unwrap().received_messages.len(), 10);

    let mut i = 0;

    for msg in process_1.lock().unwrap().received_messages.clone() {
        i += 1;
        assert_eq!(msg.get_tip(), "PONG");
        assert_eq!(
            msg.fetch_data::<String>().expect("Can not fetch data"),
            i.to_string()
        );
    }
}
