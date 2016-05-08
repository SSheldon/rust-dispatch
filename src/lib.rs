/*!
Rust wrapper for Apple's Grand Central Dispatch (GCD).

GCD is an implementation of task parallelism that allows tasks to be submitted
to queues where they are scheduled to execute.

For more information, see Apple's [Grand Central Dispatch reference](
https://developer.apple.com/library/mac/documentation/Performance/Reference/GCD_libdispatch_Ref/index.html).

# Serial Queues

Serial queues execute tasks serially in FIFO order. The application's main
queue is serial and can be accessed through the `Queue::main` function.

```
use dispatch::{Queue, QueueAttribute};

let queue = Queue::create("com.example.rust", QueueAttribute::Serial);
queue.async(|| println!("Hello"));
queue.async(|| println!("World"));
```

# Concurrent Queues

Concurrent dispatch queues execute tasks concurrently. GCD provides global
concurrent queues that can be accessed through the `Queue::global` function.

`Queue` has two methods that can simplify processing data in parallel, `foreach`
and `map`:

```
use dispatch::{Queue, QueuePriority};

let queue = Queue::global(QueuePriority::Default);

let mut nums = vec![1, 2];
queue.foreach(&mut nums, |x| *x += 1);
assert!(nums == [2, 3]);

let nums = queue.map(nums, |x| x.to_string());
assert!(nums[0] == "2");
```
*/

#![warn(missing_docs)]

use std::cell::UnsafeCell;
use std::ffi::{CStr, CString};
use std::mem;
use std::os::raw::{c_long, c_void};
use std::ptr;
use std::str;
use std::time::Duration;

use ffi::*;

/// Raw foreign function interface for libdispatch.
pub mod ffi;

/// The type of a dispatch queue.
pub enum QueueAttribute {
    /// The queue executes blocks serially in FIFO order.
    Serial,
    /// The queue executes blocks concurrently.
    Concurrent,
}

impl QueueAttribute {
    fn as_raw(&self) -> dispatch_queue_attr_t {
        match *self {
            QueueAttribute::Serial => DISPATCH_QUEUE_SERIAL,
            QueueAttribute::Concurrent => DISPATCH_QUEUE_CONCURRENT,
        }
    }
}

/// The priority of a global concurrent queue.
pub enum QueuePriority {
    /// The queue is scheduled for execution before any default priority or low
    /// priority queue.
    High,
    /// The queue is scheduled for execution after all high priority queues,
    /// but before any low priority queues.
    Default,
    /// The queue is scheduled for execution after all default priority and
    /// high priority queues.
    Low,
    /// The queue is scheduled for execution after all high priority queues
    /// have been scheduled. The system runs items on a thread whose
    /// priority is set for background status and any disk I/O is throttled to
    /// minimize the impact on the system.
    Background,
}

impl QueuePriority {
    fn as_raw(&self) -> c_long {
        match *self {
            QueuePriority::High       => DISPATCH_QUEUE_PRIORITY_HIGH,
            QueuePriority::Default    => DISPATCH_QUEUE_PRIORITY_DEFAULT,
            QueuePriority::Low        => DISPATCH_QUEUE_PRIORITY_LOW,
            QueuePriority::Background => DISPATCH_QUEUE_PRIORITY_BACKGROUND,
        }
    }
}

/// A Grand Central Dispatch queue.
///
/// For more information, see Apple's [Grand Central Dispatch reference](
/// https://developer.apple.com/library/mac/documentation/Performance/Reference/GCD_libdispatch_Ref/index.html).
pub struct Queue {
    ptr: dispatch_queue_t,
}

fn time_after_delay(delay: Duration) -> dispatch_time_t {
    delay.as_secs().checked_mul(1_000_000_000).and_then(|i| {
        i.checked_add(delay.subsec_nanos() as u64)
    }).and_then(|i| {
        if i < (i64::max_value() as u64) { Some(i as i64) } else { None }
    }).map_or(DISPATCH_TIME_FOREVER, |i| unsafe {
        dispatch_time(DISPATCH_TIME_NOW, i)
    })
}

fn context_and_function<F>(closure: F) -> (*mut c_void, dispatch_function_t)
        where F: FnOnce() {
    extern fn work_execute_closure<F>(context: Box<F>) where F: FnOnce() {
        (*context)();
    }

    let closure = Box::new(closure);
    let func: extern fn(Box<F>) = work_execute_closure::<F>;
    unsafe {
        (mem::transmute(closure), mem::transmute(func))
    }
}

