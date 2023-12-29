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
queue.exec_async(|| println!("Hello"));
queue.exec_async(|| println!("World"));
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
queue.for_each(&mut nums, |x| *x += 1);
assert!(nums == [2, 3]);

let nums = queue.map(nums, |x| x.to_string());
assert!(nums[0] == "2");
```

# Timer Events

GCD provides a timer facility that can be used to schedule blocks of code to
execute periodically, starting after a delay. The `TimerNode` type is a wrapper
around a dispatch source that can be used to schedule timer events.

`TimerNode` has the `schedule` method to schedule a timer event, the `update`
method to update the timer's interval, and the `cancel` method to cancel the
timer. Dropping a `TimerNode` will cancel the timer.

```
use dispatch::TimerNode;
use std::time::Duration;
use std::thread::sleep;
use std::sync::{Arc, Mutex};

let count = Arc::new(Mutex::new(0));
let count_clone = count.clone();
let mut timer = TimerNode::schedule(Duration::from_millis(10), Duration::from_secs(0), None, move || {
    let mut count = count_clone.lock().unwrap();
    *count += 1;
    println!("Hello, counter! -> {}", *count);
}).unwrap();
sleep(Duration::from_millis(100));
timer.update(Duration::from_millis(20), Duration::from_secs(0));
sleep(Duration::from_millis(100));
timer.cancel();
println!("Counter: {}", *count.lock().unwrap());
assert!(*count.lock().unwrap() >= 15);
```

*/

#![warn(missing_docs)]

use std::error::Error;
use std::fmt;
use std::mem;
use std::os::raw::c_void;
use std::time::Duration;

use crate::ffi::*;

pub use crate::group::{Group, GroupGuard};
pub use crate::once::Once;
pub use crate::queue::{Queue, QueueAttribute, QueuePriority, SuspendGuard};
pub use crate::sem::{Semaphore, SemaphoreGuard};
pub use crate::source::TimerNode;

/// Raw foreign function interface for libdispatch.
pub mod ffi;
mod group;
mod queue;
mod once;
mod sem;
mod source;

/// An error indicating a wait timed out.
#[derive(Clone, Debug)]
pub struct WaitTimeout {
    duration: Duration,
}

impl fmt::Display for WaitTimeout {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Wait timed out after duration {:?}", self.duration)
    }
}

impl Error for WaitTimeout { }

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
