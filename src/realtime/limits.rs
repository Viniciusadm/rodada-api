use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct ConnectionRateLimit {
    max_messages: usize,
    window: Duration,
    seen_at: VecDeque<Instant>,
}

impl ConnectionRateLimit {
    pub fn new(max_messages: usize, window: Duration) -> Self {
        Self {
            max_messages,
            window,
            seen_at: VecDeque::new(),
        }
    }

    pub fn allow(&mut self, now: Instant) -> bool {
        while self
            .seen_at
            .front()
            .is_some_and(|seen_at| now.duration_since(*seen_at) > self.window)
        {
            self.seen_at.pop_front();
        }

        if self.seen_at.len() >= self.max_messages {
            return false;
        }

        self.seen_at.push_back(now);
        true
    }
}
