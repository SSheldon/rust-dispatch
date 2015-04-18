use libc::c_void;

#[allow(non_camel_case_types)]
pub type dispatch_queue_t = *mut c_void;

#[link(name = "System", kind = "dylib")]
extern {
    pub fn dispatch_get_main_queue() -> dispatch_queue_t;
    // dispatch_queue_t dispatch_get_global_queue ( long identifier, unsigned long flags );
    // dispatch_queue_t dispatch_queue_create ( const char *label, dispatch_queue_attr_t attr );
    // dispatch_queue_attr_t dispatch_queue_attr_make_with_qos_class ( dispatch_queue_attr_t attr, dispatch_qos_class_t qos_class, int relative_priority );
    // const char * dispatch_queue_get_label ( dispatch_queue_t queue );
    // void dispatch_set_target_queue ( dispatch_object_t object, dispatch_queue_t queue );
    pub fn dispatch_main();

    // void dispatch_async ( dispatch_queue_t queue, dispatch_block_t block );
    pub fn dispatch_async_f(queue: dispatch_queue_t, context: *mut c_void, work: extern fn());
    // void dispatch_sync ( dispatch_queue_t queue, dispatch_block_t block );
    pub fn dispatch_sync_f(queue: dispatch_queue_t, context: *mut c_void, work: extern fn());
    // void dispatch_after ( dispatch_time_t when, dispatch_queue_t queue, dispatch_block_t block );
    // void dispatch_after_f ( dispatch_time_t when, dispatch_queue_t queue, void *context, dispatch_function_t work );
    // void dispatch_apply ( size_t iterations, dispatch_queue_t queue, void (^block)(size_t) );
    // void dispatch_apply_f ( size_t iterations, dispatch_queue_t queue, void *context, void (*work)(void *, size_t) );
    // void dispatch_once ( dispatch_once_t *predicate, dispatch_block_t block );
    // void dispatch_once_f ( dispatch_once_t *predicate, void *context, dispatch_function_t function );
}
