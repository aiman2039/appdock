use std::time::Duration;
const PUBLISH: Duration = Duration::from_millis(2100);
const DISCOVER: Duration = Duration::from_millis(10500);

/// The caller supplies monotonic elapsed time; tests use a fake clock. Missed
/// deadlines run once and schedule from now, with no catch-up bursts.
#[derive(Default)]
pub struct RefreshSchedule {
    publish: Duration,
    discover: Duration,
}
impl RefreshSchedule {
    pub fn due(&mut self, now: Duration, pointer_down: bool) -> (bool, bool) {
        let publish = now >= self.publish;
        let discover = now >= self.discover && !pointer_down;
        if publish {
            self.publish = now + PUBLISH;
        }
        if discover {
            self.discover = now + DISCOVER;
        }
        (publish, discover)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn d1_initial_boundaries_delays_and_pointer_resumption() {
        let mut clock = RefreshSchedule::default();
        assert_eq!(clock.due(Duration::ZERO, false), (true, true));
        assert_eq!(
            clock.due(Duration::from_millis(2099), false),
            (false, false)
        );
        assert_eq!(clock.due(PUBLISH, false), (true, false));
        assert_eq!(clock.due(DISCOVER, true), (true, false));
        assert_eq!(
            clock.due(DISCOVER + Duration::from_millis(1), false),
            (false, true)
        );
        let delayed = Duration::from_secs(200);
        assert_eq!(clock.due(delayed, false), (true, true));
        assert_eq!(clock.due(delayed, false), (false, false));
        assert_eq!(clock.due(delayed + PUBLISH, false), (true, false));
        assert_eq!(clock.due(delayed + DISCOVER, false), (true, true));
    }
}

/// Probe immediately, then once per minute; busy sessions do not consume a deadline.
#[derive(Default)]
pub struct UpdateSchedule {
    next: Duration,
}
impl UpdateSchedule {
    pub fn due(&mut self, now: Duration, enabled: bool, available: bool) -> bool {
        if !enabled || !available || now < self.next {
            return false;
        }
        self.next = now + Duration::from_secs(60);
        true
    }
    pub fn reset(&mut self) {
        self.next = Duration::ZERO;
    }
}
#[cfg(test)]
mod update_tests {
    use super::*;
    #[test]
    fn minute_polling_handles_disabled_busy_and_sleep_without_bursts() {
        let mut s = UpdateSchedule::default();
        assert!(!s.due(Duration::ZERO, false, true));
        assert!(!s.due(Duration::ZERO, true, false));
        assert!(s.due(Duration::ZERO, true, true));
        assert!(!s.due(Duration::from_secs(59), true, true));
        assert!(s.due(Duration::from_secs(60), true, true));
        assert!(!s.due(Duration::from_secs(120), true, false));
        assert!(s.due(Duration::from_secs(121), true, true));
        assert!(s.due(Duration::from_secs(600), true, true));
        assert!(!s.due(Duration::from_secs(600), true, true));
        s.reset();
        assert!(s.due(Duration::from_secs(601), true, true));
    }
}
