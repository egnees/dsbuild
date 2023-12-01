mod events;
mod process_runner;
mod timer_manager;
mod real_context;
mod network_manager;

pub mod real_system;

#[cfg(test)]
mod tests {
    use std::{sync::{Mutex, Arc}, collections::VecDeque, thread, time::Duration, net::SocketAddr, str::FromStr};

    use crate::{common::{process::Process, context::Context, message::Message}, real_mode::real_system::RealSystem};

    use super::{timer_manager::TimerManager, events::Event, process_runner::{ProcessRunner, RunConfig}, network_manager::NetworkManager};

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
        assert_eq!(first_event, Event::TimerFired { name: "timer_1".to_string() });

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
        }

        let mut proc = TwoTimersProcess {
            on_timer_1_cnt: 0,
            on_timer_2_cnt: 0,
            on_start_cnt: 0,
        };

        let config = RunConfig {
            host: "localhost:10085".to_string()
        };
        let mut process_runner = ProcessRunner::new(config);

        let result = process_runner.run(&mut proc);
        
        assert!(result.is_ok());
        
        assert_eq!(proc.on_timer_1_cnt, 4);
        assert_eq!(proc.on_timer_2_cnt, 3);
        assert_eq!(proc.on_start_cnt, 1);
    }

    #[test]
    fn test_process_runner_defer_stop() {
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
        }

        let mut proc = OneTimerProcess {
            timer_cnt: 0
        };

        let config = RunConfig {
            host: "localhost:10085".to_string(),
        };

        let mut runner = ProcessRunner::new(config);
        let result = runner.run(&mut proc);

        assert!(result.is_ok());

        assert_eq!(proc.timer_cnt, 4);
    }

    #[test]
    fn test_process_runner_err_stop() {
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
        }

        let mut proc = OneTimerProcess {
            timer_cnt: 0,
        };

        let mut runner = ProcessRunner::new(RunConfig { host: "localhost:10085".to_string() });
        let result = runner.run(&mut proc);
        
        assert_eq!(proc.timer_cnt, 4);
        assert_eq!(result, Err("Error".to_string()));
    }

    #[test]
    #[should_panic(expected = "Trying to run ProcessRunner twice")]
    fn test_process_runner_runs_once() {
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
        }

        let mut proc = StopOnStartProcess {
            started: false,
        };

        let mut runner = ProcessRunner::new(RunConfig { host: "localhost:10085".to_string() });
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
        }

        let mut proc = TwoTimersProcess {
            on_timer_1_cnt: 0,
            on_timer_2_cnt: 0,
            on_start_cnt: 0,
        };

        let system = RealSystem::new();
        let result = system.run_process(&mut proc, "localhost:10085");

        assert_eq!(result, Ok(()));

        assert_eq!(proc.on_timer_1_cnt, 4);
        assert_eq!(proc.on_timer_2_cnt, 3);
        assert_eq!(proc.on_start_cnt, 1);

        proc.on_timer_1_cnt = 0;
        proc.on_timer_2_cnt = 0;
        proc.on_start_cnt = 0;
        
        let result_2 = system.run_process(&mut proc, "localhost:10085");

        assert_eq!(result_2, Ok(()));

        assert_eq!(proc.on_timer_1_cnt, 4);
        assert_eq!(proc.on_timer_2_cnt, 3);
        assert_eq!(proc.on_start_cnt, 1);
    }

    #[test]
    fn test_network_manager() {
        let event_queue = Arc::new(Mutex::new(VecDeque::new()));
        
        let host_1 = SocketAddr::from_str("127.0.0.1:10088")
                .expect("Can not create SocketAddr from 127.0.0.1:10085")
                .to_string();
        let host_2 = SocketAddr::from_str("127.0.0.1:10089")
                .expect("Can not create SocketAddr from 127.0.0.1:10086")
                .to_string();

        let mut network_manager_1 = NetworkManager::new(
            event_queue.clone(),
            100, 
            host_1.clone(), 
            0.1).unwrap();
        
        let mut network_manager_2 = NetworkManager::new(
            event_queue.clone(),
            100,
            host_2.clone(),
            0.1).unwrap();

        network_manager_1.start_listen().unwrap();
        network_manager_2.start_listen().unwrap();

        let first_msg = Message::new("1".to_string(), format!("hello from {host_1})"));

        network_manager_1.send_message(host_2.clone(), first_msg.clone()).unwrap();

        let second_msg = Message::new("2".to_string(), format!("hello from {host_2})"));

        network_manager_2.send_message(host_1.clone(), second_msg.clone()).unwrap();

        thread::sleep(Duration::from_secs_f64(0.2));

        // Double free apears here, need fix
        network_manager_1.stop_listen().expect("Network manager 1 can not stop listening");
        network_manager_2.stop_listen().expect("Network manager 2 can not stop listening");

        assert_eq!(event_queue.lock().unwrap().len(), 2);

        let first_event = event_queue.lock().unwrap().pop_front().expect("Data race detected in the test");
        let second_event = event_queue.lock().unwrap().pop_front().expect("Data race detected in the test");

        let event_first = Event::MessageReceived { msg: first_msg.clone(), from: host_1.clone() };
        let event_second = Event::MessageReceived { msg: second_msg.clone(), from: host_2.clone() };

        assert!((first_event == event_first && second_event == event_second) 
                ||
                (first_event == event_second && second_event == event_first));
    }
}