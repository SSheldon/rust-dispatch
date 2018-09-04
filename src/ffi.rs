#![allow(missing_docs, non_camel_case_types, improper_ctypes)]

use std::os::raw::{c_char, c_int, c_long, c_uint, c_ulong, c_void};

use block::{Block, BlockArguments, ConcreteBlock, IntoConcreteBlock};
use libc::{mode_t, off_t, timespec};

pub fn block<A, R, F>(closure: F) -> *const Block<A, R>
where
    A: BlockArguments,
    F: 'static + IntoConcreteBlock<A, Ret = R>,
{
    let block = ConcreteBlock::new(closure);
    let block = block.copy();
    let ptr = (&*block) as *const _;
    let ptr = unsafe { _Block_copy(ptr as *const c_void) };

    ptr as *const Block<A, R>
}

pub enum dispatch_object_s {}

pub type dispatch_block_t = *const Block<(), ()>;
pub type dispatch_function_t = extern "C" fn(*mut c_void);
pub type dispatch_semaphore_t = *mut dispatch_object_s;
pub type dispatch_group_t = *mut dispatch_object_s;
pub type dispatch_object_t = *mut dispatch_object_s;
pub type dispatch_once_t = c_long;
pub type dispatch_queue_t = *mut dispatch_object_s;
pub type dispatch_time_t = u64;
// dispatch_source_type_t
pub type dispatch_fd_t = c_int;
pub type dispatch_data_handler_t = *const Block<(dispatch_data_t, c_int), ()>;
pub type dispatch_data_t = *mut dispatch_object_s;
pub type dispatch_data_applier_t =
    *const Block<(dispatch_data_t, usize, *const c_void, usize), bool>;
pub type dispatch_io_t = *mut dispatch_object_s;
pub type dispatch_io_handler_t = *const Block<(bool, dispatch_data_t, c_int), ()>;
pub type dispatch_cleanup_handler_t = *const Block<(c_int,), ()>;
pub type dispatch_io_type_t = c_ulong;
pub type dispatch_io_close_flags_t = c_ulong;
pub type dispatch_io_interval_flags_t = c_ulong;
pub type dispatch_queue_attr_t = *const dispatch_object_s;
pub type dispatch_block_flags_t = c_ulong;
pub type dispatch_qos_class_t = c_uint;

