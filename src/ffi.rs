#![allow(non_camel_case_types)]

use libc::{c_char, c_void};

pub type dispatch_function_t = extern fn(*mut c_void);
pub type dispatch_object_t = *mut ();
pub type dispatch_queue_t = *mut ();
pub type dispatch_queue_attr_t = *const ();

#[link(name = "System", kind = "dylib")]
extern {
    static _dispatch_queue_attr_concurrent: ();

    pub fn dispatch_get_main_queue() -> dispatch_queue_t;
    // dispatch_queue_t dispatch_get_global_queue ( long identifier, unsigned long flags );
    pub fn dispatch_queue_create(label: *const c_char, attr: dispatch_queue_attr_t) -> dispatch_queue_t;
    // dispatch_queue_attr_t dispatch_queue_attr_make_with_qos_class ( dispatch_queue_attr_t attr, dispatch_qos_class_t qos_class, int relative_priority );
    // const char * dispatch_queue_get_label ( dispatch_queue_t queue );
    // void dispatch_set_target_queue ( dispatch_object_t object, dispatch_queue_t queue );
    pub fn dispatch_main();

    // void dispatch_async ( dispatch_queue_t queue, dispatch_block_t block );
    pub fn dispatch_async_f(queue: dispatch_queue_t, context: *mut c_void, work: dispatch_function_t);
    // void dispatch_sync ( dispatch_queue_t queue, dispatch_block_t block );
    pub fn dispatch_sync_f(queue: dispatch_queue_t, context: *mut c_void, work: dispatch_function_t);
    // void dispatch_after ( dispatch_time_t when, dispatch_queue_t queue, dispatch_block_t block );
    // void dispatch_after_f ( dispatch_time_t when, dispatch_queue_t queue, void *context, dispatch_function_t work );
    // void dispatch_apply ( size_t iterations, dispatch_queue_t queue, void (^block)(size_t) );
    // void dispatch_apply_f ( size_t iterations, dispatch_queue_t queue, void *context, void (*work)(void *, size_t) );
    // void dispatch_once ( dispatch_once_t *predicate, dispatch_block_t block );
    // void dispatch_once_f ( dispatch_once_t *predicate, void *context, dispatch_function_t function );

    pub fn dispatch_release(object: dispatch_object_t);
}

#[cfg(test)]
mod tests {
    use std::ptr;
    use libc::c_void;
    use super::*;

    #[test]
    fn test_serial_queue() {
        extern fn serial_queue_test_add(num: *mut c_void) {
            unsafe {
                *(num as *mut u32) = 1;
            }
        }

        let mut num: u32 = 0;
        let num_ptr: *mut u32 = &mut num;
        unsafe {
            let q = dispatch_queue_create(ptr::null(), ptr::null());
            dispatch_sync_f(q, num_ptr as *mut c_void, serial_queue_test_add);
            dispatch_release(q);
        }
        assert!(num == 1);
    }
}
