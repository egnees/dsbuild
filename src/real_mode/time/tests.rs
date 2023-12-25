use std::sync::{Mutex, Arc};

use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use super::basic_manager::BasicTimeManager;
use super::defs::*;
use super::manager::TimeManager;

#[test]
fn test_basic_time_manager() {
    // Create runtime
    let runtime = Runtime::new().expect("Can not create tokio runtime");

    // Create set timer request
    let request = SetTimerRequest { process: "process".to_owned(), timer_name: "timer".to_owned(), delay: 0.1 };

    // Create sender and receiver timer fired events
    let (sender, mut receiver) = mpsc::channel(32);

    // Create timer
    let timer = BasicTimeManager::set_timer(request.clone(), sender);

    // Spawn timer
    runtime.spawn(timer);

    // Create mutex which will help to check if timer fired
    let received_flag = Arc::new(Mutex::new(bool::from(false)));
    let received_flag_copy = received_flag.clone();

    // Wait for timer fired
    runtime.block_on(async move {
        let received = receiver.recv().await.expect("Can not received timer fired event");
        assert_eq!(received.process, request.process);
        assert_eq!(received.timer_name, request.timer_name);
        *received_flag_copy.lock().expect("Can not lock mutex") = true;
    });

    // Check timer fired
    assert_eq!(*received_flag.lock().expect("Can not lock mutex"), true);
}