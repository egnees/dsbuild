mod events;
mod process_runner;
mod timer_manager;
mod real_context;
mod network_manager;

pub mod real_system;

#[cfg(test)]
mod tests {
    use std::{sync::{Mutex, Arc}, collections::VecDeque, thread, time::Duration};

    use crate::common::{process::Process, context::Context};

    use super::{timer_manager::TimerManager, events::Event, process_runner::{ProcessRunner, RunConfig}};

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
}