fn context_and_sync_function<F>(closure: &mut Option<F>) ->
        (*mut c_void, dispatch_function_t)
        where F: FnOnce() {
    extern fn work_read_closure<F>(context: &mut Option<F>) where F: FnOnce() {
        // This is always passed Some, so it's safe to unwrap
        let closure = context.take().unwrap();
        closure();
    }

    let context: *mut Option<F> = closure;
    let func: extern fn(&mut Option<F>) = work_read_closure::<F>;
    unsafe {
        (context as *mut c_void, mem::transmute(func))
    }
}

fn context_and_apply_function<F>(closure: &F) ->
        (*mut c_void, extern fn(*mut c_void, usize))
        where F: Fn(usize) {
    extern fn work_apply_closure<F>(context: &F, iter: usize)
            where F: Fn(usize) {
        context(iter);
    }

    let context: *const F = closure;
    let func: extern fn(&F, usize) = work_apply_closure::<F>;
    unsafe {
        (context as *mut c_void, mem::transmute(func))
    }
}

impl Queue {
    /// Returns the serial dispatch `Queue` associated with the application's
    /// main thread.
    pub fn main() -> Self {
        let queue = dispatch_get_main_queue();
        unsafe {
            dispatch_retain(queue);
        }
        Queue { ptr: queue }
    }

    /// Returns a system-defined global concurrent `Queue` with the specified
    /// priority.
    pub fn global(priority: QueuePriority) -> Self {
        unsafe {
            let queue = dispatch_get_global_queue(priority.as_raw(), 0);
            dispatch_retain(queue);
            Queue { ptr: queue }
        }
    }

    /// Creates a new dispatch `Queue`.
    pub fn create(label: &str, attr: QueueAttribute) -> Self {
        let label = CString::new(label).unwrap();
        let queue = unsafe {
            dispatch_queue_create(label.as_ptr(), attr.as_raw())
        };
        Queue { ptr: queue }
    }

    /// Creates a new dispatch `Queue` with the given target queue.
    ///
    /// A dispatch queue's priority is inherited from its target queue.
    /// Additionally, if both the queue and its target are serial queues,
    /// their blocks will not be invoked concurrently.
    pub fn with_target_queue(label: &str, attr: QueueAttribute, target: &Queue)
            -> Self {
        let queue = Queue::create(label, attr);
        unsafe {
            dispatch_set_target_queue(queue.ptr, target.ptr);
        }
        queue
    }

    /// Returns the label that was specified for self.
    pub fn label(&self) -> &str {
        let label = unsafe {
            let label_ptr = dispatch_queue_get_label(self.ptr);
            if label_ptr.is_null() {
                return "";
            }
            CStr::from_ptr(label_ptr)
        };
        str::from_utf8(label.to_bytes()).unwrap()
    }

    /// Submits a closure for execution on self and waits until it completes.
    pub fn sync<T, F>(&self, work: F) -> T
            where F: Send + FnOnce() -> T, T: Send {
        let mut result = None;
        {
            let result_ref = &mut result;
            let work = move || {
                *result_ref = Some(work());
            };

            let mut work = Some(work);
            let (context, work) = context_and_sync_function(&mut work);
            unsafe {
                dispatch_sync_f(self.ptr, context, work);
            }
        }
        // This was set so it's safe to unwrap
        result.unwrap()
    }