#[cfg_attr(any(target_os = "macos", target_os = "ios"), link(name = "System", kind = "dylib"))]
#[cfg_attr(
    not(any(target_os = "macos", target_os = "ios")), link(name = "dispatch", kind = "dylib")
)]
extern "C" {
    static _dispatch_main_q: dispatch_object_s;
    static _dispatch_queue_attr_concurrent: dispatch_object_s;
    pub static _dispatch_data_destructor_free: dispatch_block_t;
    pub static _dispatch_data_destructor_munmap: dispatch_block_t;
    pub static _dispatch_data_empty: dispatch_object_s;

    pub fn _Block_copy(block: *const c_void) -> *mut c_void;
    pub fn _Block_release(block: *mut c_void);

    pub fn qos_class_self() -> dispatch_qos_class_t;
    pub fn qos_class_main() -> dispatch_qos_class_t;

    pub fn dispatch_queue_attr_make_initially_inactive(
        attr: dispatch_queue_attr_t,
    ) -> dispatch_queue_attr_t;
    pub fn dispatch_queue_attr_make_with_qos_class(
        attr: dispatch_queue_attr_t,
        qos_class: dispatch_qos_class_t,
        relative_priority: c_int,
    ) -> dispatch_queue_attr_t;

    pub fn dispatch_get_global_queue(identifier: c_long, flags: c_ulong) -> dispatch_queue_t;
    pub fn dispatch_queue_create(
        label: *const c_char,
        attr: dispatch_queue_attr_t,
    ) -> dispatch_queue_t;
    pub fn dispatch_queue_get_label(queue: dispatch_queue_t) -> *const c_char;
    pub fn dispatch_queue_get_qos_class(
        queue: dispatch_queue_t,
        relative_priority_ptr: *mut c_int,
    ) -> dispatch_qos_class_t;
    pub fn dispatch_set_target_queue(object: dispatch_object_t, queue: dispatch_queue_t);
    pub fn dispatch_main();

    pub fn dispatch_async(queue: dispatch_queue_t, block: dispatch_block_t);
    pub fn dispatch_async_f(
        queue: dispatch_queue_t,
        context: *mut c_void,
        work: dispatch_function_t,
    );
    pub fn dispatch_sync(queue: dispatch_queue_t, block: dispatch_block_t);
    pub fn dispatch_sync_f(
        queue: dispatch_queue_t,
        context: *mut c_void,
        work: dispatch_function_t,
    );
    pub fn dispatch_after(when: dispatch_time_t, queue: dispatch_queue_t, block: dispatch_block_t);
    pub fn dispatch_after_f(
        when: dispatch_time_t,
        queue: dispatch_queue_t,
        context: *mut c_void,
        work: dispatch_function_t,
    );
    pub fn dispatch_apply(
        iterations: usize,
        queue: dispatch_queue_t,
        block: *const Block<(usize,), ()>,
    );
    pub fn dispatch_apply_f(
        iterations: usize,
        queue: dispatch_queue_t,
        context: *mut c_void,
        work: extern "C" fn(*mut c_void, usize),
    );
    pub fn dispatch_once(predicate: *mut dispatch_once_t, block: dispatch_block_t);
    pub fn dispatch_once_f(
        predicate: *mut dispatch_once_t,
        context: *mut c_void,
        function: dispatch_function_t,
    );

    pub fn dispatch_group_async(
        group: dispatch_group_t,
        queue: dispatch_queue_t,
        block: dispatch_block_t,
    );
    pub fn dispatch_group_async_f(
        group: dispatch_group_t,
        queue: dispatch_queue_t,
        context: *mut c_void,
        work: dispatch_function_t,
    );
    pub fn dispatch_group_create() -> dispatch_group_t;
    pub fn dispatch_group_enter(group: dispatch_group_t);
    pub fn dispatch_group_leave(group: dispatch_group_t);
    pub fn dispatch_group_notify(
        group: dispatch_group_t,
        queue: dispatch_queue_t,
        block: dispatch_block_t,
    );
    pub fn dispatch_group_notify_f(
        group: dispatch_group_t,
        queue: dispatch_queue_t,
        context: *mut c_void,
        work: dispatch_function_t,
    );
    pub fn dispatch_group_wait(group: dispatch_group_t, timeout: dispatch_time_t) -> c_long;

    pub fn dispatch_get_context(object: dispatch_object_t) -> *mut c_void;
    pub fn dispatch_release(object: dispatch_object_t);
    pub fn dispatch_resume(object: dispatch_object_t);
    pub fn dispatch_retain(object: dispatch_object_t);
    pub fn dispatch_set_context(object: dispatch_object_t, context: *mut c_void);
    pub fn dispatch_set_finalizer_f(object: dispatch_object_t, finalizer: dispatch_function_t);
    pub fn dispatch_suspend(object: dispatch_object_t);

    pub fn dispatch_semaphore_create(value: c_long) -> dispatch_semaphore_t;
    pub fn dispatch_semaphore_signal(dsema: dispatch_semaphore_t) -> c_long;
    pub fn dispatch_semaphore_wait(dsema: dispatch_semaphore_t, timeout: dispatch_time_t)
        -> c_long;

    pub fn dispatch_barrier_async(queue: dispatch_queue_t, block: dispatch_block_t);
    pub fn dispatch_barrier_async_f(
        queue: dispatch_queue_t,
        context: *mut c_void,
        work: dispatch_function_t,
    );
    pub fn dispatch_barrier_sync(queue: dispatch_queue_t, block: dispatch_block_t);
    pub fn dispatch_barrier_sync_f(
        queue: dispatch_queue_t,
        context: *mut c_void,
        work: dispatch_function_t,
    );

    // void dispatch_source_cancel ( dispatch_source_t source );
    // dispatch_source_t dispatch_source_create ( dispatch_source_type_t type, uintptr_t handle, unsigned long mask, dispatch_queue_t queue );
    // unsigned long dispatch_source_get_data ( dispatch_source_t source );
    // uintptr_t dispatch_source_get_handle ( dispatch_source_t source );
    // unsigned long dispatch_source_get_mask ( dispatch_source_t source );
    // void dispatch_source_merge_data ( dispatch_source_t source, unsigned long value );
    // void dispatch_source_set_registration_handler ( dispatch_source_t source, dispatch_block_t handler );
    // void dispatch_source_set_registration_handler_f ( dispatch_source_t source, dispatch_function_t handler );
    // void dispatch_source_set_cancel_handler ( dispatch_source_t source, dispatch_block_t handler );
    // void dispatch_source_set_cancel_handler_f ( dispatch_source_t source, dispatch_function_t handler );
    // void dispatch_source_set_event_handler ( dispatch_source_t source, dispatch_block_t handler );
    // void dispatch_source_set_event_handler_f ( dispatch_source_t source, dispatch_function_t handler );
    // void dispatch_source_set_timer ( dispatch_source_t source, dispatch_time_t start, uint64_t interval, uint64_t leeway );
    // long dispatch_source_testcancel ( dispatch_source_t source );

    pub fn dispatch_read(
        fd: dispatch_fd_t,
        length: usize,
        queue: dispatch_queue_t,
        handler: dispatch_data_handler_t,
    );
    pub fn dispatch_write(
        fd: dispatch_fd_t,
        data: dispatch_data_t,
        queue: dispatch_queue_t,
        handler: dispatch_data_handler_t,
    );

    pub fn dispatch_io_create(
        io_type: dispatch_io_type_t,
        fd: dispatch_fd_t,
        queue: dispatch_queue_t,
        cleanup_handler: dispatch_cleanup_handler_t,
    ) -> dispatch_io_t;
    pub fn dispatch_io_create_with_path(
        io_type: dispatch_io_type_t,
        path: *const c_char,
        oflag: c_int,
        mode: mode_t,
        queue: dispatch_queue_t,
        cleanup_handler: dispatch_cleanup_handler_t,
    ) -> dispatch_io_t;
    pub fn dispatch_io_create_with_io(
        io_type: dispatch_io_type_t,
        io: dispatch_io_t,
        queue: dispatch_queue_t,
        cleanup_handler: dispatch_cleanup_handler_t,
    ) -> dispatch_io_t;

    pub fn dispatch_io_read(
        channel: dispatch_io_t,
        offset: off_t,
        length: usize,
        queue: dispatch_queue_t,
        io_handler: dispatch_io_handler_t,
    );
    pub fn dispatch_io_write(
        channel: dispatch_io_t,
        offset: off_t,
        data: dispatch_data_t,
        queue: dispatch_queue_t,
        io_handler: dispatch_io_handler_t,
    );
    pub fn dispatch_io_close(channel: dispatch_io_t, flags: dispatch_io_close_flags_t);
    pub fn dispatch_io_barrier(channel: dispatch_io_t, barrier: dispatch_block_t);
    pub fn dispatch_io_set_high_water(channel: dispatch_io_t, high_water: usize);
    pub fn dispatch_io_set_low_water(channel: dispatch_io_t, low_water: usize);
    pub fn dispatch_io_set_interval(
        channel: dispatch_io_t,
        interval: u64,
        flags: dispatch_io_interval_flags_t,
    );
    pub fn dispatch_io_get_descriptor(channel: dispatch_io_t) -> dispatch_fd_t;

    pub fn dispatch_data_create(
        buffer: *const c_void,
        size: usize,
        queue: dispatch_queue_t,
        destructor: dispatch_block_t,
    ) -> dispatch_data_t;
    pub fn dispatch_data_get_size(data: dispatch_data_t) -> usize;
    pub fn dispatch_data_create_map(
        data: dispatch_data_t,
        buffer_ptr: *const *mut c_void,
        size_ptr: *mut usize,
    ) -> dispatch_data_t;
    pub fn dispatch_data_create_concat(
        data1: dispatch_data_t,
        data2: dispatch_data_t,
    ) -> dispatch_data_t;
    pub fn dispatch_data_create_subrange(
        data: dispatch_data_t,
        offset: usize,
        length: usize,
    ) -> dispatch_data_t;
    pub fn dispatch_data_apply(data: dispatch_data_t, applier: dispatch_data_applier_t) -> bool;
    pub fn dispatch_data_copy_region(
        data: dispatch_data_t,
        location: usize,
        offset_ptr: *mut usize,
    ) -> dispatch_data_t;

    pub fn dispatch_time(when: dispatch_time_t, delta: i64) -> dispatch_time_t;
    pub fn dispatch_walltime(when: *const timespec, delta: i64) -> dispatch_time_t;

    // void dispatch_queue_set_specific ( dispatch_queue_t queue, const void *key, void *context, dispatch_function_t destructor );
    // void * dispatch_queue_get_specific ( dispatch_queue_t queue, const void *key );
    // void * dispatch_get_specific ( const void *key );

    pub fn dispatch_block_create(
        flags: dispatch_block_flags_t,
        block: dispatch_block_t,
    ) -> dispatch_block_t;
    pub fn dispatch_block_create_with_qos_class(
        flags: dispatch_block_flags_t,
        qos_class: dispatch_qos_class_t,
        relative_priority: c_int,
        block: dispatch_block_t,
    ) -> dispatch_block_t;
    pub fn dispatch_block_perform(flags: dispatch_block_flags_t, block: dispatch_block_t);
    pub fn dispatch_block_wait(block: dispatch_block_t, timeout: dispatch_time_t) -> c_long;
    pub fn dispatch_block_notify(
        block: dispatch_block_t,
        queue: dispatch_queue_t,
        notification_block: dispatch_block_t,
    );
    pub fn dispatch_block_cancel(block: dispatch_block_t);
    pub fn dispatch_block_testcancel(block: dispatch_block_t) -> c_long;
}

