use std::sync::{Arc, Mutex};
use std::time;

use crate::real::events::Event;
use crate::real::time::time_manager::TimeManager;

use super::basic_timer_setter::BasicTimerSetter;
use super::defs::*;
use super::timer_setter::TimerSetter;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

#[test]
fn test_basic_timer_setter() {
    // Create runtime
    let runtime = Runtime::new().expect("Can not create tokio runtime");

    // Create set timer request
    let request = SetTimerRequest {
        process: "process".to_owned(),
        timer_name: "timer".to_owned(),
        delay: 0.1,
    };

    // Create sender and receiver timer fired events
    let (sender, mut receiver) = mpsc::channel(32);

    // Create timer
    let timer = BasicTimerSetter::set_timer(request.clone(), sender);

    // Spawn timer
    runtime.spawn(timer);

    // Create mutex which will help to check if timer fired
    let received_flag = Arc::new(Mutex::new(bool::from(false)));
    let received_flag_copy = received_flag.clone();

    // Wait for timer fired
    runtime.block_on(async move {
        let received = receiver
            .recv()
            .await
            .expect("Can not received timer fired event");

        match received {
            Event::TimerFired {
                process_name,
                timer_name,
            } => {
                assert_eq!(process_name, request.process);
                assert_eq!(timer_name, request.timer_name);
            }
            Event::MessageReceived {
                msg: _,
                from: _,
                to: _,
            } => {
                panic!("Incorrect event received");
            }
            Event::SystemStarted {} => {
                panic!("Incorrect event received");
            }
        }

        *received_flag_copy.lock().expect("Can not lock mutex") = true;
    });

    // Check timer fired
    assert_eq!(*received_flag.lock().expect("Can not lock mutex"), true);
}

#[test]
fn test_time_manager() {
    // Init timer manager.
    let mut time_manager = TimeManager::<BasicTimerSetter>::default();

    // Create runtime and event channel.
    let runtime = tokio::runtime::Runtime::new().expect("Can not create runtime");
    let (sender, mut receiver) = mpsc::channel(32);

    // Spawn timer in runtime, mark the time.
    let started_time = time::Instant::now();
    runtime.spawn(async move {
        // Set timer and then cancel it by the cancel_all_timers interface.
        // As a result the following timer must not trigger.
        time_manager.set_timer(sender.clone(), "process0", "timer0", 1.0, false);
        time_manager.cancel_all_timers();

        // Set basic timer on 0.1 seconds.
        time_manager.set_timer(sender.clone(), "process1", "timer1", 0.1, false);

        // Try to reset previous timer with not set overwrite.
        // Nothing must happen.
        time_manager.set_timer(sender.clone(), "process1", "timer1", 0.05, false);

        // Set the other one timer on 0.15 seconds.
        time_manager.set_timer(sender.clone(), "process2", "timer2", 0.15, false);

        // Reset this timer with overwrite equals true and new delay equals 0.07.
        // This means timer must be reset.
        time_manager.set_timer(sender.clone(), "process2", "timer2", 0.07, true);

        // As a result the second timer must fire in 0.07 seconds and the first one in 0.1 seconds.

        // Set one more timer and after that cancel it.
        // As a result timer must not trigger.
        time_manager.set_timer(sender.clone(), "process3", "timer3", 1.0, false);
        time_manager.cancel_timer("process3", "timer3");

        // All conditions will be checked in the other one task, on which runtime will be blocked.
    });

    // Flag to check all timers triggered.
    let received_timers = Arc::new(Mutex::new(0u32));
    let received_timers_clone = received_timers.clone();

    // Block until two timers will triger.
    // If works too long, then timer manager logic is incorrect
    runtime.block_on(async move {
        // Get event of the first timer fired, which must happen in 0.07 seconds.
        let event = receiver
            .recv()
            .await
            .expect("Can not receive because no senders");
        match event {
            Event::TimerFired {
                process_name,
                timer_name,
            } => {
                assert_eq!(process_name, "process2");
                assert_eq!(timer_name, "timer2");

                let mut locked = received_timers.lock().expect("Can not lock received flag");
                *locked += 1;
            }
            Event::MessageReceived {
                msg: _,
                from: _,
                to: _,
            } => panic!("Receive incorrect event type"),
            Event::SystemStarted {} => panic!("Received incorrect event"),
        }

        // Check elapsed time which first timer needs to fire.
        let first_timer_elapsed_time = started_time.elapsed().as_secs_f64();
        assert!(first_timer_elapsed_time > 0.065 && first_timer_elapsed_time < 0.09);

        // Get event of the second timer fired, which must happen in 0.1 seconds.
        let event = receiver
            .recv()
            .await
            .expect("Can not receive because no senders");
        match event {
            Event::TimerFired {
                process_name,
                timer_name,
            } => {
                assert_eq!(process_name, "process1");
                assert_eq!(timer_name, "timer1");

                let mut locked = received_timers.lock().expect("Can not lock received flag");
                *locked += 1;
            }
            Event::MessageReceived {
                msg: _,
                from: _,
                to: _,
            } => panic!("Receive incorrect event type"),
            Event::SystemStarted {} => panic!("Received incorrect event"),
        }

        // Check elapsed time which second timer needs to fire.
        let second_timer_elapsed_time = started_time.elapsed().as_secs_f64();
        assert!(second_timer_elapsed_time > 0.09 && second_timer_elapsed_time < 0.12);

        // Check there are no other senders
        let received_option = receiver.recv().await;
        assert!(received_option.is_none());
    });

    // Check that both timers are executed
    assert_eq!(
        *received_timers_clone
            .lock()
            .expect("Can not lock received flag"),
        2
    );

    // Check what execution time is close to 0.1 seconds.
    let elapsed_time = started_time.elapsed().as_secs_f64();
    assert!(elapsed_time > 0.095 && elapsed_time < 0.15);
}
