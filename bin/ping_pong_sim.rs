//! Run ping-pong ecosystem simulation.

use dsbuild::examples::ping_pong::sim;

fn main() {
    const NEED_CYCLES: u32 = 100;
    sim::run_sim(NEED_CYCLES);
}