pub fn dispatch_get_main_queue() -> dispatch_queue_t {
    unsafe { &_dispatch_main_q as *const _ as dispatch_queue_t }
}

/// A QOS class which indicates work performed by this thread is interactive with the user.
pub const QOS_CLASS_USER_INTERACTIVE: dispatch_qos_class_t = 0x21;
/// A QOS class which indicates work performed by this thread was initiated by the user
/// and that the user is likely waiting for the results.
pub const QOS_CLASS_USER_INITIATED: dispatch_qos_class_t = 0x19;
/// A default QOS class used by the system in cases where more specific QOS class information is not available.
pub const QOS_CLASS_DEFAULT: dispatch_qos_class_t = 0x15;
/// A QOS class which indicates work performed by this thread may or may not be initiated by the user
/// and that the user is unlikely to be immediately waiting for the results.
pub const QOS_CLASS_UTILITY: dispatch_qos_class_t = 0x11;
/// A QOS class which indicates work performed by this thread was not initiated by the user
/// and that the user may be unaware of the results.
pub const QOS_CLASS_BACKGROUND: dispatch_qos_class_t = 0x09;
/// A QOS class value which indicates the absence or removal of QOS class information.
pub const QOS_CLASS_UNSPECIFIED: dispatch_qos_class_t = 0x00;

