pub struct LeaderInfo {
    next_index: Vec<i64>,
    match_index: Vec<i64>,
}

impl LeaderInfo {
    pub fn new(nodes: usize, node_log_size: usize) -> Self {
        Self {
            next_index: vec![node_log_size as i64; nodes],
            match_index: vec![-1; nodes],
        }
    }
}

pub enum Role {
    Leader(LeaderInfo),
    Follower(Option<usize>), // optional id of leader
    Candidate(usize),        // votes granted
}