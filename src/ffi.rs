#![allow(non_camel_case_types)]

use libc::{c_char, c_long, c_ulong, c_void, size_t};

pub type dispatch_function_t = extern fn(*mut c_void);
pub type dispatch_object_t = *mut ();
pub type dispatch_queue_t = *mut ();
pub type dispatch_queue_attr_t = *const ();

#[link(name = "System", kind = "dylib")]
extern {
    static _dispatch_queue_attr_concurrent: ();

    pub fn dispatch_get_main_queue() -> dispatch_queue_t;
    pub fn dispatch_get_global_queue(identifier: c_long, flags: c_ulong) -> dispatch_queue_t;
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
    pub fn dispatch_apply_f(iterations: size_t, queue: dispatch_queue_t, context: *mut c_void, work: extern fn(*mut c_void, size_t));
    // void dispatch_once ( dispatch_once_t *predicate, dispatch_block_t block );
    // void dispatch_once_f ( dispatch_once_t *predicate, void *context, dispatch_function_t function );

    // void dispatch_group_async ( dispatch_group_t group, dispatch_queue_t queue, dispatch_block_t block );
    // void dispatch_group_async_f ( dispatch_group_t group, dispatch_queue_t queue, void *context, dispatch_function_t work );
    // dispatch_group_t dispatch_group_create ( void );
    // void dispatch_group_enter ( dispatch_group_t group );
    // void dispatch_group_leave ( dispatch_group_t group );
    // void dispatch_group_notify ( dispatch_group_t group, dispatch_queue_t queue, dispatch_block_t block );
    // void dispatch_group_notify_f ( dispatch_group_t group, dispatch_queue_t queue, void *context, dispatch_function_t work );
    // long dispatch_group_wait ( dispatch_group_t group, dispatch_time_t timeout );

    // void * dispatch_get_context ( dispatch_object_t object );
    pub fn dispatch_release(object: dispatch_object_t);
    // void dispatch_resume ( dispatch_object_t object );
    pub fn dispatch_retain(object: dispatch_object_t);
    // void dispatch_set_context ( dispatch_object_t object, void *context );
    // void dispatch_set_finalizer_f ( dispatch_object_t object, dispatch_function_t finalizer );
    // void dispatch_suspend ( dispatch_object_t object );
}

pub const DISPATCH_QUEUE_SERIAL: dispatch_queue_attr_t = 0 as dispatch_queue_attr_t;
pub static DISPATCH_QUEUE_CONCURRENT: dispatch_queue_attr_t = &_dispatch_queue_attr_concurrent;

pub const DISPATCH_QUEUE_PRIORITY_HIGH: c_long       = 2;
pub const DISPATCH_QUEUE_PRIORITY_DEFAULT: c_long    = 0;
pub const DISPATCH_QUEUE_PRIORITY_LOW: c_long        = -2;
pub const DISPATCH_QUEUE_PRIORITY_BACKGROUND: c_long = -1 << 15;

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
            let q = dispatch_queue_create(ptr::null(), DISPATCH_QUEUE_SERIAL);
            dispatch_sync_f(q, num_ptr as *mut c_void, serial_queue_test_add);
            dispatch_release(q);
        }
        assert!(num == 1);
    }
}
