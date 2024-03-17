use log::info;

use crate::{common::context::Context, Address, Message, Process};

use super::system;

#[derive(Clone)]
struct Proc {}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct LocalMessage {
    info: String,
    other: Address,
}

impl Process for Proc {
    fn on_local_message(&mut self, msg: Message, ctx: Context) -> Result<(), String> {
        info!(
            "Process got local message {:?}",
            msg.get_data::<LocalMessage>().unwrap()
        );

        let other = msg.get_data::<LocalMessage>().unwrap().other.clone();

        ctx.send_local(msg.clone());
        ctx.send(msg, other);

        Ok(())
    }

    fn on_timer(&mut self, name: String, ctx: Context) -> Result<(), String> {
        todo!()
    }

    fn on_message(&mut self, msg: Message, from: Address, ctx: Context) -> Result<(), String> {
        info!(
            "Process got network message {:?}",
            msg.get_data::<LocalMessage>().unwrap()
        );

        Ok(())
    }
}

#[test]
fn works() {
    // Init logging.
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .init();

    let second_addr = Address {
        host: "127.0.0.1".to_owned(),
        port: 11123,
        process_name: "proc2".to_owned(),
    };

    let mut system = system::System::new(1024, "127.0.0.1", 11123);

    let proc1 = Proc {};
    let wrapper1 = system.add_process(proc1, "proc1".to_owned());

    let proc2 = Proc {};
    system.add_process(proc2, "proc2".to_owned());

    let sender = wrapper1.sender;
    let mut receiver = wrapper1.receiver;

    system.spawn(Box::pin(async move {
        sender
            .send(
                Message::new(
                    "local",
                    &LocalMessage {
                        info: "Hello".to_owned(),
                        other: second_addr.clone(),
                    },
                )
                .unwrap(),
            )
            .await
            .unwrap();
    }));

    system.spawn(Box::pin(async move {
        let msg = receiver.recv().await.unwrap();
        info!(
            "User got local message {:?}",
            msg.get_data::<LocalMessage>().unwrap()
        );
    }));

    system.run();
}
