use dsbuild::Address;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct User {
    address: Address,
    name: String,
    password: String,
    chat: Option<String>,
}

impl User {
    pub fn new(address: Address, name: String, password: String) -> Self {
        Self {
            address,
            name,
            password,
            chat: None,
        }
    }

    pub fn address(&self) -> Address {
        self.address.clone()
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn password(&self) -> &str {
        self.password.as_str()
    }

    pub fn connected_chat(&self) -> Option<&str> {
        self.chat.as_ref().map(|s| s.as_str())
    }

    pub fn is_connected_to_chat(&self) -> bool {
        self.chat.is_some()
    }

    pub fn connect_to_chat(&mut self, chat: String) {
        self.chat = Some(chat);
    }

    pub fn disconnect_from_chat(&mut self) {
        self.chat = None;
    }

    pub fn verify(&self, address: &Address, name: &str, password: &str) -> bool {
        self.address == *address && self.name == name && self.password == password
    }
}
