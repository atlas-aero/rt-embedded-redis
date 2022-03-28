use crate::network::tests::mocks::TestClock;
use crate::network::timeout::{Timeout, TimeoutError};
use alloc::vec;
use embedded_time::duration::Extensions;

#[test]
fn test_new_missing_clock() {
    let timeout: Timeout<TestClock> = Timeout::new(None, 100.microseconds()).unwrap();
    assert_eq!(false, timeout.expired().unwrap());
}

#[test]
fn test_new_zero_duration() {
    let clock = TestClock::new(vec![]);
    let timeout = Timeout::new(Some(&clock), 0.microseconds()).unwrap();

    assert_eq!(false, timeout.expired().unwrap());
}

#[test]
fn test_new_timer_start_error() {
    let clock = TestClock::new(vec![]);
    let timeout = Timeout::new(Some(&clock), 100.microseconds());

    assert_eq!(TimeoutError::TimerStartFailed, timeout.unwrap_err());
}

#[test]
fn test_expired_true() {
    let clock = TestClock::new(vec![100, 300]);
    let timeout = Timeout::new(Some(&clock), 100.microseconds()).unwrap();

    assert_eq!(true, timeout.expired().unwrap());
}

#[test]
fn test_expired_false() {
    let clock = TestClock::new(vec![100, 150]);
    let timeout = Timeout::new(Some(&clock), 100.microseconds()).unwrap();

    assert_eq!(false, timeout.expired().unwrap());
}

#[test]
fn test_expired_error() {
    let clock = TestClock::new(vec![100]);
    let timeout = Timeout::new(Some(&clock), 100.microseconds()).unwrap();

    // Call fails as TestClock next_instants is empty
    assert_eq!(TimeoutError::TimerError, timeout.expired().unwrap_err());
}
