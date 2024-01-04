//! Definition of the [`AddressResolver`] which is responsible for resolving [network addresses][`Address`]
//! by process name.

use super::defs::*;

pub trait AddressResolver {
    /// Resolves [network address][`Address`] by specified `process_name`.
    /// 
    /// # Returns
    /// 
    /// - [`Err`] if there is no process with specified `process_name`
    /// - [`Ok`] with resolved [address][`Address`] else
    fn resolve(&self, process_name: &str) -> Result<Address, String>;

    /// Adds record of [network address][`Address`] to the resolver.
    /// 
    /// # Returns
    /// 
    /// - [`Err`] if case of record with such [process name][`Address::process_name`] is already present in the resolver
    /// - [`Ok`] with empty content else
    fn add_record(&mut self, record: &Address) -> Result<(), String>;
}
