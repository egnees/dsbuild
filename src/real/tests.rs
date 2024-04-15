use std::time::{Duration, SystemTime};

use tokio::{sync::mpsc, time::sleep};

use crate::{
    common::context::Context, real::timer::TimerManager, Address, Message, Process, RealSystem,
};

#[derive(Clone)]
struct LocalProcess {}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct LocalMessage {
    info: String,
    other: Address,
}

impl Process for LocalProcess {
    fn on_local_message(&mut self, msg: Message, ctx: Context) -> Result<(), String> {
        let other = msg.get_data::<LocalMessage>().unwrap().other.clone();
        ctx.send(msg, other);
        Ok(())
    }

    fn on_timer(&mut self, _: String, _: Context) -> Result<(), String> {
        Err("no timers should be".into())
    }

    fn on_message(&mut self, msg: Message, _: Address, ctx: Context) -> Result<(), String> {
        ctx.send_local(msg.clone());
        ctx.stop();
        Ok(())
    }
}

#[test]
fn local_messages_works() {
    let first_addr = Address {
        host: "127.0.0.1".to_owned(),
        port: 11123,
        process_name: "proc1".to_owned(),
    };

    let second_addr = Address {
        host: "127.0.0.1".to_owned(),
        port: 11123,
        process_name: "proc2".to_owned(),
    };

    let mut system = RealSystem::new(1024, "127.0.0.1", 11123, "storage_mount".into());

    let proc1 = LocalProcess {};
    let mut wrapper1 = system.add_process(proc1, "proc1".to_owned());

    let proc2 = LocalProcess {};
    let mut wrapper2 = system.add_process(proc2, "proc2".to_owned());

    // Spawn local messages handler for the first process.
    system.spawn(async move {
        wrapper1
            .sender
            .send(
                Message::borrow_new(
                    "",
                    LocalMessage {
                        info: "message from the first process".to_owned(),
                        other: second_addr,
                    },
                )
                .unwrap(),
            )
            .await
            .unwrap();

        let msg = wrapper1.receiver.recv().await.unwrap();
        assert_eq!(
            msg.get_data::<LocalMessage>().unwrap().info,
            "message from the second process"
        );
    });

    // Spawn local messages handler for the second process.
    system.spawn(async move {
        wrapper2
            .sender
            .send(
                Message::borrow_new(
                    "",
                    LocalMessage {
                        info: "message from the second process".to_owned(),
                        other: first_addr,
                    },
                )
                .unwrap(),
            )
            .await
            .unwrap();

        let msg = wrapper2.receiver.recv().await.unwrap();
        assert_eq!(
            msg.get_data::<LocalMessage>().unwrap().info,
            "message from the first process"
        );
    });

    system.run();
}

#[tokio::test]
async fn timer_manager_set_timer_works() {
    let (sender, mut receiver) = mpsc::channel(100);

    let mut manager = TimerManager::new(sender);

    let time1 = SystemTime::now();

    // Set timer.
    manager.set_timer("timer1".to_owned(), 0.30, false);

    sleep(Duration::from_millis(100)).await;

    // Overwrite.
    manager.set_timer("timer1".to_owned(), 0.15, true);

    // No overwrite.
    manager.set_timer("timer1".to_owned(), 0.10, false);

    let timer_name = receiver.recv().await.unwrap();
    assert_eq!(timer_name, "timer1");

    let elapsed = SystemTime::now().duration_since(time1).unwrap().as_millis();
    assert!(220 < elapsed && elapsed < 270);
}

#[tokio::test]
async fn timer_manager_cancel_works() {
    let (sender, mut receiver) = mpsc::channel(100);

    let mut manager = TimerManager::new(sender);

    // Check 'set_timer' works.
    let time1 = SystemTime::now();

    manager.set_timer("timer1".to_owned(), 0.10, false);
    manager.set_timer("timer2".to_owned(), 0.20, false);

    sleep(Duration::from_millis(50)).await;

    manager.cancel_timer("timer1");

    let timer_name = receiver.recv().await.unwrap();
    assert_eq!(timer_name, "timer2");

    let elapsed = SystemTime::now().duration_since(time1).unwrap().as_millis();
    assert!(180 < elapsed && elapsed < 220);

    manager.set_timer("timer1".to_owned(), 0.10, false);
    manager.set_timer("timer2".to_owned(), 0.20, false);
    manager.set_timer("timer3".to_owned(), 0.10, false);
    manager.set_timer("timer4".to_owned(), 0.20, false);

    sleep(Duration::from_millis(50)).await;

    manager.cancel_all_timers();

    tokio::select! {
        Some(_) = receiver.recv() => panic!("all timers must be cancelled"),
        _ = sleep(Duration::from_millis(200)) => {}
        else => panic!("sleep must be called")
    }
}
