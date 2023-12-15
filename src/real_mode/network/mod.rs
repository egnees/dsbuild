mod grpc_manager;
mod manager;

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        sync::{Arc, Mutex},
    };

    use crate::common::message::Message;

    use super::{
        grpc_manager::GRpcNetworkManager,
        manager::{Address, NetworkManagerTrait},
    };

    #[test]
    fn test_grpc_manager() {
        let mut manager1 = GRpcNetworkManager::new(
            "[::1]:50051".to_string(),
            Arc::new(Mutex::new(VecDeque::new())),
        );
        manager1
            .start_listen()
            .expect("Manager1 can not start listen");

        let mut manager2 = GRpcNetworkManager::new(
            "[::1]:50052".to_string(),
            Arc::new(Mutex::new(VecDeque::new())),
        );
        manager2
            .start_listen()
            .expect("Manager2 can not start listen");

        let msg = Message::borrow_new("tip", "data".to_string()).expect("Can not create message");
        let address = Address {
            host: "localhost:8000".to_owned(),
            process_name: "listener".to_owned(),
        };

        manager1
            .send_message("process_name", &msg, &address)
            .expect("Can not send message");
    }
}
