use super::defs::*;

pub trait AddressResolver {
    fn resolve(&self, process_name: &str) -> Result<Address, String>;

    fn add_record(&mut self, record: &Address) -> Result<(), String>;
}
