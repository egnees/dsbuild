use rand::{distributions::Alphanumeric, seq::SliceRandom, Rng};
use rand_pcg::Pcg64;
use rand_seeder::Seeder;
use serde::{Deserialize, Serialize};

use crate::{Address, Context, Message, Process, Sim};

struct StorageProc {}

#[derive(Clone, Serialize, Deserialize)]
struct ReadRequest {
    file: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct AppendRequest {
    file: String,
    data: String,
}

impl Process for StorageProc {
    fn on_local_message(&mut self, msg: Message, ctx: Context) {
        if msg.tip() == "read" {
            ctx.clone().spawn(async move {
                let read_request = msg.data::<ReadRequest>().unwrap();
                let mut offset = 0;
                let mut buf = vec![0u8; 128];
                let buf_slice = buf.as_mut_slice();
                let mut read_result = String::new();
                let mut file = ctx.open_file(&read_request.file).await.unwrap();
                loop {
                    let read_bytes = file.read(offset, buf_slice).await.unwrap();
                    if read_bytes == 0 {
                        break;
                    }
                    offset += read_bytes;
                    read_result
                        .push_str(std::str::from_utf8(&buf_slice[..read_bytes as usize]).unwrap());
                }
                ctx.send_local(Message::new("read_result", &read_result).unwrap());
            });
        } else {
            ctx.clone().spawn(async move {
                let append_request = msg.data::<AppendRequest>().unwrap();
                if !ctx.file_exists(&append_request.file).await.unwrap() {
                    ctx.create_file(&append_request.file).await.unwrap();
                }

                let mut file = ctx.open_file(&append_request.file).await.unwrap();
                let data = append_request.data.as_bytes();
                let mut offset = 0;
                loop {
                    let appended = file.append(&data[offset as usize..]).await.unwrap();
                    if appended == 0 {
                        break;
                    }
                    offset += appended;
                }
                ctx.send_local(Message::new("append_result", &"ok").unwrap());
            });
        }
    }

    fn on_timer(&mut self, _name: String, _ctx: Context) {
        unreachable!()
    }

    fn on_message(&mut self, _msg: Message, _from: Address, _ctx: Context) {
        unreachable!()
    }
}

#[test]
fn storage_works() {
    let mut sys = Sim::new(12345);
    sys.add_node_with_storage("node", "node", 12345, 1 << 20);
    sys.add_process("storage_process", StorageProc {}, "node");

    sys.send_local_message(
        "storage_process",
        "node",
        Message::new(
            "append",
            &AppendRequest {
                file: "file1".to_owned(),
                data: "append1\n".to_owned(),
            },
        )
        .unwrap(),
    );

    sys.step_until_no_events();

    sys.send_local_message(
        "storage_process",
        "node",
        Message::new(
            "read",
            &ReadRequest {
                file: "file1".to_owned(),
            },
        )
        .unwrap(),
    );

    sys.step_until_no_events();

    let messages = sys.read_local_messages("storage_process", "node").unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].tip(), "append_result");
    assert_eq!(messages[1].tip(), "read_result");
    assert_eq!(messages[1].data::<String>().unwrap(), "append1\n");
}

#[test]
fn storage_stress() {
    let mut sys = Sim::new(12345);
    sys.add_node_with_storage("node", "node", 12345, 1 << 20);
    sys.add_process("storage_process", StorageProc {}, "node");

    let mut files = ["file1", "file2", "file3", "file4", "file5"];
    for iter in 0..100 {
        files.shuffle(&mut Seeder::from(iter).make_rng::<Pcg64>());
        for (i, file) in files.iter().enumerate() {
            let content: String = Seeder::from(iter ^ i)
                .make_rng::<Pcg64>()
                .sample_iter(&Alphanumeric)
                .take(1055)
                .map(char::from)
                .collect();
            sys.send_local_message(
                "storage_process",
                "node",
                Message::new(
                    "append",
                    &AppendRequest {
                        file: file.to_string(),
                        data: content,
                    },
                )
                .unwrap(),
            );
        }
    }

    sys.step_until_no_events();

    for file in files.iter() {
        sys.send_local_message(
            "storage_process",
            "node",
            Message::new(
                "read",
                &ReadRequest {
                    file: file.to_string(),
                },
            )
            .unwrap(),
        );
    }

    sys.step_until_no_events();

    let messages = sys.read_local_messages("storage_process", "node").unwrap();
    for message in messages {
        if message.tip() == "read_result" {
            assert_eq!(message.data::<String>().unwrap().len(), 1055 * 100);
        }
    }
}
