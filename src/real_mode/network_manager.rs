use std::{
    collections::VecDeque,
    net::UdpSocket,
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::common::message::Message;

use crate::real_mode::events::Event;

pub struct NetworkManager {
    event_queue: Arc<Mutex<VecDeque<Event>>>,
    max_message_size: usize,
    socket: UdpSocket,
    listen_stopped: Arc<Mutex<bool>>,
    listen_thread_join_handler: Option<JoinHandle<()>>,
}

impl NetworkManager {
    pub fn new(
        event_queue: Arc<Mutex<VecDeque<Event>>>,
        max_message_size: usize,
        host: String,
        timeout: f64,
    ) -> Result<Self, String> {
        let socket = UdpSocket::bind(host.clone())
            .map_err(|_err| format!("Can not bind UDP socket to address {}", host))?;

        let set_read_timeout_result =
            socket.set_read_timeout(Some(Duration::from_secs_f64(timeout)));
        assert!(
            set_read_timeout_result.is_ok(),
            "{}",
            format!("Can not set read timeout {}", timeout)
        );

        let set_write_timeout_result =
            socket.set_write_timeout(Some(Duration::from_secs_f64(timeout)));
        assert!(
            set_write_timeout_result.is_ok(),
            "{}",
            format!("Can not set write timeout {}", timeout)
        );

        let ret = Self {
            event_queue,
            max_message_size,
            socket,
            listen_stopped: Arc::new(Mutex::new(false)),
            listen_thread_join_handler: None,
        };

        Ok(ret)
    }

    pub fn send_message(&mut self, to: String, msg: Message) {
        let data = msg.clone().tip + ";" + msg.data.as_str();
        let raw_data = data.as_bytes();
        let _ = self.socket.send_to(raw_data, to.clone());
    }

    pub fn start_listen(&mut self) -> Result<(), String> {
        let socket_clone = self
            .socket
            .try_clone()
            .map_err(|_err| "Can not clone socket")?;

        let msg_size = self.max_message_size;

        let queue_copy = self.event_queue.clone();

        let listen_stopped_copy = self.listen_stopped.clone();

        let handler = thread::spawn(move || {
            let mut vec = vec![0u8; msg_size];
            let buf = vec.as_mut_slice();

            loop {
                let recv_result = socket_clone.recv_from(buf);

                if let Ok((received_size, from)) = recv_result {
                    let recv_str_result = String::from_utf8(buf[..received_size].to_vec());

                    if recv_str_result.is_err() {
                        continue;
                    }

                    let recv_str = recv_str_result.unwrap();

                    let split_vec: Vec<&str> = recv_str.split(";").collect();
                    assert!(
                        split_vec.len() == 2,
                        "{}",
                        format!("Bad message data: {}", recv_str)
                    );

                    let tip = split_vec[0].to_string();
                    let data = split_vec[1].to_string();

                    let msg = Message::new(tip, data);

                    let event = Event::MessageReceived {
                        msg,
                        from: from.to_string(),
                    };

                    queue_copy.lock().unwrap().push_back(event);
                }

                let is_listen_stopped = *(listen_stopped_copy.lock().unwrap());
                if is_listen_stopped {
                    break;
                }
            }
        });

        self.listen_thread_join_handler = Some(handler);

        Ok(())
    }

    pub fn stop_listen(&mut self) -> Result<(), String> {
        *(self.listen_stopped.lock().unwrap()) = true;

        let join_handler_opt = std::mem::replace(&mut self.listen_thread_join_handler, None);
        if let Some(join_handler) = join_handler_opt {
            join_handler
                .join()
                .map_err(|_e| "Can not join listen thread")?;
        }

        Ok(())
    }
}
