pub fn enable_debug_log() {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Debug)
        .init();
}

pub fn enable_trace_log() {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Trace)
        .init();
}
