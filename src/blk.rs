use std::cell::RefCell;
use std::fmt;
use std::os::raw::c_void;
use std::sync::Arc;

use ffi::*;
use {Group, IntoTimeout, QosClass, Queue, WaitTimeout};

bitflags! {
    /// Flags to pass to the `DispatchBlock::create()` functions.
    pub struct BlockFlags: dispatch_block_flags_t {
        /// Flag indicating that a dispatch block object should act as a barrier block
        /// when submitted to a DISPATCH_QUEUE_CONCURRENT queue.
        const BARRIER = 0x1;
        /// Flag indicating that a dispatch block object should execute disassociated
        /// from current execution context attributes such as QOS class, os_activity_t
        /// and properties of the current IPC request (if any).
        const DETACHED = 0x2;
        /// Flag indicating that a dispatch block object should be assigned the execution
        /// context attributes that are current at the time the block object is created.
        const ASSIGN_CURRENT = 0x4;
        /// Flag indicating that a dispatch block object should be not be assigned a QOS class.
        const NO_QOS_CLASS = 0x8;
        /// Flag indicating that execution of a dispatch block object submitted to
        /// a queue should prefer the QOS class assigned to the queue over the QOS class
        /// assigned to the block (resp. associated with the block at the time of submission).
        const INHERIT_QOS_CLASS = 0x10;
        /// Flag indicating that execution of a dispatch block object submitted to
        /// a queue should prefer the QOS class assigned to the block (resp. associated
        /// with the block at the time of submission) over the QOS class assigned to
        /// the queue, as long as doing so will not result in a lower QOS class.
        const ENFORCE_QOS_CLASS = 0x20;
    }
}

/// Creates, synchronously executes, and releases a dispatch block from the specified block and flags.
pub fn perform<F>(flags: BlockFlags, closure: F)
where
    F: 'static + Fn(),
{
    unsafe { dispatch_block_perform(flags.bits(), block(closure)) }
}

/// Dispatch blocks allow you to configure properties of individual units of work on a queue directly.
///
/// They also allow you to address individual work units for the purposes of waiting for their completion,
/// getting notified about their completion, and/or canceling them.
pub struct DispatchBlock {
    ptr: dispatch_block_t,
}

impl DispatchBlock {
    /// Creates a new dispatch block on the heap using a closure.
    pub fn new<F>(closure: F) -> Self
    where
        F: 'static + Fn(),
    {
        Self::create(BlockFlags::INHERIT_QOS_CLASS, closure)
    }

    /// Creates a new dispatch block on the heap using a closure and the given flags.
    pub fn create<F>(flags: BlockFlags, closure: F) -> Self
    where
        F: 'static + Fn(),
    {
        let ptr = unsafe { dispatch_block_create(flags.bits(), block(closure)) };

        DispatchBlock { ptr }
    }

    /// Creates a new dispatch block on the heap from a closure and the given flags,
    /// and assigns it the specified QoS class and relative priority.
    pub fn create_with_qos_class<F>(
        flags: BlockFlags,
        qos_class: QosClass,
        relative_priority: i32,
        closure: F,
    ) -> Self
    where
        F: 'static + Fn(),
    {
        let ptr = unsafe {
            dispatch_block_create_with_qos_class(
                flags.bits(),
                qos_class as u32,
                relative_priority,
                block(closure),
            )
        };

        DispatchBlock { ptr }
    }

    /// Extracts the raw `dispatch_block_t`.
    pub fn as_raw(&self) -> dispatch_block_t {
        self.ptr
    }

    /// Consumes the `DispatchBlock`, returning the wrapped `dispatch_block_t`.
    pub fn into_raw(self) -> dispatch_block_t {
        self.ptr
    }

    /// Waits synchronously until execution of the specified dispatch block has completed or until the specified timeout has elapsed.
    pub fn wait_timeout<T: IntoTimeout>(&self, timeout: T) -> Result<(), WaitTimeout> {
        let when = timeout.into_raw();

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

impl<F: 'static + Fn()> From<F> for DispatchBlock {
    fn from(closure: F) -> Self {
        DispatchBlock::new(closure)
    }
}

impl Queue {
    /// Submits a closure for execution on self and waits until it completes.
    pub fn sync_block<T, F>(&self, flags: BlockFlags, work: F) -> T
    where
        F: 'static + Send + Fn() -> T,
        T: 'static + Send + fmt::Debug,
    {
        let result = Arc::new(RefCell::new(None));
        {
            let result_ref = result.clone();
            let work = move || {
                *result_ref.borrow_mut() = Some(work());
            };
            let block = DispatchBlock::create(flags, work);

            unsafe {
                dispatch_sync(self.ptr, block.clone().into_raw());
            }
        }
        // This was set so it's safe to unwrap
        let result = result.borrow_mut().take();
        result.unwrap()
    }

