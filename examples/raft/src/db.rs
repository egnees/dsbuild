use std::collections::{hash_map::Entry, HashMap};

use crate::cmd::{
    Command, CommandReply, CommandType, KeyType, ValueType, ALREADY_EXISTS_CODE, CREATED_CODE,
    DELETED_CODE, NOT_FOUND_CODE, NOT_UPDATED_CODE, UPDATED_CODE,
};

//////////////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct DataBase {
    data: HashMap<KeyType, ValueType>,
}

impl DataBase {
    pub fn apply_command(&mut self, cmd: Command) -> CommandReply {
        match cmd.tp {
            CommandType::Create(key) => {
                if let Entry::Vacant(e) = self.data.entry(key) {
                    e.insert(Default::default());
                    CommandReply::new(CREATED_CODE, "created", cmd.id)
                } else {
                    CommandReply::new(
                        ALREADY_EXISTS_CODE, // http code for conflict
                        "already exists",
                        cmd.id,
                    )
                }
            }
            CommandType::Update(key, value) => {
                if let Entry::Occupied(mut e) = self.data.entry(key.clone()) {
                    e.insert(value.clone());
                    CommandReply::new(UPDATED_CODE, "updated", cmd.id)
                } else {
                    CommandReply::new(NOT_FOUND_CODE, "not found", cmd.id)
                }
            }
            CommandType::Delete(key) => {
                let delete_result = self.data.remove(&key);
                if delete_result.is_some() {
                    CommandReply::new(DELETED_CODE, "deleted", cmd.id)
                } else {
                    CommandReply::new(NOT_FOUND_CODE, "not found", cmd.id)
                }
            }
            CommandType::Cas(key, cmp, new) => {
                let value = self.data.get(&key);
                if value.is_none() {
                    return CommandReply::new(NOT_FOUND_CODE, "not found", cmd.id);
                }
                let value = value.unwrap().clone();
                if value == cmp {
                    self.data.insert(key, new);
                    CommandReply::new(UPDATED_CODE, "updated", cmd.id)
                } else {
                    CommandReply::new(NOT_UPDATED_CODE, "not updated", cmd.id)
                }
            }
        }
    }

    pub fn read_value(&self, key: &KeyType) -> Option<ValueType> {
        self.data.get(key).cloned()
    }
}

//////////////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use crate::cmd::{
        Command, CommandId, CommandType, ALREADY_EXISTS_CODE, CREATED_CODE, DELETED_CODE,
        NOT_FOUND_CODE, NOT_UPDATED_CODE, UPDATED_CODE,
    };

    use super::DataBase;

    #[test]
    fn basic() {
        let mut db = DataBase::default();

        let rep1 = db.apply_command(Command::new(
            CommandType::Create("k1".to_owned()),
            CommandId(0, 0),
        ));
        assert_eq!(rep1.status, CREATED_CODE);

        let rep2 = db.apply_command(Command::new(
            CommandType::Update("k1".to_owned(), "v1".to_owned()),
            CommandId(0, 1),
        ));
        assert_eq!(rep2.status, UPDATED_CODE);

        let res1 = db.read_value(&"k1".to_owned());
        assert!(res1.is_some());
        assert_eq!(res1.unwrap(), "v1");

        let rep3 = db.apply_command(Command::new(
            CommandType::Delete("k1".to_owned()),
            CommandId(0, 2),
        ));
        assert_eq!(rep3.status, DELETED_CODE);

        let res2 = db.read_value(&"k1".to_owned());
        assert!(res2.is_none());

        let rep4 = db.apply_command(Command::new(
            CommandType::Delete("k1".to_owned()),
            CommandId(0, 3),
        ));
        assert_eq!(rep4.status, NOT_FOUND_CODE);
    }

    //////////////////////////////////////////////////////////////////////////////////////////

    #[test]
    fn creates_empty() {
        let mut db = DataBase::default();

        let rep = db.apply_command(Command::new(
            CommandType::Create("k1".to_owned()),
            CommandId(0, 0),
        ));
        assert_eq!(rep.status, CREATED_CODE);

        let res = db.read_value(&"k1".to_owned());
        assert!(res.is_some());
        assert_eq!(res.unwrap(), "");
    }

    //////////////////////////////////////////////////////////////////////////////////////////

    #[test]
    fn create_twice() {
        let mut db = DataBase::default();

        let rep1 = db.apply_command(Command::new(
            CommandType::Create("k1".to_owned()),
            CommandId(0, 0),
        ));
        assert_eq!(rep1.status, CREATED_CODE);

        let rep2 = db.apply_command(Command::new(
            CommandType::Create("k1".to_owned()),
            CommandId(0, 1),
        ));
        assert_eq!(rep2.status, ALREADY_EXISTS_CODE);
    }

    //////////////////////////////////////////////////////////////////////////////////////////

    #[test]
    fn update_works() {
        let mut db = DataBase::default();

        let rep1 = db.apply_command(Command::new(
            CommandType::Create("k1".to_owned()),
            CommandId(0, 0),
        ));
        assert_eq!(rep1.status, CREATED_CODE);

        let rep2 = db.apply_command(Command::new(
            CommandType::Update("k1".to_owned(), "v1".to_owned()),
            CommandId(0, 1),
        ));
        assert_eq!(rep2.status, UPDATED_CODE);

        let res1 = db.read_value(&"k1".to_owned());
        assert_eq!(res1, Some("v1".to_owned()));

        let rep3 = db.apply_command(Command::new(
            CommandType::Update("k1".to_owned(), "v2".to_owned()),
            CommandId(0, 2),
        ));
        assert_eq!(rep3.status, UPDATED_CODE);

        let res2 = db.read_value(&"k1".to_owned());
        assert_eq!(res2, Some("v2".to_owned()));
    }

    //////////////////////////////////////////////////////////////////////////////////////////

    #[test]
    fn cas_works() {
        let mut db = DataBase::default();

        let rep1 = db.apply_command(Command::new(
            CommandType::Create("k1".to_owned()),
            CommandId(0, 0),
        ));
        assert_eq!(rep1.status, CREATED_CODE);

        let rep2 = db.apply_command(Command::new(
            CommandType::Update("k1".to_owned(), "v1".to_owned()),
            CommandId(0, 1),
        ));
        assert_eq!(rep2.status, UPDATED_CODE);

        let res1 = db.read_value(&"k1".to_owned());
        assert_eq!(res1, Some("v1".to_owned()));

        let rep3 = db.apply_command(Command::new(
            CommandType::Cas("k1".to_owned(), "v1".to_owned(), "v2".to_owned()),
            CommandId(0, 2),
        ));
        assert_eq!(rep3.status, UPDATED_CODE);

        let rep4 = db.apply_command(Command::new(
            CommandType::Cas("k1".to_owned(), "v1".to_owned(), "v3".to_owned()),
            CommandId(0, 3),
        ));
        assert_eq!(rep4.status, NOT_UPDATED_CODE);

        let res = db.read_value(&"k1".to_owned());
        assert_eq!(res, Some("v2".to_owned()));
    }
}
