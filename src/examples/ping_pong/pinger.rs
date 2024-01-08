//! Assistant for [`PingProcess`] functionality.

use crate::process_lib::ping::PingProcess;

/// Creates new [`PingProcess`] with specified delay,
/// partner process name and need count of pongs to receive before stop.
pub fn create_pinger(delay: f64, partner: String, need_cnt: u32) -> PingProcess {
    PingProcess::new_verbose(delay, partner, need_cnt)
}
