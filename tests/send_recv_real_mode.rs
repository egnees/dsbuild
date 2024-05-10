use dsbuild::{Address, Context, Message, Process, RealSystem, Tag};
use tokio::sync::oneshot;

struct SendRecvProcess {
    pair: Address,
}

impl Process for SendRecvProcess {
    fn on_local_message(&mut self, msg: Message, ctx: Context) -> Result<(), String> {
        let tag = msg.get_data::<Tag>().unwrap();
        let to = self.pair.clone();
        ctx.clone().spawn(async move {
            let got_msg = ctx
                .send_recv_with_tag(msg.clone(), tag, to, 2.0 * 5.0)
                .await
                .unwrap();
            ctx.send_local(got_msg);
            ctx.stop();
        });
        Ok(())
    }

    fn on_timer(&mut self, _name: String, _ctx: Context) -> Result<(), String> {
        unreachable!()
    }

    fn on_message(&mut self, msg: Message, from: Address, ctx: Context) -> Result<(), String> {
        let tag = msg.get_data::<Tag>().unwrap();
        ctx.clone().spawn(async move {
            ctx.send_with_tag(msg, tag, from, 5.0).await.unwrap();
            ctx.stop();
        });

        Ok(())
    }
}

#[test]
fn send_recv_works() {
    let mut sys = RealSystem::new(1024, "127.0.0.1", 10092, "/tmp/");
    let addr1 = Address::new_ref("127.0.0.1", 10092, "proc1");
    let addr2 = Address::new_ref("127.0.0.1", 10092, "proc2");
    let mut proc1_io = sys.add_process(
        SendRecvProcess {
            pair: addr2.clone(),
        },
        addr1.process_name.clone(),
    );
    sys.add_process(
        SendRecvProcess {
            pair: addr1.clone(),
        },
        addr2.process_name.clone(),
    );
    let (flag_event_sender, flag_event_receiver) = oneshot::channel();
    sys.spawn(async move {
        let sending_msg = Message::new::<Tag>("msg", &15).unwrap();
        proc1_io.sender.send(sending_msg.clone()).await.unwrap();
        let msg = proc1_io.receiver.recv().await.unwrap();
        assert_eq!(msg, sending_msg);

        flag_event_sender.send(true).unwrap();
    });
    sys.run();

    let got_message = flag_event_receiver.blocking_recv().unwrap();
    assert_eq!(got_message, true);
}
