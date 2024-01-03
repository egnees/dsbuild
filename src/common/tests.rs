use serde::{Deserialize, Serialize};

use super::message::Message;

#[test]
pub fn test_message_basic() {
    let message_data = "hello".to_string();

    let message = Message::new("message_type", &message_data).expect("Can not create message");

    assert_eq!(message.get_tip(), "message_type");

    let deserialized_data = message
        .get_data::<String>()
        .expect("Can not extract data from message");
    assert_eq!(deserialized_data, message_data);

    let message_1 = Message::borrow_new("message_type_1", format!("format_str_{}", 1))
        .expect("Can not create new message");
    assert_eq!(
        message_1.get_data::<String>().expect("Can not fetch data"),
        "format_str_1"
    );
}

#[test]
pub fn test_data_outlives_message() {
    let message_data = "data".to_string();

    let deserialized_data: String;

    {
        let message = Message::new("type", &message_data).expect("Can not create message");
        deserialized_data = message.get_data::<String>().expect("Can not extract data");
    }

    assert_eq!(deserialized_data, message_data);
}

#[test]
pub fn test_serialize_works() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct InnerDataType {
        s1: String,
        s2: String,
        z: u8,
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct DataType {
        x: i32,
        y: u32,
        inner: InnerDataType,
    }

    let data = DataType {
        x: 5,
        y: 6,
        inner: InnerDataType {
            s1: "string_1".to_string(),
            s2: "string_2".to_string(),
            z: 10,
        },
    };

    let message = Message::new("tip", &data).expect("Can not create message");

    assert_eq!(message.get_tip(), "tip");

    let fetched_data = message.get_data::<DataType>().expect("Can not fetch data");

    assert_eq!(fetched_data, data);
}
