use std::collections::VecDeque;

use tokio::time::Instant;

use crate::core::node::MiddlewareId;

pub struct TimerManager {
    timers: VecDeque<Timer>,
}

pub struct Timer {
    pub deadline: Instant,
    pub source: String,
    pub name: String,
    pub middleware_id: MiddlewareId,
}

impl TimerManager {
    pub fn new() -> Self {
        Self {
            timers: VecDeque::new()
        }
    }

    pub fn add(&mut self, deadline: Instant, source: String, name: String, middleware_id: MiddlewareId) {
        let insert_idx = self.timers.binary_search_by_key(&deadline, |t| t.deadline)
            .unwrap_or_else(|idx| idx);
        self.timers.insert(insert_idx, Timer { deadline, source, name, middleware_id })
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
}