/// The queue executes blocks serially in FIFO order.
pub const DISPATCH_QUEUE_SERIAL: dispatch_queue_attr_t = 0 as dispatch_queue_attr_t;
/// The queue executes blocks concurrently.
pub static DISPATCH_QUEUE_CONCURRENT: &'static dispatch_object_s =
    unsafe { &_dispatch_queue_attr_concurrent };

/// Items dispatched to the queue will run at high priority,
/// i.e. the queue will be scheduled for execution
/// before any default priority or low priority queue.
pub const DISPATCH_QUEUE_PRIORITY_HIGH: c_long = 2;
/// Items dispatched to the queue will run at the default priority,
/// i.e. the queue will be scheduled for execution
/// after all high priority queues have been scheduled,
/// but before any low priority queues have been scheduled.
pub const DISPATCH_QUEUE_PRIORITY_DEFAULT: c_long = 0;
/// Items dispatched to the queue will run at low priority,
/// i.e. the queue will be scheduled for execution
/// after all default priority and high priority queues have been scheduled.
pub const DISPATCH_QUEUE_PRIORITY_LOW: c_long = -2;
/// Items dispatched to the queue will run at background priority,
/// i.e. the queue will be scheduled for execution
/// after all higher priority queues have been scheduled
/// and the system will run items on this queue on a thread
/// with background status as per setpriority(2)
/// (i.e. disk I/O is throttled and the thread's scheduling priority is set to lowest value).
pub const DISPATCH_QUEUE_PRIORITY_BACKGROUND: c_long = -1 << 15;

