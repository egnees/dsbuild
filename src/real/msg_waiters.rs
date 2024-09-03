//! Definition of message waiters.

use std::collections::HashMap;

use tokio::sync::oneshot::Sender;

use crate::common::message::{Message, Tag};

pub type MessageWaiters = HashMap<Tag, Vec<Sender<Message>>>;
