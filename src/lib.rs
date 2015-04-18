extern crate libc;

use std::ffi::CString;
use std::mem;
use std::ptr;
use libc::{c_long, c_void, size_t};

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

pub enum QueuePriority {
    High,
    Default,
    Low,
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

fn context_and_apply_function<F>(closure: &F) ->
        (*mut c_void, extern fn(*mut c_void, size_t))
        where F: Fn(usize) {
    extern fn work_apply_closure<F>(context: &F, iter: size_t)
            where F: Fn(usize) {
        context(iter as usize);
    }

    let context: *const F = closure;
    let func: extern fn(&F, size_t) = work_apply_closure::<F>;
    unsafe {
        (context as *mut c_void, mem::transmute(func))
    }
}

impl Queue {
    pub fn main() -> Self {
        let queue = dispatch_get_main_queue();
        unsafe {
            dispatch_retain(queue);
        }
        Queue { ptr: queue }
    }

    pub fn global(priority: QueuePriority) -> Self {
        unsafe {
            let queue = dispatch_get_global_queue(priority.as_raw(), 0);
            dispatch_retain(queue);
            Queue { ptr: queue }
        }
    }

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

    pub fn async<F>(&self, work: F) where F: 'static + Send + FnOnce() {
        let (context, work) = context_and_function(work);
        unsafe {
            dispatch_async_f(self.ptr, context, work);
        }
    }

    pub fn apply<T, F>(&self, slice: &mut [T], work: F)
            where F: Send + Sync + Fn(&mut T), T: Send {
        let slice_ptr = slice.as_mut_ptr();
        let work = move |i| unsafe {
            work(&mut *slice_ptr.offset(i as isize));
        };
        let (context, work) = context_and_apply_function(&work);
        unsafe {
            dispatch_apply_f(slice.len() as size_t, self.ptr, context, work);
        }
    }

    pub fn map<T, U, F>(&self, vec: Vec<T>, work: F) -> Vec<U>
            where F: Send + Sync + Fn(T) -> U, T: Send, U: Send {
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
            dispatch_apply_f(len as size_t, self.ptr, context, work);
            dest.set_len(len);
        }

        dest
    }
}

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

pub fn main() -> ! {
    unsafe {
        dispatch_main();
    }
    unreachable!();
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

    #[test]
    fn test_serial_queue_async() {
        let q = Queue::create(None, QueueAttribute::Serial);
        let mut num = 0;

        // Create a pointer we can send to our async block
        struct SendPtr(*mut i32);
        unsafe impl Send for SendPtr { }
        let num_ptr = SendPtr(&mut num);

        q.async(move || unsafe { *num_ptr.0 = 1 });

        // Sync an empty block to ensure the async one finishes
        q.sync(|| ());
        assert!(num == 1);
    }

    #[test]
    fn test_apply() {
        let q = Queue::create(None, QueueAttribute::Serial);
        let mut nums = [0, 1];

        q.apply(&mut nums, |x| *x += 1);
        assert!(nums == [1, 2]);
    }

    #[test]
    fn test_map() {
        let q = Queue::create(None, QueueAttribute::Serial);
        let nums = vec![0, 1];

        let result = q.map(nums, |x| x + 1);
        assert!(result == [1, 2]);
    }
}
