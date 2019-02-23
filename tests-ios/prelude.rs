extern crate dispatch;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use dispatch::*;
use dispatch::ffi::*;

fn async_increment(queue: &Queue, num: &Arc<Mutex<i32>>) {
    let num = num.clone();
    queue.async_exec(move || {
        let mut num = num.lock().unwrap();
        *num += 1;
    });
}
