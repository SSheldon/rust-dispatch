use std::ptr;
use std::time::{Duration, Instant, SystemTime, SystemTimeError, UNIX_EPOCH};

use libc::timespec;

use ffi::*;

/// A type indicating whether a timed wait on a dispatch object returned due to a time out or not.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaitTimeout;

/// When to timeout.
pub trait Timeout {
    /// Extracts the raw `dispatch_time_t`.
    fn as_raw(self) -> dispatch_time_t;
}

impl<T: Timeout> Timeout for Option<T> {
    fn as_raw(self) -> dispatch_time_t {
        if let Some(timeout) = self {
            timeout.as_raw()
        } else {
            DISPATCH_TIME_NOW
        }
    }
}

impl Timeout for i32 {
    fn as_raw(self) -> dispatch_time_t {
        Duration::from_millis(self as u64).as_raw()
    }
}

impl Timeout for u32 {
    fn as_raw(self) -> dispatch_time_t {
        Duration::from_millis(self as u64).as_raw()
    }
}

impl Timeout for Duration {
    fn as_raw(self) -> dispatch_time_t {
        after(self)
    }
}

impl Timeout for dispatch_time_t {
    fn as_raw(self) -> dispatch_time_t {
        self
    }
}

impl Timeout for Instant {
    fn as_raw(self) -> dispatch_time_t {
        self.duration_since(Instant::now()).as_raw()
    }
}

impl Timeout for SystemTime {
    fn as_raw(self) -> dispatch_time_t {
        self.duration_since(SystemTime::now()).unwrap().as_raw()
    }
}

/// Returns a `dispatch_time_t` corresponding to the given time.
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

/// Returns a `dispatch_time_t` corresponding to the wall time.
pub fn now() -> dispatch_time_t {
    unsafe { dispatch_walltime(ptr::null(), 0) }
}

/// Returns a `dispatch_time_t` corresponding to the given time.
pub fn at(tm: SystemTime) -> Result<dispatch_time_t, SystemTimeError> {
    let dur = tm.duration_since(UNIX_EPOCH)?;
    let ts = timespec {
        tv_sec: dur.as_secs() as i64,
        tv_nsec: dur.subsec_nanos() as i64,
    };

    Ok(unsafe { dispatch_walltime(&ts, 0) })
}
