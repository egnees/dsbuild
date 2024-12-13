use dsbuild::Context;

//////////////////////////////////////////////////////////////////////////////////////////

/// Allows to read last value from file.
/// Panics in case of I/O failure.
/// If no values present in file or file not exists, returns [None].
pub async fn read_last_value<T>(file_path: &'static str, ctx: Context) -> Option<T>
where
    for<'a> T: serde::Deserialize<'a>,
{
    if !ctx.file_exists(file_path).await.unwrap() {
        return None;
    }

    let mut res = Option::default();
    let mut file = ctx.open_file(file_path).await.unwrap();

    let mut offset = 0;
    let mut buffer = [0u8; 4096];
    let mut last_one = Vec::new();

    loop {
        let bytes = file.read(offset, &mut buffer).await.unwrap();
        if bytes == 0 {
            break;
        }
        offset += bytes;
        for c in buffer[..bytes as usize].iter().copied() {
            if c == b'\n' {
                let current_value: T = serde_json::from_slice(last_one.as_slice()).unwrap();
                res.replace(current_value);
                last_one.clear();
            } else {
                last_one.push(c);
            }
        }
    }

    res
}

//////////////////////////////////////////////////////////////////////////////////////////

/// Allows to append value in the file.
/// If file not present, it will be created.
pub async fn append_value<T>(file_path: &'static str, value: T, ctx: Context)
where
    T: serde::Serialize,
{
    let mut file = if ctx.file_exists(file_path).await.unwrap() {
        ctx.open_file(file_path).await.unwrap()
    } else {
        ctx.create_file(file_path).await.unwrap()
    };

    let mut bytes = serde_json::to_vec(&value).unwrap();
    bytes.push(b'\n');

    let mut offset = 0;
    while offset < bytes.len() {
        let bytes_write = file.append(&bytes[offset..]).await.unwrap();
        if bytes_write == 0 {
            panic!("no disk space available")
        }
        offset += bytes_write as usize;
    }
}

//////////////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use std::{fmt::Debug, marker::PhantomData};

    use dsbuild::{Address, Context, Message, Process, Sim};
    use serde::{Deserialize, Serialize};

    use crate::disk::{append_value, read_last_value};

    //////////////////////////////////////////////////////////////////////////////////////////

    const NODE: &str = "node";
    const PROCESS: &str = "process";
    const APPEND: &str = "append";
    const READ: &str = "read";
    const FILE_PATH: &str = "file_path";

    //////////////////////////////////////////////////////////////////////////////////////////

    #[derive(Default)]
    struct Proc<T> {
        _phantom: PhantomData<T>,
    }

    impl<T> Process for Proc<T>
    where
        T: Send + Sync + for<'a> Deserialize<'a> + Serialize + PartialEq + Debug + 'static,
    {
        fn on_local_message(&mut self, msg: Message, ctx: Context) -> Result<(), String> {
            let value = msg.get_data::<T>().unwrap();
            let read_request = msg.get_tip() == READ;
            ctx.clone().spawn(async move {
                if read_request {
                    let read_value = read_last_value::<T>(FILE_PATH, ctx).await.unwrap();
                    assert_eq!(value, read_value);
                } else {
                    append_value(FILE_PATH, value, ctx).await;
                }
            });
            Ok(())
        }

        fn on_timer(&mut self, _name: String, _ctx: Context) -> Result<(), String> {
            unimplemented!()
        }

        fn on_message(
            &mut self,
            _msg: Message,
            _from: Address,
            _ctx: Context,
        ) -> Result<(), String> {
            unimplemented!()
        }
    }

    //////////////////////////////////////////////////////////////////////////////////////////

    fn send_append_request<'a, T: Serialize + Deserialize<'a>>(sim: &mut Sim, x: &'a T) {
        sim.send_local_message(PROCESS, NODE, Message::new(APPEND, &x).unwrap());
    }

    fn send_read_request<'a, T: Serialize + Deserialize<'a>>(sim: &mut Sim, x: &'a T) {
        sim.send_local_message(PROCESS, NODE, Message::new(READ, &x).unwrap());
    }

    fn wait(sim: &mut Sim) {
        sim.step_until_no_events();
    }

    fn make_sim<T>(storage: usize) -> Sim
    where
        T: Send
            + Sync
            + for<'a> Deserialize<'a>
            + Serialize
            + PartialEq
            + Debug
            + Default
            + 'static,
    {
        let mut sim = Sim::new(12345);
        sim.add_node_with_storage(NODE, "localhost", 8080, storage);
        sim.add_process(PROCESS, Proc::<T>::default(), NODE);
        sim
    }

    //////////////////////////////////////////////////////////////////////////////////////////

    #[test]
    fn read_write_i64() {
        let mut sim = make_sim::<i64>(1 << 20);

        // append 5 to file
        send_append_request(&mut sim, &5i64);
        wait(&mut sim);

        // read 5
        send_read_request(&mut sim, &5i64);
        wait(&mut sim);

        // append 10000 to file
        send_append_request(&mut sim, &10000i64);
        wait(&mut sim);

        // read 10000
        send_read_request(&mut sim, &10000i64);
        wait(&mut sim);

        // append 4096 1000 times and 9999 then
        for _ in 0..1000 {
            send_append_request(&mut sim, &4096i64);
        }
        send_append_request(&mut sim, &9999i64);
        wait(&mut sim);

        // read 9999
        send_read_request(&mut sim, &9999i64);
        wait(&mut sim);
    }

    //////////////////////////////////////////////////////////////////////////////////////////

    #[test]
    fn read_write_long_strings() {
        use std::io::Read;

        let mut sim = make_sim::<String>(1 << 20);

        let make_string = |n| {
            let mut s = String::default();
            (0..n)
                .map(|i| b'a' + (i % 26) as u8)
                .collect::<Vec<u8>>()
                .as_slice()
                .read_to_string(&mut s)
                .unwrap();
            s
        };

        let s1 = make_string(100_000);
        send_append_request(&mut sim, &s1);
        wait(&mut sim);

        send_read_request(&mut sim, &s1);
        wait(&mut sim);

        let s2 = make_string(200_000);
        send_append_request(&mut sim, &s2);
        wait(&mut sim);

        send_read_request(&mut sim, &s2);
        wait(&mut sim);

        let s3 = make_string(4);
        send_append_request(&mut sim, &s3);
        wait(&mut sim);

        send_read_request(&mut sim, &s3);
        wait(&mut sim);
    }

    //////////////////////////////////////////////////////////////////////////////////////////

    #[test]
    fn read_write_custom() {
        #[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
        struct X {
            x: Option<usize>,
        }

        #[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
        struct Custom {
            x: i32,
            y: i64,
            z: X,
            val: String,
        }

        let mut sim = make_sim::<Custom>(1 << 10);

        let custom1 = Custom {
            x: 1,
            y: 2,
            z: X { x: Some(3) },
            val: "hello".to_owned(),
        };
        send_append_request(&mut sim, &custom1);
        wait(&mut sim);

        send_read_request(&mut sim, &custom1);
        wait(&mut sim);

        let custom2 = Custom {
            x: -15,
            y: -60000,
            z: X { x: None },
            val: "bye".to_owned(),
        };
        send_append_request(&mut sim, &custom2);
        wait(&mut sim);

        send_read_request(&mut sim, &custom2);
        wait(&mut sim);
    }
}
