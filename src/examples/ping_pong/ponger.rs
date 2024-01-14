//! Assistant for [`PongProcess`] functionality.

use crate::process_lib::pong::PongProcess;

/// Creates new [`PongProcess`] with specified `max_inactivity_window` (in seconds).
pub fn create_ponger(max_inactivity_window: f64) -> PongProcess {
    PongProcess::new_verbose(max_inactivity_window)
}
