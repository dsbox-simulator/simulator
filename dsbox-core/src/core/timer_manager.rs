use crate::core::node::NodeId;
use std::collections::VecDeque;
use tokio::time::Instant;

pub struct TimerManager {
    timers: VecDeque<Timer>,
}

pub struct Timer {
    pub deadline: Instant,
    pub kind: TimerKind,
}

pub enum TimerKind {
    TimerService {
        source: String,
        msg_id: Option<usize>,
        name: String,
    },
    ExpectRegistry {
        node_id: NodeId,
    },
    ShutdownTimeout {
        node_ids: Vec<NodeId>,
    },
}

impl TimerManager {
    pub fn new() -> Self {
        Self {
            timers: VecDeque::new(),
        }
    }

    pub fn add(&mut self, deadline: Instant, kind: TimerKind) {
        let insert_idx = self
            .timers
            .binary_search_by_key(&deadline, |t| t.deadline)
            .unwrap_or_else(|idx| idx);
        self.timers.insert(insert_idx, Timer { deadline, kind })
    }

    pub async fn wait_next(&mut self) -> Timer {
        if let Some(timer) = self.timers.front() {
            if Instant::now() < timer.deadline {
                tokio::time::sleep_until(timer.deadline).await;
            }
            self.timers.pop_front().unwrap()
        } else {
            std::future::pending().await
        }
    }

    pub fn retain<P>(&mut self, predicate: P)
    where
        P: FnMut(&Timer) -> bool,
    {
        self.timers.retain(predicate);
    }
}
