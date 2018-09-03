use ffi::*;
use {Timeout, WaitTimeout};

/// A counting semaphore.
///
/// Calls to `Semaphore::signal` must be balanced with calls to `Semaphore::wait`.
/// Attempting to dispose of a semaphore with a count lower than value causes an EXC_BAD_INSTRUCTION exception.
#[derive(Debug)]
pub struct Semaphore {
    ptr: dispatch_semaphore_t,
}

impl Semaphore {
    /// Creates new counting semaphore with an initial value.
    ///
    /// Passing zero for the value is useful for
    /// when two threads need to reconcile the completion of a particular event.
    /// Passing a value greater than zero is useful for managing a finite pool of resources,
    /// where the pool size is equal to the value.
    pub fn new(n: u64) -> Self {
        let ptr = unsafe { dispatch_semaphore_create(n as i64) };

        Semaphore { ptr }
    }

    /// Wait (decrement) for a semaphore.
    ///
    /// Decrement the counting semaphore.
    pub fn wait(&self) -> Result<(), WaitTimeout> {
        self.wait_timeout(DISPATCH_TIME_FOREVER)
    }

    /// Wait (decrement) for a semaphoreor until the specified timeout has elapsed.
    ///
    /// Decrement the counting semaphore.
    pub fn wait_timeout<T: Timeout>(&self, timeout: T) -> Result<(), WaitTimeout> {
        let when = timeout.as_raw();

        let n = unsafe { dispatch_semaphore_wait(self.ptr, when) };

        if n == 0 {
            Ok(())
        } else {
            Err(WaitTimeout)
        }
    }

    /// Signal (increment) a semaphore.
    ///
    /// Increment the counting semaphore.
    /// If the previous value was less than zero, this function wakes a waiting thread before returning.
    ///
    /// This function returns `true` if a thread is woken. Otherwise, `false` is returned.
    pub fn signal(&self) -> bool {
        unsafe { dispatch_semaphore_signal(self.ptr) != 0 }
    }
}

unsafe impl Sync for Semaphore {}
unsafe impl Send for Semaphore {}

impl Clone for Semaphore {
    fn clone(&self) -> Self {
        unsafe {
            dispatch_retain(self.ptr);
        }
        Semaphore { ptr: self.ptr }
    }
}

impl Drop for Semaphore {
    fn drop(&mut self) {
        unsafe {
            dispatch_release(self.ptr);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semaphore() {
        let sem = Semaphore::new(1);

        assert!(sem.wait().is_ok());
        assert_eq!(sem.wait_timeout(0).unwrap_err(), WaitTimeout);

        assert!(!sem.signal());
        assert!(sem.wait_timeout(DISPATCH_TIME_FOREVER).is_ok());

        // Calls to dispatch_semaphore_signal must be balanced with calls to wait().
        // Attempting to dispose of a semaphore with a count lower than value causes an EXC_BAD_INSTRUCTION exception.
        assert!(!sem.signal());
    }
}
