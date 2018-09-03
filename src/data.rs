use std::ops::Add;
use std::os::raw::c_void;
use std::ptr;
use std::slice;

use ffi::*;
use Queue;

/// The destructor responsible for freeing the data when it is no longer needed.
pub trait Destructor {
    /// Extracts the raw `dispatch_block_t`.
    fn as_raw(self) -> dispatch_block_t;
}

impl Destructor for dispatch_block_t {
    fn as_raw(self) -> dispatch_block_t {
        self
    }
}

impl<F: 'static + Fn()> Destructor for F {
    fn as_raw(self) -> dispatch_block_t {
        block(self)
    }
}

/// The default destructor for dispatch data objects.
/// Used at data object creation to indicate that the supplied buffer
/// should be copied into internal storage managed by the system.
pub fn dispatch_data_destructor_default() -> dispatch_block_t {
    ptr::null()
}

/// The destructor for dispatch data objects created from a malloc'd buffer.
/// Used at data object creation to indicate that the supplied buffer
/// was allocated by the malloc() family and should be destroyed with free(3).
pub fn dispatch_data_destructor_free() -> dispatch_block_t {
    unsafe { _dispatch_data_destructor_free }
}

/// The destructor for dispatch data objects that have been created
/// from buffers that require deallocation with munmap(2).
pub fn dispatch_data_destructor_munmap() -> dispatch_block_t {
    unsafe { _dispatch_data_destructor_free }
}

/// An immutable object representing a contiguous or sparse region of memory.
#[derive(Debug)]
pub struct Data {
    ptr: dispatch_data_t,
}

impl Data {
    pub(crate) fn borrow(ptr: dispatch_data_t) -> Data {
        if ptr.is_null() {
            Data::empty()
        } else {
            unsafe { dispatch_retain(ptr) };

            Data { ptr }
        }
    }

    /// Creates a new dispatch data object with the specified memory buffer.
    pub fn create(queue: &Queue, buf: &[u8]) -> Self {
        Self::create_with_destructor(queue, buf.as_ptr() as *const c_void, buf.len(), ptr::null())
    }

    /// Creates a new dispatch data object with the specified memory buffer and destructor.
    pub fn create_with_destructor<F: Destructor>(
        queue: &Queue,
        buffer: *const c_void,
        size: usize,
        destructor: F,
    ) -> Self {
        let ptr =
            unsafe { dispatch_data_create(buffer, size, queue.as_raw(), destructor.as_raw()) };

        debug!("create data with {} bytes, data: {:?}", size, ptr);

        Data { ptr }
    }

    /// Extracts the raw `dispatch_data_t`.
    pub fn as_raw(&self) -> dispatch_data_t {
        self.ptr
    }

    /// The singleton dispatch data object representing a zero-length memory region.
    pub fn empty() -> Self {
        Data {
            ptr: unsafe { &_dispatch_data_empty as *const dispatch_object_s as dispatch_data_t },
        }
    }

    /// Returns the logical size of the memory managed by a dispatch data object
    pub fn len(&self) -> usize {
        unsafe { dispatch_data_get_size(self.ptr) }
    }

    /// Returns `true` if the data has a length of 0.
    pub fn is_empty(&self) -> bool {
        self.ptr == unsafe { &_dispatch_data_empty as *const dispatch_object_s as dispatch_data_t }
            || self.len() == 0
    }

    /// Returns a new dispatch data object containing a contiguous representation of the specified object’s memory.
    pub fn map(&self) -> (Self, &[u8]) {
        let mut buf = ptr::null_mut();
        let mut len = 0;

        let ptr = unsafe { dispatch_data_create_map(self.ptr, &mut buf, &mut len) };
        let data = Data { ptr };
        let buf = unsafe { slice::from_raw_parts(buf as *const u8, len) };

        debug!(
            "map data to {} bytes contiguous memory, data: {:?}",
            buf.len(),
            ptr
        );

        (data, buf)
    }

    /// Returns a new dispatch data object consisting of the concatenated data from two other data objects.
    pub fn concat(&self, other: &Data) -> Self {
        let ptr = unsafe { dispatch_data_create_concat(self.ptr, other.ptr) };

        debug!(
            "concat data: {:?} with data: {:?} to data: {:?}",
            self.ptr, other.ptr, ptr
        );

        Data { ptr }
    }