    /// Submits a closure for asynchronous execution on self and returns dispatch block immediately.
    pub fn async_block<B>(&self, work: B) -> DispatchBlock
    where
        B: Into<DispatchBlock>,
    {
        let block = work.into();

        unsafe {
            dispatch_async(self.ptr, block.clone().into_raw());
        }

        block
    }

    /// After the specified delay, submits a closure for asynchronous execution
    /// on self and returns dispatch block immediately.
    pub fn after_block<T, B>(&self, delay: T, work: B) -> DispatchBlock
    where
        T: IntoTimeout,
        B: Into<DispatchBlock>,
    {
        let when = delay.into_raw();
        let block = work.into();

        unsafe {
            dispatch_after(when, self.ptr, block.clone().into_raw());
        }

        block
    }

    /// Submits a closure to be executed on self the given number of iterations
    /// and waits until it completes.
    pub fn apply_block<F>(&self, iterations: usize, work: F)
    where
        F: 'static + Sync + Fn(usize),
    {
        let block = block(work);

        unsafe {
            dispatch_apply(iterations, self.ptr, block);
        }
    }

    /// Submits a closure to be executed on self as a barrier and waits until
    /// it completes.
    ///
    /// Barriers create synchronization points within a concurrent queue.
    /// If self is concurrent, when it encounters a barrier it delays execution
    /// of the closure (and any further ones) until all closures submitted
    /// before the barrier finish executing.
    /// At that point, the barrier closure executes by itself.
    /// Upon completion, self resumes its normal execution behavior.
    ///
    /// If self is a serial queue or one of the global concurrent queues,
    /// this method behaves like the normal `sync` method.
    pub fn barrier_sync_block<T, F>(&self, flags: BlockFlags, work: F) -> T
    where
        F: 'static + Send + Fn() -> T,
        T: 'static + Send + fmt::Debug,
    {
        let result = Arc::new(RefCell::new(None));
        {
            let result_ref = result.clone();
            let work = move || {
                *result_ref.borrow_mut() = Some(work());
            };
            let block = DispatchBlock::create(flags, work);

            unsafe {
                dispatch_barrier_sync(self.ptr, block.clone().into_raw());
            }
        }
        // This was set so it's safe to unwrap
        let result = result.borrow_mut().take();
        result.unwrap()
    }

    /// Submits a closure to be executed on self as a barrier and returns
    /// a `DispatchBlock` immediately.
    ///
    /// Barriers create synchronization points within a concurrent queue.
    /// If self is concurrent, when it encounters a barrier it delays execution
    /// of the closure (and any further ones) until all closures submitted
    /// before the barrier finish executing.
    /// At that point, the barrier closure executes by itself.
    /// Upon completion, self resumes its normal execution behavior.
    ///
    /// If self is a serial queue or one of the global concurrent queues,
    /// this method behaves like the normal `async` method.
    pub fn barrier_async_block<B>(&self, work: B) -> DispatchBlock
    where
        B: Into<DispatchBlock>,
    {
        let block = work.into();

        unsafe {
            dispatch_barrier_async(self.ptr, block.clone().into_raw());
        }

        block
    }
}

impl Group {
    /// Submits a closure asynchronously to the given `Queue` and associates it
    /// with self and returns a `DispatchBlock` immediately.
    pub fn async_block<B>(&self, queue: &Queue, work: B)
    where
        B: Into<DispatchBlock>,
    {
        let block = work.into();

        unsafe {
            dispatch_group_async(self.ptr, queue.ptr, block.clone().into_raw());
        }
    }