    /// Submits a closure for asynchronous execution on self and returns
    /// immediately.
    pub fn async<F>(&self, work: F) where F: 'static + Send + FnOnce() {
        let (context, work) = context_and_function(work);
        unsafe {
            dispatch_async_f(self.ptr, context, work);
        }
    }

    /// After the specified delay, submits a closure for asynchronous execution
    /// on self.
    pub fn after_ms<F>(&self, ms: u32, work: F)
            where F: 'static + Send + FnOnce() {
        self.after(Duration::from_millis(ms as u64), work);
    }

    /// After the specified delay, submits a closure for asynchronous execution
    /// on self.
    pub fn after<F>(&self, delay: Duration, work: F)
            where F: 'static + Send + FnOnce() {
        let when = time_after_delay(delay);
        let (context, work) = context_and_function(work);
        unsafe {
            dispatch_after_f(when, self.ptr, context, work);
        }
    }

    /// Submits a closure to be executed on self the given number of iterations
    /// and waits until it completes.
    pub fn apply<F>(&self, iterations: usize, work: F)
            where F: Sync + Fn(usize) {
        let (context, work) = context_and_apply_function(&work);
        unsafe {
            dispatch_apply_f(iterations, self.ptr, context, work);
        }
    }

    /// Submits a closure to be executed on self for each element of the
    /// provided slice and waits until it completes.
    pub fn foreach<T, F>(&self, slice: &mut [T], work: F)
            where F: Sync + Fn(&mut T), T: Send {
        let slice_ptr = slice.as_mut_ptr();
        let work = move |i| unsafe {
            work(&mut *slice_ptr.offset(i as isize));
        };
        let (context, work) = context_and_apply_function(&work);
        unsafe {
            dispatch_apply_f(slice.len(), self.ptr, context, work);
        }
    }

    /// Submits a closure to be executed on self for each element of the
    /// provided vector and returns a `Vec` of the mapped elements.
    pub fn map<T, U, F>(&self, vec: Vec<T>, work: F) -> Vec<U>
            where F: Sync + Fn(T) -> U, T: Send, U: Send {
        let mut src = vec;
        let len = src.len();
        let src_ptr = src.as_ptr();

        let mut dest = Vec::with_capacity(len);
        let dest_ptr = dest.as_mut_ptr();

        let work = move |i| unsafe {
            let result = work(ptr::read(src_ptr.offset(i as isize)));
            ptr::write(dest_ptr.offset(i as isize), result);
        };
        let (context, work) = context_and_apply_function(&work);
        unsafe {
            src.set_len(0);
            dispatch_apply_f(len, self.ptr, context, work);
            dest.set_len(len);
        }

        dest
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
    pub fn barrier_sync<T, F>(&self, work: F) -> T
            where F: Send + FnOnce() -> T, T: Send {
        let mut result = None;
        {
            let result_ref = &mut result;
            let work = move || {
                *result_ref = Some(work());
            };

            let mut work = Some(work);
            let (context, work) = context_and_sync_function(&mut work);
            unsafe {
                dispatch_barrier_sync_f(self.ptr, context, work);
            }
        }
        // This was set so it's safe to unwrap
        result.unwrap()
    }

    /// Submits a closure to be executed on self as a barrier and returns
    /// immediately.
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
    pub fn barrier_async<F>(&self, work: F)
            where F: 'static + Send + FnOnce() {
        let (context, work) = context_and_function(work);
        unsafe {
            dispatch_barrier_async_f(self.ptr, context, work);
        }
    }

    /// Suspends the invocation of blocks on self and returns a `SuspendGuard`
    /// that can be dropped to resume.
    ///
    /// The suspension occurs after completion of any blocks running at the
    /// time of the call.
    /// Invocation does not resume until all `SuspendGuard`s have been dropped.
    pub fn suspend(&self) -> SuspendGuard {
        SuspendGuard::new(self)
    }
}

unsafe impl Sync for Queue { }
unsafe impl Send for Queue { }

impl Clone for Queue {
    fn clone(&self) -> Self {
        unsafe {
            dispatch_retain(self.ptr);
        }
        Queue { ptr: self.ptr }
    }
}

impl Drop for Queue {
    fn drop(&mut self) {
        unsafe {
            dispatch_release(self.ptr);
        }
    }
}

/// An RAII guard which will resume a suspended `Queue` when dropped.
pub struct SuspendGuard {
    queue: Queue,
}

impl SuspendGuard {
    fn new(queue: &Queue) -> SuspendGuard {
        unsafe {
            dispatch_suspend(queue.ptr);
        }
        SuspendGuard { queue: queue.clone() }
    }

    /// Drops self, allowing the suspended `Queue` to resume.
    pub fn resume(self) { }
}

impl Clone for SuspendGuard {
    fn clone(&self) -> Self {
        SuspendGuard::new(&self.queue)
    }
}

impl Drop for SuspendGuard {
    fn drop(&mut self) {
        unsafe {
            dispatch_resume(self.queue.ptr);
        }
    }
}

/// A Grand Central Dispatch group.
///
/// A `Group` is a mechanism for grouping closures and monitoring them. This
/// allows for aggregate synchronization, so you can track when all the
/// closures complete, even if they are running on different queues.
pub struct Group {
    ptr: dispatch_group_t,
}

impl Group {
    /// Creates a new dispatch `Group`.
    pub fn create() -> Group {
        unsafe {
            Group { ptr: dispatch_group_create() }
        }
    }

    /// Indicates that a closure has entered self, and increments the current
    /// count of outstanding tasks. Returns a `GroupGuard` that should be
    /// dropped when the closure leaves self, decrementing the count.
    pub fn enter(&self) -> GroupGuard {
        GroupGuard::new(self)
    }

