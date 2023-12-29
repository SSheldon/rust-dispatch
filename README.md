Rust wrapper for Apple's Grand Central Dispatch (GCD).

GCD is an implementation of task parallelism that allows tasks to be submitted
to queues where they are scheduled to execute.

For more information, see Apple's [Grand Central Dispatch reference](
https://developer.apple.com/library/mac/documentation/Performance/Reference/GCD_libdispatch_Ref/index.html).

* Documentation: http://ssheldon.github.io/rust-objc/dispatch/
* Crate: https://crates.io/crates/dispatch

# Serial Queues

Serial queues execute tasks serially in FIFO order. The application's main
queue is serial and can be accessed through the `Queue::main` function.

``` rust
use dispatch::{Queue, QueueAttribute};

let queue = Queue::create("com.example.rust", QueueAttribute::Serial);
queue.exec_async(|| println!("Hello"));
queue.exec_async(|| println!("World"));
```

# Concurrent Queues

Concurrent dispatch queues execute tasks concurrently. GCD provides global
concurrent queues that can be accessed through the `Queue::global` function.

`Queue` has two methods that can simplify processing data in parallel,
`for_each` and `map`:

``` rust
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