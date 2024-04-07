use std::time::Instant;

use crate::metrics::{REPUST_REMOTE_TIMER, REPUST_TOTAL_TIMER};

pub enum TrackerType {
    Total,
    Remote,
}

pub struct Tracker {
    pub start: Instant,
    tracker_type: TrackerType,
}

impl std::fmt::Debug for Tracker {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "Tracker<start={:?}>", self.start)
    }
}

impl Tracker {
    pub fn new(tracker_type: TrackerType) -> Tracker {
        Self {
            start: Instant::now(),
            tracker_type,
        }
    }
}

impl Drop for Tracker {
    fn drop(&mut self) {
        let dur = self.start.elapsed();
        match self.tracker_type {
            TrackerType::Total => {
                REPUST_TOTAL_TIMER
                    .get()
                    .unwrap()
                    .record(dur.as_secs_f64(), &[]);
            }
            TrackerType::Remote => {
                REPUST_REMOTE_TIMER
                    .get()
                    .unwrap()
                    .record(dur.as_secs_f64(), &[]);
            }
        }
    }
}

pub fn total_tracker() -> Tracker {
    Tracker::new(TrackerType::Total)
}

pub fn remote_tracker() -> Tracker {
    Tracker::new(TrackerType::Remote)
}