    /// Submits a closure asynchronously to the given `Queue` and associates it
    /// with self.
    pub fn async<F>(&self, queue: &Queue, work: F)
            where F: 'static + Send + FnOnce() {
        let (context, work) = context_and_function(work);
        unsafe {
            dispatch_group_async_f(self.ptr, queue.ptr, context, work);
        }
    }

    /// Schedules a closure to be submitted to the given `Queue` when all tasks
    /// associated with self have completed.
    /// If self is empty, the closure is submitted immediately.
    pub fn notify<F>(&self, queue: &Queue, work: F)
            where F: 'static + Send + FnOnce() {
        let (context, work) = context_and_function(work);
        unsafe {
            dispatch_group_notify_f(self.ptr, queue.ptr, context, work);
        }
    }

    /// Waits synchronously for all tasks associated with self to complete.
    pub fn wait(&self) {
        let result = unsafe {
            dispatch_group_wait(self.ptr, DISPATCH_TIME_FOREVER)
        };
        assert!(result == 0, "Dispatch group wait errored");
    }

    /// Waits for all tasks associated with self to complete within the
    /// specified duration.
    /// Returns true if the tasks completed or false if the timeout elapsed.
    pub fn wait_timeout_ms(&self, ms: u32) -> bool {
        self.wait_timeout(Duration::from_millis(ms as u64))
    }

    /// Waits for all tasks associated with self to complete within the
    /// specified duration.
    /// Returns true if the tasks completed or false if the timeout elapsed.
    pub fn wait_timeout(&self, timeout: Duration) -> bool {
        let when = time_after_delay(timeout);
        let result = unsafe {
            dispatch_group_wait(self.ptr, when)
        };
        result == 0
    }

    /// Returns whether self is currently empty.
    pub fn is_empty(&self) -> bool {
        let result = unsafe {
            dispatch_group_wait(self.ptr, DISPATCH_TIME_NOW)
        };
        result == 0
    }
}

unsafe impl Sync for Group { }
unsafe impl Send for Group { }

impl Clone for Group {
    fn clone(&self) -> Self {
        unsafe {
            dispatch_retain(self.ptr);
        }
        Group { ptr: self.ptr }
    }
}

impl Drop for Group {
    fn drop(&mut self) {
        unsafe {
            dispatch_release(self.ptr);
        }
    }
}

/// An RAII guard which will leave a `Group` when dropped.
pub struct GroupGuard {
    group: Group,
}

impl GroupGuard {
    fn new(group: &Group) -> GroupGuard {
        unsafe {
            dispatch_group_enter(group.ptr);
        }
        GroupGuard { group: group.clone() }
    }

    /// Drops self, leaving the `Group`.
    pub fn leave(self) { }
}

impl Clone for GroupGuard {
    fn clone(&self) -> Self {
        GroupGuard::new(&self.group)
    }
}

impl Drop for GroupGuard {
    fn drop(&mut self) {
        unsafe {
            dispatch_group_leave(self.group.ptr);
        }
    }
}

#[allow(dead_code)]
struct Once {
    predicate: UnsafeCell<dispatch_once_t>,
}

#[allow(dead_code)]
impl Once {
    // TODO: make this a const fn when the feature is stable
    pub fn new() -> Once {
        Once { predicate: UnsafeCell::new(0) }
    }

    #[inline(always)]
    pub fn call_once<F>(&'static self, work: F) where F: FnOnce() {
        #[cold]
        #[inline(never)]
        fn once<F>(predicate: *mut dispatch_once_t, work: F)
                where F: FnOnce() {
            let mut work = Some(work);
            let (context, work) = context_and_sync_function(&mut work);
            unsafe {
                dispatch_once_f(predicate, context, work);
            }
        }

        unsafe {
            let predicate = self.predicate.get();
            if *predicate != !0 {
                once(predicate, work);
            }
        }
    }
}

