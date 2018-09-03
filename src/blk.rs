use std::os::raw::c_void;

use ffi::*;
use {Queue, Timeout, WaitTimeout};

/// Creates, synchronously executes, and releases a dispatch block from the specified block and flags.
pub fn perform<F>(flags: dispatch_block_flags_t, closure: F)
where
    F: 'static + Fn(),
{
    unsafe { dispatch_block_perform(flags, block(closure)) }
}

/// Dispatch blocks allow you to configure properties of individual units of work on a queue directly.
/// They also allow you to address individual work units for the purposes of waiting for their completion,
/// getting notified about their completion, and/or canceling them.
pub struct DispatchBlock {
    ptr: dispatch_block_t,
}

impl DispatchBlock {
    /// Creates a new dispatch block on the heap using an existing block and the given flags.
    pub fn create<F>(flags: dispatch_block_flags_t, closure: F) -> Self
    where
        F: 'static + Fn(),
    {
        let ptr = unsafe { dispatch_block_create(flags, block(closure)) };

        DispatchBlock { ptr }
    }

    /// Creates a new dispatch block on the heap from an existing block and the given flags,
    /// and assigns it the specified QoS class and relative priority.
    pub fn create_with_qos_class<F>(
        flags: dispatch_block_flags_t,
        qos_class: dispatch_qos_class_t,
        relative_priority: i32,
        closure: F,
    ) -> Self
    where
        F: 'static + Fn(),
    {
        let ptr = unsafe {
            dispatch_block_create_with_qos_class(
                flags,
                qos_class,
                relative_priority,
                block(closure),
            )
        };

        DispatchBlock { ptr }
    }

    /// Waits synchronously until execution of the specified dispatch block has completed or until the specified timeout has elapsed.
    pub fn wait_timeout<T: Timeout>(&self, timeout: T) -> Result<(), WaitTimeout> {
        let when = timeout.as_raw();

        if unsafe { dispatch_block_wait(self.ptr, when) } == 0 {
            Ok(())
        } else {
            Err(WaitTimeout)
        }
    }

    /// Waits synchronously until execution of the specified dispatch block has completed or forever.
    pub fn wait(&self) -> Result<(), WaitTimeout> {
        if unsafe { dispatch_block_wait(self.ptr, DISPATCH_TIME_FOREVER) } == 0 {
            Ok(())
        } else {
            Err(WaitTimeout)
        }
    }

    /// The dispatch block object completed.
    pub fn done(&self) -> bool {
        unsafe { dispatch_block_wait(self.ptr, DISPATCH_TIME_NOW) == 0 }
    }

    /// Schedules a notification block to be submitted to a queue when the execution of a specified dispatch block has completed.
    pub fn notify(&self, queue: &Queue, notification_block: Self) {
        unsafe { dispatch_block_notify(self.ptr, queue.as_raw(), notification_block.ptr) }
    }

    /// Asynchronously cancels the specified dispatch block.
    pub fn cancel(&self) {
        unsafe { dispatch_block_cancel(self.ptr) }
    }

    /// Tests whether the given dispatch block has been canceled.
    pub fn canceled(&self) -> bool {
        unsafe { dispatch_block_testcancel(self.ptr) != 0 }
    }
}

unsafe impl Sync for DispatchBlock {}
unsafe impl Send for DispatchBlock {}

impl Clone for DispatchBlock {
    fn clone(&self) -> Self {
        let ptr = unsafe { _Block_copy(self.ptr as *const c_void) as dispatch_block_t };

        DispatchBlock { ptr }
    }
}

impl Drop for DispatchBlock {
    fn drop(&mut self) {
        unsafe {
            _Block_release(self.ptr as *mut c_void);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::sync::Arc;

    use super::*;

    #[test]
    fn test_perform() {
        let n = Arc::new(Cell::new(0));
        let c = n.clone();

        perform(DISPATCH_BLOCK_NO_QOS_CLASS, move || c.set(123));

        assert_eq!(n.get(), 123);
    }

    #[test]
    fn test_block() {
        let n = Arc::new(Cell::new(0));
        let c = n.clone();
        let block = DispatchBlock::create(DISPATCH_BLOCK_NO_QOS_CLASS, move || c.set(123));

        assert!(!block.canceled());

        assert_eq!(block.wait_timeout(100u32), Err(WaitTimeout));

        block.cancel();

        assert!(block.canceled());

        assert_eq!(block.wait_timeout(100u32), Err(WaitTimeout));

        assert!(!block.done());
    }
}
