use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::io::{Error, ErrorKind};
use std::mem::transmute;
use std::os::raw::c_void;
use std::ptr;
use std::time::Duration;

use crate::ffi::*;
use crate::time_after_delay;

/// Source: https://stackoverflow.com/a/32270215/5214809
extern "C" fn timer_handler(arg: *mut c_void) {
    let closure: &mut Box<dyn FnMut()> = unsafe { transmute(arg) };
    closure();
}

/// A timer node.
/// 
/// Schedule a timer to run a block of code after a delay using `TimerNode::schedule`.
/// Drop the timer to cancel it.
pub struct TimerNode {
    timer: dispatch_source_t,
}

impl Debug for TimerNode {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "TimerNode {{ timer: {:?} }}", self.timer)
    }
}

impl TimerNode {
    /// Schedules a timer to run a block of code after a delay.
    ///
    /// The block will be run on the default global queue.
    ///
    /// # Arguments
    /// - interval: The interval between executions of the block.
    /// - delay: The delay before the first execution of the block.
    /// - leeway: The leeway to apply to the timer.
    /// - block: The block of code to execute.
    ///
    /// # Returns
    /// A `Result` containing either a `TimerNode` or an `Error`.
    ///
    pub fn schedule<F>(interval: Duration, delay: Duration, leeway: Option<Duration>, block: F) -> Result<Self, Error>
    where
        F: FnMut() + Send,
    {
        let context: Box<Box<dyn FnMut()>> = Box::new(Box::new(block));
        let timer = unsafe {
            dispatch_source_create(
                DISPATCH_SOURCE_TYPE_TIMER,
                std::ptr::null(),
                0,
                ptr::null_mut(),
            )
        };
        if timer.is_null() {
            return Err(Error::new(ErrorKind::Other, "Failed to create timer"));
        }
        let when = time_after_delay(delay);
        let leeway = leeway.unwrap_or(Duration::from_millis(0));
        unsafe {
            dispatch_set_context(timer, transmute(context));
            dispatch_source_set_event_handler_f(timer, timer_handler);
            dispatch_source_set_timer(timer, when, interval.as_nanos() as u64, leeway.as_nanos() as u64);
            dispatch_resume(timer);
        }
        let node = TimerNode { timer: timer };
        Ok(node)
    }

    /// Update the timer with a new interval and delay.
    ///
    /// # Arguments
    /// - interval: The new interval between executions of the block.
    /// - delay: The new delay before the first execution of the block.
    /// - leeway: The new leeway to apply to the timer.
    pub fn update(&self, interval: Duration, delay: Duration, leeway: Option<Duration>) {
        let when = time_after_delay(delay);
        let leeway = leeway.unwrap_or(Duration::from_millis(0));
        unsafe {
            dispatch_suspend(self.timer);
            dispatch_source_set_timer(self.timer, when, interval.as_nanos() as u64, leeway.as_nanos() as u64);
            dispatch_resume(self.timer);
        };
    }

    /// Cancel the timer.
    fn cancel(&self) {
        unsafe {
            let context = dispatch_get_context(self.timer);
            dispatch_source_cancel(self.timer);
            dispatch_release(self.timer);
            let _: Box<Box<dyn FnMut()>> = Box::from_raw(context as *mut _);
        }
    }
}

impl Drop for TimerNode {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(test)]
mod test {
    use crate::source::TimerNode;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_timer() {
        let count = Arc::new(Mutex::new(0));
        let count_clone = count.clone();
        let block = move || {
            let mut count = count_clone.lock().unwrap();
            *count += 1;
            println!("Hello, counter! {}", *count);
        };
        let _node =
            TimerNode::schedule(Duration::from_millis(10), Duration::from_secs(0), None, block).unwrap();
        thread::sleep(Duration::from_millis(100));
        assert!(*count.lock().unwrap() >= 10);
    }
}
