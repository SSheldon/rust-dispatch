extern crate libc;

use std::ffi::CString;
use std::mem;
use std::ptr;
use libc::c_void;

use ffi::*;

pub mod ffi;

pub enum QueueAttribute {
    Serial,
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

pub struct Queue {
    ptr: dispatch_queue_t,
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

impl Queue {
    pub fn create(label: Option<&str>, attr: QueueAttribute) -> Self {
        let label = label.map(|s| CString::new(s).unwrap());
        let label_ptr = label.map_or(ptr::null(), |s| s.as_ptr());
        let attr = attr.as_raw();
        let queue = unsafe { dispatch_queue_create(label_ptr, attr) };
        Queue { ptr: queue }
    }

    pub fn sync<T, F>(&self, work: F) -> T
            where F: Send + FnOnce() -> T, T: Send {
        unsafe {
            let mut result: T = mem::uninitialized();
            let result_ptr: *mut T = &mut result;
            let (context, work) = context_and_function(move || {
                ptr::write(result_ptr, work());
            });
            dispatch_sync_f(self.ptr, context, work);
            result
        }
    }
}

impl Drop for Queue {
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
    fn test_serial_queue() {
        let q = Queue::create(None, QueueAttribute::Serial);
        let mut num = 0;
        q.sync(|| num = 1);
        assert!(num == 1);

        assert!(q.sync(|| num) == 1);
    }
}
