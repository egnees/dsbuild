pub struct LeaderInfo {
    pub next_index: Vec<i64>,
    pub match_index: Vec<i64>,
    pub commit_index: Vec<i64>,
}

impl LeaderInfo {
    pub fn new(nodes: usize, node_log_size: usize) -> Self {
        Self {
            next_index: vec![node_log_size as i64; nodes],
            match_index: vec![-1; nodes],
            commit_index: vec![-1; nodes],
        }
    }

    /// Allows to get commit index according to 'majority' rule.
    /// Return N such that there is at least majority nodes with commit_index >= N
    pub fn commit_index(&self) -> i64 {
        let mut left = -1i64;
        let mut right = i64::MAX / 2;
        while left + 1 < right {
            let mid = (left + right) / 2;
            let cnt = self
                .commit_index
                .iter()
                .map(|i| (*i >= mid) as usize)
                .sum::<usize>();
            if cnt >= self.majority() {
                left = mid;
            } else {
                right = mid;
            }
        }

        left
    }

    fn majority(&self) -> usize {
        self.next_index.len() / 2 + 1
    }
}

pub enum Role {
    Leader(LeaderInfo),
    Follower(Option<usize>), // optional id of leader
    Candidate(usize),        // votes granted
}

//////////////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use super::LeaderInfo;

    fn check_commit_index(info: &mut LeaderInfo, v: Vec<i64>, true_value: i64) {
        assert_eq!(info.next_index.len(), v.len());
        info.commit_index = v;
        assert_eq!(info.commit_index(), true_value);
    }

    #[test]
    fn commit_index_odd_members_count() {
        let mut leader_info = LeaderInfo::new(5, 10);

        let mut check = |v: Vec<i64>, true_value: i64| {
            check_commit_index(&mut leader_info, v, true_value);
        };

        check(vec![1, 2, 2, 5, 3], 2);
        check(vec![2, 2, 0, 0, 0], 0);
        check(vec![2, 5, 5, 0, 5], 5);
        check(vec![-1, -1, 0, 0, 0], 0);
        check(vec![-1, -1, -1, 0, 0], -1);
    }

    #[test]
    fn commit_index_even_members_count() {
        let mut leader_info = LeaderInfo::new(4, 10);

        let mut check = |v: Vec<i64>, true_value: i64| {
            check_commit_index(&mut leader_info, v, true_value);
        };

        check(vec![1, 1, 0, 0], 0);
        check(vec![0, 1, 1, 1], 1);
    }

    #[test]
    fn works_fast() {
        let mut leader_info = LeaderInfo::new(100_000, 50_000_000);
        leader_info.commit_index = (0..100_000).map(|x| x * 3 + 10001).collect();

        let elapsed = {
            let start = SystemTime::now();

            let commit_index = leader_info.commit_index();
            assert_eq!(commit_index, 10001 + ((100_000 / 2) - 1) * 3);

            start.elapsed().unwrap()
        };

        assert!(elapsed < Duration::from_millis(100));
    }
}
