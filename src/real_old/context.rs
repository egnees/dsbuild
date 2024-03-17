//! Definition of [`RealContext`].

use crate::common::process::Address;

#[derive(Clone)]
pub struct RealContext {
    process_address: Address,
}
