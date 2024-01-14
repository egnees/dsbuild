//! Assistant for [`PingProcess`] functionality.

use crate::{common::process::Address, process_lib::ping::PingProcess};

/// Creates new [`PingProcess`] with specified delay,
/// partner process name and need count of pongs to receive before stop.
pub fn create_pinger(delay: f64, partner: Address, need_cnt: u32) -> PingProcess {
    PingProcess::new_verbose(delay, partner, need_cnt)
}