    /// Schedules a closure to be submitted to the given `Queue` when all tasks
    /// associated with self have completed and returns a `DispatchBlock` immediately.
    /// If self is empty, the closure is submitted immediately.
    pub fn notify_block<B>(&self, queue: &Queue, work: B)
    where
        B: Into<DispatchBlock>,
    {
        let block = work.into();

        unsafe {
            dispatch_group_notify(self.ptr, queue.ptr, block.clone().into_raw());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use super::*;
    use QueueAttribute;

    fn async_increment_block(queue: &Queue, num: &Arc<Mutex<i32>>) {
        let num = num.clone();
        queue.async_block(DispatchBlock::create(
            BlockFlags::ASSIGN_CURRENT,
            move || {
                let mut num = num.lock().unwrap();
                *num += 1;
            },
        ));
    }

    #[test]
    fn test_perform_block() {
        let n = Arc::new(Cell::new(0));
        let c = n.clone();

        perform(BlockFlags::NO_QOS_CLASS, move || c.set(123));

        assert_eq!(n.get(), 123);
    }

    #[test]
    fn test_block_block() {
        let n = Arc::new(Cell::new(0));
        let c = n.clone();
        let block = DispatchBlock::create(BlockFlags::NO_QOS_CLASS, move || c.set(123));

        assert!(!block.canceled());

        assert_eq!(block.wait_timeout(100u32), Err(WaitTimeout));

        block.cancel();

        assert!(block.canceled());

        assert_eq!(block.wait_timeout(100u32), Err(WaitTimeout));

        assert!(!block.done());
    }

    #[test]
    fn test_serial_queue_block() {
        let q = Queue::create("", QueueAttribute::Serial);
        let mut num = 0;

        q.sync(|| num = 1);
        assert_eq!(num, 1);
        assert_eq!(q.qos_class(), (QosClass::Unspecified, 0));

        assert_eq!(q.sync_block(BlockFlags::ASSIGN_CURRENT, move || num), 1);
    }

    #[test]
    fn test_serial_queue_async_block() {
        let q = Queue::create("", QueueAttribute::Serial);
        let num = Arc::new(Mutex::new(0));

        async_increment_block(&q, &num);

        // Sync an empty block to ensure the async one finishes
        q.sync(|| ());
        assert_eq!(*num.lock().unwrap(), 1);
    }

    #[test]
    fn test_after_block() {
        let q = Queue::create("", QueueAttribute::Serial);
        let group = Group::create();
        let num = Arc::new(Mutex::new(0));

        let delay = Duration::from_millis(0);
        let num2 = num.clone();
        let start = Instant::now();
        {
            let group = group.clone();
            let guard = RefCell::new(Some(group.enter()));
            q.after_block(delay, move || {
                let mut num = num2.lock().unwrap();
                *num = 1;
                guard.borrow_mut().take().unwrap().leave();
            });
        }

        // Wait for the previous block to complete
        assert!(group.wait_timeout(Duration::from_millis(5000)));
        assert!(start.elapsed() >= delay);
        assert_eq!(*num.lock().unwrap(), 1);
    }

    #[test]
    fn test_apply_block() {
        let q = Queue::create("", QueueAttribute::Serial);
        let num = Arc::new(Mutex::new(0));
        {
            let num = num.clone();
            q.apply_block(5, move |_| *num.lock().unwrap() += 1);
        }
        assert_eq!(*num.lock().unwrap(), 5);
    }

    #[test]
    fn test_barrier_sync_block() {
        let q = Queue::create("", QueueAttribute::Concurrent);
        let num = Arc::new(Mutex::new(0));

        async_increment_block(&q, &num);
        async_increment_block(&q, &num);

        let num2 = num.clone();
        let result = q.barrier_sync_block(BlockFlags::ASSIGN_CURRENT, move || {
            let mut num = num2.lock().unwrap();
            if *num == 2 {
                *num = 10;
            }
            *num
        });
        assert_eq!(result, 10);

        async_increment_block(&q, &num);
        async_increment_block(&q, &num);

        q.barrier_sync(|| ());
        assert_eq!(*num.lock().unwrap(), 12);
    }

    #[test]
    fn test_barrier_async_block() {
        let q = Queue::create("", QueueAttribute::Concurrent);
        let num = Arc::new(Mutex::new(0));

        async_increment_block(&q, &num);
        async_increment_block(&q, &num);

        let num2 = num.clone();
        q.barrier_async_block(move || {
            let mut num = num2.lock().unwrap();
            if *num == 2 {
                *num = 10;
            }
        });

        async_increment_block(&q, &num);
        async_increment_block(&q, &num);

        q.barrier_sync(|| ());
        assert_eq!(*num.lock().unwrap(), 12);
    }

    #[test]
    fn test_group_block() {
        let group = Group::create();
        let q = Queue::create("", QueueAttribute::Serial);
        let num = Arc::new(Mutex::new(0));
        {
            let num = num.clone();
            group.async_block(&q, move || {
                let mut num = num.lock().unwrap();
                *num += 1;
            });
        }

        {
            let group = group.clone();
            let guard = RefCell::new(Some(group.enter()));
            let num = num.clone();
            q.async_block(DispatchBlock::create(
                BlockFlags::ASSIGN_CURRENT,
                move || {
                    let mut num = num.lock().unwrap();
                    *num += 1;
                    guard.borrow_mut().take().unwrap().leave();
                },
            ));
        }

        let notify_group = Group::create();

        {
            let guard = RefCell::new(Some(notify_group.enter()));
            let num = num.clone();
            group.notify_block(&q, move || {
                let mut num = num.lock().unwrap();
                *num *= 5;
                guard.borrow_mut().take().unwrap().leave();
            });
        }

        // Wait for the notify block to finish
        notify_group.wait();
        // If the notify ran, the group should be empty
        assert!(group.is_empty());
        // The notify must have run after the two blocks of the group
        assert_eq!(*num.lock().unwrap(), 10);
    }
}
