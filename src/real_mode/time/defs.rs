#[derive(Debug, Clone)]
pub struct SetTimerRequest {
    pub process: String,
    pub timer_name: String,
    pub delay: f64,
}

#[derive(Debug, Clone)]
pub struct TimerFiredEvent {
    pub process: String,
    pub timer_name: String,
}