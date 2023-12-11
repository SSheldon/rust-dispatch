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
*/

#![warn(missing_docs)]

use std::convert::TryFrom;
use std::error::Error;
use std::fmt;
use std::os::raw::c_void;
use std::panic::AssertUnwindSafe;
use std::time::Duration;

use crate::ffi::*;

pub use crate::group::{Group, GroupGuard};
pub use crate::once::Once;
pub use crate::queue::{Queue, QueueAttribute, QueuePriority, SuspendGuard};
pub use crate::sem::{Semaphore, SemaphoreGuard};

/// Raw foreign function interface for libdispatch.
pub mod ffi;
mod group;
mod once;
mod queue;
mod sem;

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

impl Error for WaitTimeout {}

fn time_after_delay(delay: Duration) -> dispatch_time_t {
    i64::try_from(delay.as_nanos()).map_or(DISPATCH_TIME_FOREVER, |i| unsafe {
        dispatch_time(DISPATCH_TIME_NOW, i)
    })
}

fn context_and_function<F>(closure: F) -> (*mut c_void, dispatch_function_t)
where
    F: 'static + FnOnce(),
{
    extern "C" fn work_execute_closure<F>(context: *mut c_void)
    where
        F: FnOnce(),
    {
        let closure: Box<F> = unsafe { Box::from_raw(context as *mut _) };
        if std::panic::catch_unwind(AssertUnwindSafe(closure)).is_err() {
            // Abort to prevent unwinding across FFI
            std::process::abort();
        }
    }

    let closure = Box::new(closure);
    let func: dispatch_function_t = work_execute_closure::<F>;
    (Box::into_raw(closure) as *mut c_void, func)
}

fn with_context_and_sync_function<F, T>(
    closure: F,
    wrapper: impl FnOnce(*mut c_void, dispatch_function_t),
) -> Option<T>
where
    F: FnOnce() -> T,
{
    #[derive(Debug)]
    struct SyncContext<F, T> {
        closure: *mut F,
        result: Option<std::thread::Result<T>>,
    }

    extern "C" fn work_execute_closure<F, T>(context: *mut c_void)
    where
        F: FnOnce() -> T,
    {
        let sync_context: &mut SyncContext<F, T> = unsafe { &mut *(context as *mut _) };
        let closure = unsafe { Box::from_raw(sync_context.closure) };
        sync_context.result = Some(std::panic::catch_unwind(AssertUnwindSafe(closure)));
    }

    let mut sync_context: SyncContext<F, T> = SyncContext {
        closure: Box::into_raw(Box::new(closure)),
        result: None,
    };
    let func: dispatch_function_t = work_execute_closure::<F, T>;
    wrapper(&mut sync_context as *mut _ as *mut c_void, func);

    // If the closure panicked, resume unwinding
    match sync_context.result.transpose() {
        Ok(res) => {
            if res.is_none() {
                // if the closure didn't run (for example when using `Once`), free the box
                std::mem::drop(unsafe { Box::from_raw(sync_context.closure) });
            }
            res
        }
        Err(unwind_payload) => std::panic::resume_unwind(unwind_payload),
    }
}

fn context_and_apply_function<F>(closure: &F) -> (*mut c_void, extern "C" fn(*mut c_void, usize))
where
    F: Fn(usize),
{
    extern "C" fn work_apply_closure<F>(context: *mut c_void, iter: usize)
    where
        F: Fn(usize),
    {
        let context: &F = unsafe { &*(context as *const _) };
        if std::panic::catch_unwind(AssertUnwindSafe(|| context(iter))).is_err() {
            // Abort to prevent unwinding across FFI
            std::process::abort();
        }
    }

    let context: *const F = closure;
    let func: extern "C" fn(*mut c_void, usize) = work_apply_closure::<F>;
    (context as *mut c_void, func)
}