    /// Returns a new dispatch data object whose contents consist of a portion of another object’s memory region.
    pub fn subrange(&self, offset: usize, length: usize) -> Self {
        let ptr = unsafe { dispatch_data_create_subrange(self.ptr, offset, length) };

        Data { ptr }
    }

    /// Traverses the memory of a dispatch data object and executes custom code on each region.
    pub fn apply<F>(&self, applier: F) -> bool
    where
        F: 'static + Fn(&Self, usize, &[u8]) -> bool,
    {
        let data = self.clone();
        let applier = block(
            move |ptr: dispatch_data_t, offset: usize, buffer: *const c_void, size: usize| {
                assert_eq!(data.ptr, ptr);

                let buf = unsafe { slice::from_raw_parts(buffer as *const u8, size) };

                applier(&data, offset, buf)
            },
        );

        unsafe { dispatch_data_apply(self.ptr, applier) }
    }

    /// Returns a data object containing a portion of the data in another data object.
    pub fn copy_region(&self, location: usize) -> (Self, usize) {
        let mut p = 0;

        let ptr = unsafe { dispatch_data_copy_region(self.ptr, location, &mut p) };
        let data = Data { ptr };

        (data, p)
    }
}

impl From<dispatch_data_t> for Data {
    fn from(ptr: dispatch_data_t) -> Self {
        if ptr.is_null() {
            Data::empty()
        } else {
            Data { ptr }
        }
    }
}

unsafe impl Sync for Data {}
unsafe impl Send for Data {}

impl Clone for Data {
    fn clone(&self) -> Self {
        unsafe {
            dispatch_retain(self.ptr);
        }
        Data { ptr: self.ptr }
    }
}

impl Drop for Data {
    fn drop(&mut self) {
        unsafe {
            dispatch_release(self.ptr);
        }
    }
}

impl Add<Self> for Data {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.concat(&rhs)
    }
}

impl Queue {
    /// Returns an immutable object representing a contiguous or sparse region of memory.
    pub fn data(&self, buf: &[u8]) -> Data {
        Data::create(self, buf)
    }
}

#[cfg(test)]
mod tests {
    use pretty_env_logger;

    use std::cell::Cell;
    use std::mem;
    use std::sync::Arc;

    use libc;

    use super::*;
    use QueueAttribute;

    #[test]
    fn test_data() {
        let queue = Queue::main();

        assert!(!Data::empty().ptr.is_null());
        assert_eq!(Data::empty().len(), 0);

        let data = queue.data(b"hello world");

        assert_eq!(data.len(), 11);

        let data = queue.data(b"hello") + queue.data(b"world");

        assert_eq!(data.len(), 10);

        let (left, off) = data.copy_region(2);

        assert_eq!(left.len(), 5);
        assert_eq!(off, 0);

        let (right, off) = data.copy_region(8);

        assert_eq!(right.len(), 5);
        assert_eq!(off, 5);

        let (data, buf) = data.map();

        assert_eq!(data.len(), 10);
        assert_eq!(buf, b"helloworld");

        let data = data.subrange(3, 5);

        assert_eq!(data.len(), 5);

        let (data, buf) = data.map();

        assert_eq!(data.len(), 5);
        assert_eq!(buf, b"lowor");

        let n = Arc::new(Cell::new(0));
        let c = n.clone();

        assert!(data.apply(move |_data: &Data, offset: usize, buf: &[u8]| {
            assert_eq!(offset, 0);
            assert_eq!(buf, b"lowor");

            c.set(buf.len());

            true
        }));

        assert_eq!(n.get(), 5);
    }

    #[test]
    fn test_free_destructor() {
        let queue = Queue::main();
        let p = unsafe { libc::malloc(8) } as *const c_void;

        let data = Data::create_with_destructor(&queue, p, 8, dispatch_data_destructor_free());

        assert_eq!(data.len(), 8);

        mem::drop(data);
    }

    #[test]
    fn test_func_destructor() {
        let _ = pretty_env_logger::try_init();

        let q = Queue::create("test", QueueAttribute::Serial);
        let buf = "foo";

        let data =
            Data::create_with_destructor(&q, buf.as_ptr() as *const c_void, buf.len(), move || {
                trace!("data destructed");
            });

        assert_eq!(data.len(), 3);
    }
}
