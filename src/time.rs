use std::error::Error;
use std::fmt;
use std::time::{Duration, Instant, SystemTime, SystemTimeError, UNIX_EPOCH};

use libc::{c_long, time_t, timespec};

use ffi::*;

/// A `dispatch_time_t` corresponding to the current time.
pub const NOW: dispatch_time_t = DISPATCH_TIME_NOW;
/// A `dispatch_time_t` corresponding to the maximum time.
pub const FOREVER: dispatch_time_t = DISPATCH_TIME_FOREVER;

/// A type indicating whether a timed wait on a dispatch object returned due to a time out or not.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaitTimeout;

impl fmt::Display for WaitTimeout {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "operation time out")
    }
}

impl Error for WaitTimeout {
    fn description(&self) -> &str {
        "operation time out"
    }
}

/// When to timeout.
pub trait IntoTimeout {
    /// Consumes the `IntoTimeout`, returning the raw `dispatch_time_t`.
    fn into_raw(self) -> dispatch_time_t;
}

impl<T: IntoTimeout> IntoTimeout for Option<T> {
    fn into_raw(self) -> dispatch_time_t {
        if let Some(timeout) = self {
            timeout.into_raw()
        } else {
            DISPATCH_TIME_NOW
        }
    }
}

impl IntoTimeout for i32 {
    fn into_raw(self) -> dispatch_time_t {
        Duration::from_millis(self as u64).into_raw()
    }
}

impl IntoTimeout for u32 {
    fn into_raw(self) -> dispatch_time_t {
        Duration::from_millis(self as u64).into_raw()
    }
}

impl IntoTimeout for Duration {
    fn into_raw(self) -> dispatch_time_t {
        after(self)
    }
}

impl IntoTimeout for dispatch_time_t {
    fn into_raw(self) -> dispatch_time_t {
        self
    }
}

impl IntoTimeout for Instant {
    fn into_raw(self) -> dispatch_time_t {
        self.duration_since(Instant::now()).into_raw()
    }
}

impl IntoTimeout for SystemTime {
    fn into_raw(self) -> dispatch_time_t {
        self.duration_since(SystemTime::now()).unwrap().into_raw()
    }
}

/// Returns a `dispatch_time_t` corresponding to the given duration.
pub fn after(delay: Duration) -> dispatch_time_t {
    delay
        .as_secs()
        .checked_mul(1_000_000_000)
        .and_then(|i| i.checked_add(delay.subsec_nanos() as u64))
        .and_then(|i| {
            if i < (i64::max_value() as u64) {
                Some(i as i64)
            } else {
                None
            }
        })
        .map_or(DISPATCH_TIME_FOREVER, |i| unsafe {
            dispatch_time(DISPATCH_TIME_NOW, i)
        })
}

/// Returns a `dispatch_time_t` corresponding to the given time.
pub fn at(tm: SystemTime) -> Result<dispatch_time_t, SystemTimeError> {
    let dur = tm.duration_since(UNIX_EPOCH)?;
    let ts = timespec {
        tv_sec: dur.as_secs() as time_t,
        tv_nsec: dur.subsec_nanos() as c_long,
    };

    Ok(unsafe { dispatch_walltime(&ts, 0) })
}
