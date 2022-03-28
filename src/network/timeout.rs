use embedded_time::duration::{Extensions, Microseconds};
use embedded_time::timer::param::{OneShot, Running};
use embedded_time::{Clock, Timer};

#[derive(Debug, PartialEq)]
pub enum TimeoutError {
    TimerStartFailed,
    TimerError,
}

#[derive(Debug)]
pub struct Timeout<'a, C: Clock> {
    timer: Option<Timer<'a, OneShot, Running, C, Microseconds>>,
}

impl<'a, C: Clock> Timeout<'a, C> {
    pub fn new(clock: Option<&'a C>, duration: Microseconds) -> Result<Timeout<'a, C>, TimeoutError> {
        if clock.is_none() || duration == 0.microseconds() {
            return Ok(Self { timer: None });
        }

        let timer = clock.unwrap().new_timer(duration).start();
        if timer.is_err() {
            return Err(TimeoutError::TimerStartFailed);
        }

        Ok(Self {
            timer: Some(timer.unwrap()),
        })
    }

    pub fn expired(&self) -> Result<bool, TimeoutError> {
        if self.timer.is_none() {
            return Ok(false);
        }

        match self.timer.as_ref().unwrap().is_expired() {
            Ok(result) => Ok(result),
            Err(_) => Err(TimeoutError::TimerError),
        }
    }
}
