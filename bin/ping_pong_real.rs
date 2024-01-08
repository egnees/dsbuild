//! Run ping-pong real ecosystem.

use dsbuild::examples::ping_pong::real;

fn main() {
    const NEED_CYCLES: u32 = 100;
    real::run_real(NEED_CYCLES);
}
