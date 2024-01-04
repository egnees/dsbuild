use std::sync::{Arc, RwLock};

use super::{
    events::Event,
    process_manager::ProcessManager,
    system::{AddressResolvePolicy, System, SystemConfig},
};
use crate::common::message::Message;

use crate::process_lib::{isolated::IsolatedProcess, ping::PingProcess, pong::PongProcess};

#[test]
fn test_process_manager() {
    // Create manager
    let mut manager = ProcessManager::default();

    // Create two ping proc
    const PONG_NAME: &str = "pong_process";

    let proc_1 = PingProcess::new(0.5, PONG_NAME.into(), 1);
    let proc_2 = PingProcess::new(0.5, PONG_NAME.into(), 2);

    // Create wrappers for them
    let proc_1_wrapper = Arc::new(RwLock::new(proc_1));
    let proc_2_wrapper = Arc::new(RwLock::new(proc_2));

    // Add them to manager and check it is not allowed to add two process with equal names
    const FIRST_PING_NAME: &str = "proc_1";
    const SECOND_PING_NAME: &str = "proc_2";

    manager
        .add_process(FIRST_PING_NAME.into(), proc_1_wrapper.clone())
        .expect("Can not add process");
    manager
        .add_process(FIRST_PING_NAME.into(), proc_2_wrapper.clone())
        .expect_err("Manager allows to add process with eqaul name twice");
    manager
        .add_process(SECOND_PING_NAME.into(), proc_2_wrapper.clone())
        .expect("Can not add process with unique name");

    // Check if manager can handle system started events
    let system_started_event = Event::SystemStarted {};
    let actions = manager
        .handle_event(system_started_event)
        .expect("Can not handle system started event");

    // Check on_start method of both proc was called
    // and all process actions were returned
    assert!(proc_1_wrapper
        .read()
        .expect("Can not read first process")
        .is_started());
    assert!(proc_2_wrapper
        .read()
        .expect("Can not read second process")
        .is_started());
    assert_eq!(actions.len(), PingProcess::ON_START_ACTIONS * 2);

    // Check if manager can handle timer fired event
    let first_timer_fired = Event::TimerFired {
        process_name: FIRST_PING_NAME.into(),
        timer_name: PingProcess::PING_TIMER.into(),
    };
    let actions = manager
        .handle_event(first_timer_fired)
        .expect("Can not handle first process timer fired event");

    // Check what all actions associated with fired timer is returned
    assert_eq!(actions.len(), PingProcess::ON_TIMER_ACTIONS);

    // Check if manager can handle message received event
    let pong_message =
        Message::borrow_new(PingProcess::PONG_TIP, 1u32).expect("Can not create message");

    // First, check what message to unknown process is delivered to nobody
    let message_to_unknown_event = Event::MessageReceived {
        msg: pong_message.clone(),
        from: PONG_NAME.into(),
        to: "unknown".into(),
    };
    manager
        .handle_event(message_to_unknown_event)
        .expect_err("Process manager allows to deliver messages with unknown receiver");

    // Check that both process received no pongs
    assert_eq!(
        proc_1_wrapper
            .read()
            .expect("Can not read the first process")
            .get_last_pong(),
        0
    );
    assert_eq!(
        proc_2_wrapper
            .read()
            .expect("Can not read the second process")
            .get_last_pong(),
        0
    );

    // Create event associated with message to the first ping process
    let message_to_first_event = Event::MessageReceived {
        msg: pong_message.clone(),
        from: PONG_NAME.into(),
        to: FIRST_PING_NAME.into(),
    };
    let actions = manager
        .handle_event(message_to_first_event)
        .expect("Can not handle event associated with message to first process");

    // Check that the first ping process received pong message
    // As it expected the only one message, it must be the last one
    assert_eq!(actions.len(), PingProcess::ON_LAST_MESSAGE_ACTIONS);
    assert_eq!(
        proc_1_wrapper
            .read()
            .expect("Can not read the first process")
            .get_last_pong(),
        1
    );
    assert!(proc_1_wrapper
        .read()
        .expect("Can not read the first process")
        .is_stopped());

    // Check that the second ping process did not receive pong message
    assert_eq!(
        proc_2_wrapper
            .read()
            .expect("Can not read the second process")
            .get_last_pong(),
        0
    );

    // Create event associated with message to the second ping process
    let message_to_second_event = Event::MessageReceived {
        msg: pong_message.clone(),
        from: PONG_NAME.into(),
        to: SECOND_PING_NAME.into(),
    };

    // Check that the second ping process received pong message
    // As it expected two messages, it must not be stopped after that
    let actions = manager
        .handle_event(message_to_second_event.clone())
        .expect("Can not handle event associated with message to second process");
    assert_eq!(actions.len(), PingProcess::ON_MESSAGE_ACTIONS);
    assert_eq!(
        proc_2_wrapper
            .read()
            .expect("Can not read the second process")
            .get_last_pong(),
        1
    );

    // Create second pong message
    let pong_message =
        Message::borrow_new(PingProcess::PONG_TIP, 2u32).expect("Can not create message");
    let message_to_second_event = Event::MessageReceived {
        msg: pong_message.clone(),
        from: PONG_NAME.into(),
        to: SECOND_PING_NAME.into(),
    };

    // Send the second pong message to the second ping process
    // It must be the last message, expected by the second process, so process must be stopped after that
    let actions = manager
        .handle_event(message_to_second_event.clone())
        .expect("Can not handle event associated with message to second process");
    assert_eq!(actions.len(), PingProcess::ON_LAST_MESSAGE_ACTIONS);
    assert_eq!(
        proc_2_wrapper
            .read()
            .expect("Can not read the second process")
            .get_last_pong(),
        2
    );
    assert!(proc_2_wrapper
        .read()
        .expect("Can not read the second process")
        .is_stopped());
}

