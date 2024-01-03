use super::{defs::Address, resolver::AddressResolver};

use std::collections::HashMap;

#[derive(Default)]
pub struct ManualResolver {
    mapping: HashMap<String, Address>,
}

impl ManualResolver {
    pub fn from_trusted_list(trusted_list: Vec<Address>) -> Result<Self, String> {
        let mut resolver = ManualResolver::default();
        for address in trusted_list {
            resolver.add_record(&address)?
        }

        Ok(resolver)
    }
}

impl AddressResolver for ManualResolver {
    fn resolve(&self, process_name: &str) -> Result<Address, String> {
        self.mapping
            .get(process_name)
            .ok_or("Can resolve address: no such process name in the mapping".to_owned())
            .cloned()
    }

    fn add_record(&mut self, address: &Address) -> Result<(), String> {
        let process_name = &address.process_name;
        if self.mapping.contains_key(process_name) {
            Err(
                "Record with such process name already contains in the address resolver mapping"
                    .to_owned(),
            )
        } else {
            let insert_result = self
                .mapping
                .insert(process_name.to_string(), address.clone());

            if insert_result.is_some() {
                panic!("Imlementation error. Probably data race detected");
            }

            Ok(())
        }
    }
}
