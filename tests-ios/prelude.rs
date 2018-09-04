#[macro_use]
extern crate log;
extern crate pretty_env_logger;
extern crate tempfile;

extern crate dispatch;

use std::cell::RefCell;
use std::mem;
use std::os::raw::{c_long, c_void};
use std::sync::{Arc, Barrier, Mutex};
use std::time::{Duration, Instant};

use tempfile::tempfile;

use dispatch::ffi::*;
use dispatch::*;

fn async_increment(queue: &Queue, num: &Arc<Mutex<i32>>) {
    let num = num.clone();
    queue.async(move || {
        let mut num = num.lock().unwrap();
        *num += 1;
    });
}

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