unsafe impl Sync for Once { }

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};
    use super::*;

    fn async_increment(queue: &Queue, num: &Arc<Mutex<i32>>) {
        let num = num.clone();
        queue.async(move || {
            let mut num = num.lock().unwrap();
            *num += 1;
        });
    }

    #[test]
    fn test_serial_queue() {
        let q = Queue::create("", QueueAttribute::Serial);
        let mut num = 0;

        q.sync(|| num = 1);
        assert!(num == 1);

        assert!(q.sync(|| num) == 1);
    }

    #[test]
    fn test_sync_owned() {
        let q = Queue::create("", QueueAttribute::Serial);

        let s = "Hello, world!".to_string();
        let len = q.sync(move || s.len());
        assert!(len == 13);
    }

    #[test]
    fn test_serial_queue_async() {
        let q = Queue::create("", QueueAttribute::Serial);
        let num = Arc::new(Mutex::new(0));

        async_increment(&q, &num);

        // Sync an empty block to ensure the async one finishes
        q.sync(|| ());
        assert!(*num.lock().unwrap() == 1);
    }

    #[test]
    fn test_after() {
        let q = Queue::create("", QueueAttribute::Serial);
        let group = Group::create();
        let num = Arc::new(Mutex::new(0));

        let delay = Duration::from_millis(5);
        let num2 = num.clone();
        let guard = group.enter();
        let start = Instant::now();
        q.after(delay, move || {
            let mut num = num2.lock().unwrap();
            *num = 1;
            guard.leave();
        });

        // Wait for the previous block to complete
        assert!(group.wait_timeout(Duration::from_millis(5000)));
        assert!(start.elapsed() >= delay);
        assert!(*num.lock().unwrap() == 1);
    }

    #[test]
    fn test_queue_label() {
        let q = Queue::create("com.example.rust", QueueAttribute::Serial);
        assert!(q.label() == "com.example.rust");
    }

    #[test]
    fn test_apply() {
        let q = Queue::create("", QueueAttribute::Serial);
        let num = Arc::new(Mutex::new(0));

        q.apply(5, |_| *num.lock().unwrap() += 1);
        assert!(*num.lock().unwrap() == 5);
    }

    #[test]
    fn test_foreach() {
        let q = Queue::create("", QueueAttribute::Serial);
        let mut nums = [0, 1];

        q.foreach(&mut nums, |x| *x += 1);
        assert!(nums == [1, 2]);
    }

    #[test]
    fn test_map() {
        let q = Queue::create("", QueueAttribute::Serial);
        let nums = vec![0, 1];

        let result = q.map(nums, |x| x + 1);
        assert!(result == [1, 2]);
    }

    #[test]
    fn test_barrier_sync() {
        let q = Queue::create("", QueueAttribute::Concurrent);
        let num = Arc::new(Mutex::new(0));

        async_increment(&q, &num);
        async_increment(&q, &num);

        let num2 = num.clone();
        let result = q.barrier_sync(move || {
            let mut num = num2.lock().unwrap();
            if *num == 2 {
                *num = 10;
            }
            *num
        });
        assert!(result == 10);

        async_increment(&q, &num);
        async_increment(&q, &num);

        q.barrier_sync(|| ());
        assert!(*num.lock().unwrap() == 12);
    }

    #[test]
    fn test_barrier_async() {
        let q = Queue::create("", QueueAttribute::Concurrent);
        let num = Arc::new(Mutex::new(0));

        async_increment(&q, &num);
        async_increment(&q, &num);

        let num2 = num.clone();
        q.barrier_async(move || {
            let mut num = num2.lock().unwrap();
            if *num == 2 {
                *num = 10;
            }
        });

        async_increment(&q, &num);
        async_increment(&q, &num);

        q.barrier_sync(|| ());
        assert!(*num.lock().unwrap() == 12);
    }

    #[test]
    fn test_suspend() {
        let q = Queue::create("", QueueAttribute::Serial);
        let num = Arc::new(Mutex::new(0));

        // Suspend the queue and then dispatch some work to it
        let guard = q.suspend();
        async_increment(&q, &num);

        // Sleep and ensure the work doesn't occur
        ::std::thread::sleep(Duration::from_millis(5));
        assert!(*num.lock().unwrap() == 0);

        // But ensure the work does complete after we resume
        guard.resume();
        q.sync(|| ());
        assert!(*num.lock().unwrap() == 1);
    }

    #[test]
    fn test_group() {
        let group = Group::create();
        let q = Queue::create("", QueueAttribute::Serial);
        let num = Arc::new(Mutex::new(0));

        let num2 = num.clone();
        group.async(&q, move || {
            let mut num = num2.lock().unwrap();
            *num += 1;
        });

        let guard = group.enter();
        assert!(!group.is_empty());
        let num3 = num.clone();
        q.async(move || {
            let mut num = num3.lock().unwrap();
            *num += 1;
            guard.leave();
        });

        let num4 = num.clone();
        group.notify(&q, move || {
            let mut num = num4.lock().unwrap();
            if *num == 2 {
                *num = 10;
            }
        });

        group.wait();
        assert!(group.is_empty());

        // Our group is empty, but the notify may not have finished yet
        q.sync(|| ());
        assert!(*num.lock().unwrap() == 10);
    }
}