/// A `dispatch_time_t` corresponding to the current time.
pub const DISPATCH_TIME_NOW: dispatch_time_t = 0;
/// A `dispatch_time_t` corresponding to the maximum time.
pub const DISPATCH_TIME_FOREVER: dispatch_time_t = !0;

/// A dispatch I/O channel representing a stream of bytes.
pub const DISPATCH_IO_STREAM: dispatch_io_type_t = 0;
/// A dispatch I/O channel representing a random access file.
pub const DISPATCH_IO_RANDOM: dispatch_io_type_t = 1;

/// Stop outstanding operations on a channel when the channel is closed.
pub const DISPATCH_IO_STOP: dispatch_io_close_flags_t = 0x1;

/// Enqueue I/O handlers at a channel's interval setting
/// even if the amount of data ready to be delivered
/// is inferior to the low water mark (or zero).
pub const DISPATCH_IO_STRICT_INTERVAL: dispatch_io_interval_flags_t = 0x1;

/// Flag indicating that a dispatch block object should act as a barrier block
/// when submitted to a DISPATCH_QUEUE_CONCURRENT queue.
pub const DISPATCH_BLOCK_BARRIER: dispatch_block_flags_t = 0x1;
/// Flag indicating that a dispatch block object should execute disassociated
/// from current execution context attributes such as QOS class, os_activity_t
/// and properties of the current IPC request (if any).
pub const DISPATCH_BLOCK_DETACHED: dispatch_block_flags_t = 0x2;
/// Flag indicating that a dispatch block object should be assigned the execution
/// context attributes that are current at the time the block object is created.
pub const DISPATCH_BLOCK_ASSIGN_CURRENT: dispatch_block_flags_t = 0x4;
/// Flag indicating that a dispatch block object should be not be assigned a QOS class.
pub const DISPATCH_BLOCK_NO_QOS_CLASS: dispatch_block_flags_t = 0x8;
/// Flag indicating that execution of a dispatch block object submitted to
/// a queue should prefer the QOS class assigned to the queue over the QOS class
/// assigned to the block (resp. associated with the block at the time of submission).
pub const DISPATCH_BLOCK_INHERIT_QOS_CLASS: dispatch_block_flags_t = 0x10;
/// Flag indicating that execution of a dispatch block object submitted to
/// a queue should prefer the QOS class assigned to the block (resp. associated
/// with the block at the time of submission) over the QOS class assigned to
/// the queue, as long as doing so will not result in a lower QOS class.
pub const DISPATCH_BLOCK_ENFORCE_QOS_CLASS: dispatch_block_flags_t = 0x20;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_serial_queue() {
        use std::os::raw::c_void;
        use std::ptr;

        extern "C" fn serial_queue_test_add(num: *mut c_void) {
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
