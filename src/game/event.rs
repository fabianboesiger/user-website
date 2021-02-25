
use std::collections::VecDeque;
use chrono::{DateTime, Utc};
use std::time::{Duration, Instant};

#[derive(Ord, PartialOrd, Eq, PartialEq)]
pub struct Event {
    expires: DateTime<Utc>,
    id: u64,
}

impl Event {
    fn expires_at(&self) -> Instant {
        let duration = self.expires
            .signed_duration_since(Utc::now())
            .to_std()
            .unwrap();
        Instant::now() + duration
    }
}

pub struct EventQueue {
    deque: VecDeque<Event>,
    next: Instant,
}

impl EventQueue {
    pub fn new() -> Self {
        Self {
            deque: VecDeque::new(),
            next: Instant::now() + Duration::from_secs(60 * 60 * 24 * 1000),
        }
    }

    fn update(&mut self, event: &Event) {
        let instant = event.expires_at();

        if instant < self.next {
            self.next = instant;
        }
    }

    pub async fn next(&mut self) -> Event {
        tokio::time::sleep_until(
            tokio::time::Instant::from_std(self.next)).await;
        
        let next = self.deque.pop_front().unwrap();
        
        if let Some(event) = self.deque.front() {
            let instant = event.expires_at();
            if instant < self.next {
                self.next = instant;
            }
        } else {
            self.next = Instant::now() + Duration::from_secs(60 * 60 * 24 * 1000);
        }

        next
    }

    pub fn insert(&mut self, event: Event) {
        let instant = event.expires_at();
        if instant < self.next {
            self.next = instant;
        }
        
        let index = self.deque.binary_search(&event).unwrap_or_else(|x| x);
        self.deque.insert(index, event);
    }
}
