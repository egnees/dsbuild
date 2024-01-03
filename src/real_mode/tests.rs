use std::sync::{Arc, RwLock};

use super::{
    events::Event,
    process_manager::ProcessManager,
    system::{AddressResolvePolicy, System, SystemConfig},
};
use crate::common::{context::Context, message::Message, process::Process};

// PingProcess must send consecutive ping messages to the other one process with specified delay.
// Also, it waits for the consecutive pong answers.
#[derive(Clone)]
struct PingProcess {
    last_pong: u32,
    // delay in seconds
    delay: f64,
    // pings receiver
    partner: String,
    // need pongs count
    need_cnt: u32,
    // enable flag when on start method is called
    is_started: bool,
    // enable flag when process received the last expected pong
    is_stoped: bool,
}

impl PingProcess {
    const ON_PING_ACTIONS: usize = 2;

    pub const ON_START_ACTIONS: usize = Self::ON_PING_ACTIONS;
    pub const ON_TIMER_ACTIONS: usize = Self::ON_PING_ACTIONS;
    pub const ON_MESSAGE_ACTIONS: usize = 0;
    pub const ON_LAST_MESSAGE_ACTIONS: usize = 2;

    pub const PING_TIMER: &'static str = "PING_TIMER";
    pub const PING_TIP: &'static str = "PING";
    pub const PONG_TIP: &'static str = "PONG";

    fn ping(&self, ctx: &mut dyn Context) {
        let msg = Message::borrow_new(Self::PING_TIP, self.last_pong + 1)
            .expect("Can not create ping message");
        ctx.send_message(msg, self.partner.clone());
        ctx.set_timer(Self::PING_TIMER.to_owned(), self.delay);
    }

    pub fn get_last_pong(&self) -> u32 {
        self.last_pong
    }

    pub fn is_started(&self) -> bool {
        self.is_started
    }

    pub fn is_stoped(&self) -> bool {
        self.is_stoped
    }

    pub fn new(delay: f64, partner: String, need_cnt: u32) -> Self {
        assert!(delay > 0.0);
        Self {
            last_pong: 0,
            delay,
            partner,
            need_cnt,
            is_started: false,
            is_stoped: false,
        }
    }
}

impl Process for PingProcess {
    fn on_start(&mut self, ctx: &mut dyn Context) -> Result<(), String> {
        self.is_started = true;
        self.ping(ctx);
        Ok(())
    }

    fn on_timer(&mut self, name: String, ctx: &mut dyn Context) -> Result<(), String> {
        assert_eq!(name, Self::PING_TIMER);
        self.ping(ctx);
        Ok(())
    }

    fn on_message(
        &mut self,
        msg: Message,
        from: String,
        ctx: &mut dyn Context,
    ) -> Result<(), String> {
        assert_eq!(self.partner, from);
        assert_eq!(msg.get_tip(), Self::PONG_TIP);

        let pong_sequence_number = msg.get_data::<u32>().expect("Can not get message data");
        if pong_sequence_number == self.last_pong + 1 {
            self.last_pong += 1;
        }

        if self.last_pong == self.need_cnt {
            ctx.cancel_timer(Self::PING_TIMER.into());
            ctx.stop_process();

            self.is_stoped = true;
        }

        Ok(())
    }
}

// PongProcess waits for consecutive ping messages,
// and answers consecutive pongs messages with specified delay.
#[derive(Clone)]
struct PongProcess {
    // window of inactivity after that pong process will be stopped
    delay: f64,
}

impl PongProcess {
    const TIMER_NAME: &'static str = "PONG_TIMER";

    fn set_timer(&self, ctx: &mut dyn Context) {
        ctx.set_timer(Self::TIMER_NAME.to_owned(), self.delay);
    }

    pub fn new(delay: f64) -> Self {
        Self { delay }
    }
}

impl Process for PongProcess {
    fn on_start(&mut self, ctx: &mut dyn Context) -> Result<(), String> {
        self.set_timer(ctx);
        Ok(())
    }

    fn on_timer(&mut self, name: String, ctx: &mut dyn Context) -> Result<(), String> {
        assert_eq!(name, Self::TIMER_NAME);
        ctx.stop_process();
        Ok(())
    }

    fn on_message(
        &mut self,
        msg: Message,
        from: String,
        ctx: &mut dyn Context,
    ) -> Result<(), String> {
        let requested_pong = msg.get_data::<u32>()?;
        let answer = Message::borrow_new("PONG", requested_pong)?;
        ctx.send_message(answer, from);
        self.set_timer(ctx);
        Ok(())
    }
}

#[derive(Clone)]
struct IsolatedProcess {
    // need timers cnt
    need_cnt: u32,
    // fired timers cnt
    fired_cnt: u32,
    // delay in seconds
    one_timer_delay: f64,
}

impl IsolatedProcess {
    const TIMER_NAME: &'static str = "TIMER";

    fn set_timer(&self, ctx: &mut dyn Context) {
        ctx.set_timer(Self::TIMER_NAME.to_owned(), self.one_timer_delay);
    }

    pub fn new(need_cnt: u32, one_timer_delay: f64) -> Self {
        Self {
            need_cnt,
            fired_cnt: 0,
            one_timer_delay,
        }
    }
}

impl Process for IsolatedProcess {
    fn on_start(&mut self, ctx: &mut dyn Context) -> Result<(), String> {
        self.set_timer(ctx);

        Ok(())
    }

    fn on_timer(&mut self, name: String, ctx: &mut dyn Context) -> Result<(), String> {
        assert_eq!(name, Self::TIMER_NAME);

        self.fired_cnt += 1;
        if self.fired_cnt < self.need_cnt {
            self.set_timer(ctx);
        } else {
            ctx.stop_process();
        }

        Ok(())
    }

    fn on_message(
        &mut self,
        _msg: Message,
        _from: String,
        _ctx: &mut dyn Context,
    ) -> Result<(), String> {
        Err("No messages for isolated process".to_owned())
    }
}

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
        .is_stoped());

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
        .is_stoped());
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
    let resolve_polyicy = AddressResolvePolicy::Manual { trusted: vec![] };
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
    assert_eq!(owned_process.read().fired_cnt, NEED_CNT);
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
    let resolve_polyicy = AddressResolvePolicy::Manual { trusted: vec![] };
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