#[test]
fn test_process_manager_process_state() {
    // Create process manager.
    let mut manager = ProcessManager::default();

    // Create two ping processes.
    const FIRST_NAME: &str = "proc_1";
    const SECOND_NAME: &str = "proc_2";

    let proc_1 = PingProcess::new(0.5, FIRST_NAME.into(), 1);
    let proc_2 = PingProcess::new(0.5, SECOND_NAME.into(), 2);

    // Add the first one to manager.
    manager
        .add_process(FIRST_NAME.into(), Arc::new(RwLock::new(proc_1)))
        .expect("Can not add first process");

    // Check there are no active or listening processes.
    assert_eq!(manager.active_count(), 0);

    // Add the second one to manager.
    manager
        .add_process(SECOND_NAME.into(), Arc::new(RwLock::new(proc_2)))
        .expect("Can not add second process");

    // Check there are no active or listening processes.
    assert_eq!(manager.active_count(), 0);

    // Send system started event.
    let system_started_event = Event::SystemStarted {};
    manager
        .handle_event(system_started_event)
        .expect("Can not handle system started event");

    // Check there are two active processes.
    assert_eq!(manager.active_count(), 2);

    // Stop the first process.
    manager
        .stop_process(FIRST_NAME)
        .expect("Can not stop first process");

    // Check there is one active.
    assert_eq!(manager.active_count(), 1);

    // Stop the second process.
    manager
        .stop_process(SECOND_NAME)
        .expect("Can not stop second process");

    // Check there are no active processes.
    assert_eq!(manager.active_count(), 0);
}

#[test]
fn test_system_basic() {
    // Set need cnt for the test.
    const NEED_CNT: u32 = 2;

    // Create system.
    let resolve_polyicy = AddressResolvePolicy::Manual {
        resolve_list: vec![],
    };
    let config = SystemConfig::default(resolve_polyicy, "127.0.0.1".to_owned(), 10035)
        .expect("Can not create default config");
    let mut system = System::new(config).expect("Can not create system");

    // Add process to system.
    let isolated = IsolatedProcess::new(NEED_CNT, 0.1);
    let owned_process = system
        .add_process("isolated_process", isolated)
        .expect("Can not add process");

    // Run system.
    system.run().expect("Can not run system");

    // Check both timers fired.
    assert_eq!(owned_process.read().get_fired_cnt(), NEED_CNT);
}

#[test]
fn test_communication_inside_system() {
    // Define processes.
    const FIRST_PING_NAME: &str = "PING1";
    const SECOND_PING_NAME: &str = "PING2";
    const THIRD_PING_NAME: &str = "PING3";
    const PONG_NAME: &str = "PONG";

    const FIRST_NEED: u32 = 5;
    const SECOND_NEED: u32 = 3;
    const THIRD_NEED: u32 = 6;

    const COMMON_DELAY: f64 = 0.1;
    const PONG_DELAY: f64 = 0.4;

    // Create system.
    let resolve_polyicy = AddressResolvePolicy::Manual {
        resolve_list: vec![],
    };
    let config = SystemConfig::default(resolve_polyicy, "127.0.0.1".to_owned(), 59936)
        .expect("Can not create default config");
    let mut system = System::new(config).expect("Can not create system");

    // Add processes to system.
    let first_ping = system
        .add_process(
            FIRST_PING_NAME.into(),
            PingProcess::new(COMMON_DELAY, PONG_NAME.into(), FIRST_NEED),
        )
        .expect("Can not add the first ping process");

    let second_ping = system
        .add_process(
            SECOND_PING_NAME.into(),
            PingProcess::new(COMMON_DELAY, PONG_NAME.into(), SECOND_NEED),
        )
        .expect("Can not add the second ping process");

    let third_ping = system
        .add_process(
            THIRD_PING_NAME.into(),
            PingProcess::new(COMMON_DELAY, PONG_NAME.into(), THIRD_NEED),
        )
        .expect("Can not add the third ping process");

    system
        .add_process(PONG_NAME, PongProcess::new(PONG_DELAY))
        .expect("Can not add the pong process");

    // Run system.
    system.run().expect("System runned with error");

    // Check that all pings received pongs.
    assert_eq!(first_ping.read().get_last_pong(), FIRST_NEED);
    assert_eq!(second_ping.read().get_last_pong(), SECOND_NEED);
    assert_eq!(third_ping.read().get_last_pong(), THIRD_NEED);
}
