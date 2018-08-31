use std::ffi::CStr;
use std::io;
use std::os::raw::c_int;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::{AsRawFd, IntoRawFd};
use std::path::Path;
use std::ptr;
use std::time::Duration;

use libc::{mode_t, off_t};

use ffi::*;
use {Data, Queue};

#[derive(Clone, Debug)]
pub enum ChannelType {
    Stream,
    Random,
}

impl ChannelType {
    pub fn as_raw(&self) -> dispatch_io_type_t {
        match self {
            ChannelType::Stream => DISPATCH_IO_STREAM,
            ChannelType::Random => DISPATCH_IO_RANDOM,
        }
    }
}

pub type CleanupHandler = fn(i32);

pub struct Channel {
    ptr: dispatch_io_t,
}

impl Channel {
    /// Create a dispatch I/O channel associated with a file descriptor.
    ///
    /// The system takes control of the file descriptor until the channel is closed,
    /// an error occurs on the file descriptor or all references to the channel are released.
    /// At that time the specified cleanup handler will be enqueued
    /// and control over the file descriptor relinquished.
    pub fn open<F, H>(
        channel_type: ChannelType,
        f: F,
        queue: &Queue,
        cleanup_handler: Option<H>,
    ) -> Self
    where
        F: IntoRawFd,
        H: 'static + Fn(i32),
    {
        let fd = f.into_raw_fd() as dispatch_fd_t;

        let ptr = unsafe {
            dispatch_io_create(
                channel_type.as_raw(),
                fd,
                queue.ptr,
                cleanup_handler.map_or(ptr::null(), block),
            )
        };

        debug!(
            "create {:?} channel on queue `{}` with fd: {:?}, channel: {:?}",
            channel_type,
            queue.label(),
            fd,
            ptr
        );

        Channel { ptr }
    }

    /// Create a dispatch I/O channel associated with a path name.
    ///
    /// The specified path, oflag and mode parameters will be passed to open(2)
    /// when the first I/O operation on the channel is ready to execute
    /// and the resulting file descriptor will remain open
    /// and under the control of the system until the channel is closed,
    /// an error occurs on the file descriptor or all references to the channel are released.
    /// At that time the file descriptor will be closed
    /// and the specified cleanup handler will be enqueued.
    pub fn create<P, H>(
        channel_type: ChannelType,
        path: P,
        flags: c_int,
        mode: mode_t,
        queue: &Queue,
        cleanup_handler: Option<H>,
    ) -> Self
    where
        P: AsRef<Path>,
        H: 'static + Fn(i32),
    {
        let path = path.as_ref();
        let mut v = path.as_os_str().as_bytes().to_vec();
        v.push(0);
        let s = unsafe { CStr::from_bytes_with_nul_unchecked(&v) };
        let ptr = unsafe {
            dispatch_io_create_with_path(
                channel_type.as_raw(),
                s.as_ptr(),
                flags,
                mode,
                queue.ptr,
                cleanup_handler.map_or(ptr::null(), block),
            )
        };

        debug!(
            "create {:?} channel on queue `{}` with path name {:?}, channel: {:?}",
            channel_type,
            queue.label(),
            path,
            ptr
        );

        Channel { ptr }
    }

    /// Create a new stream dispatch I/O channel from an existing dispatch I/O channel.
    ///
    /// The new channel inherits the file descriptor or path name associated with the existing channel,
    /// but not its channel type or policies.
    pub fn open_stream<H>(&self, queue: &Queue, cleanup_handler: Option<H>) -> Self
    where
        H: 'static + Fn(i32),
    {
        let ptr = unsafe {
            dispatch_io_create_with_io(
                DISPATCH_IO_STREAM,
                self.ptr,
                queue.as_raw(),
                cleanup_handler.map_or(ptr::null(), block),
            )
        };

        debug!(
            "create stream channel on queue `{}` with fd: {:?}, channel: {:?}",
            queue.label(),
            self.descriptor(),
            ptr
        );

        Channel { ptr }
    }

    /// Create a new random dispatch I/O channel from an existing dispatch I/O channel.
    ///
    /// The new channel inherits the file descriptor or path name associated with the existing channel,
    /// but not its channel type or policies.
    pub fn open_file<H>(&self, queue: &Queue, cleanup_handler: Option<H>) -> Self
    where
        H: 'static + Fn(i32),
    {
        let ptr = unsafe {
            dispatch_io_create_with_io(
                DISPATCH_IO_RANDOM,
                self.ptr,
                queue.as_raw(),
                cleanup_handler.map_or(ptr::null(), block),
            )
        };

        debug!(
            "create random channel on queue `{}` with fd: {:?}, channel: {:?}",
            queue.label(),
            self.descriptor(),
            ptr,
        );

        Channel { ptr }
    }

    /// Schedule a read operation for asynchronous execution on the specified I/O channel.
    /// The I/O handler is enqueued one or more times depending on the general load of the system
    /// and the policy specified on the I/O channel.
    pub fn read<H>(&self, offset: isize, length: usize, queue: &Queue, io_handler: H)
    where
        H: 'static + Fn(io::Result<Data>),
    {
        debug!(
            "read {} bytes at offset {}, channel: {:?}",
            length, offset, self.ptr
        );

        let fd = self.descriptor();
        let handler = block(move |done: bool, ptr: dispatch_data_t, error: i32| {
            let result = if done {
                unsafe { dispatch_retain(ptr) };

                let data = Data::borrow(ptr);

                trace!("read {} bytes with fd: {:?}", data.len(), fd);

                Ok(data)
            } else {
                trace!("read failed, error: {}", error);

                Err(io::Error::from_raw_os_error(error))
            };

            io_handler(result)
        });

        unsafe { dispatch_io_read(self.ptr, offset as off_t, length, queue.as_raw(), handler) }
    }

    /// Schedule a write operation for asynchronous execution on the specified I/O channel.
    /// The I/O handler is enqueued one or more times depending on the general load of the system
    /// and the policy specified on the I/O channel.
    pub fn write<H>(&self, offset: isize, data: &Data, queue: &Queue, io_handler: H)
    where
        H: 'static + Fn(io::Result<Data>),
    {
        debug!(
            "write {} bytes at offset {}, channel: {:?}",
            data.len(),
            offset,
            self.ptr
        );

        let fd = self.descriptor();
        let handler = block(move |done: bool, ptr: dispatch_data_t, error: i32| {
            let result = if done {
                let data = Data::borrow(ptr);

                trace!(
                    "write finished with fd: {:?}, remaining {} bytes",
                    fd,
                    data.len()
                );

                Ok(data)
            } else {
                trace!("write failed, error: {}", error);

                Err(io::Error::from_raw_os_error(error))
            };

            io_handler(result)
        });

        unsafe {
            dispatch_io_write(
                self.ptr,
                offset as off_t,
                data.as_raw(),
                queue.as_raw(),
                handler,
            )
        }
    }

    /// Close the specified I/O channel to new read or write operations;
    /// scheduling operations on a closed channel results in their handler returning an error.
    pub fn close(&self, flags: dispatch_io_close_flags_t) {
        debug!("close channel: {:?}", self.ptr);

        unsafe { dispatch_io_close(self.ptr, flags) }
    }

    /// Schedule a barrier operation on the specified I/O channel;
    /// all previously scheduled operations on the channel will complete
    /// before the provided barrier block is enqueued onto the global queue
    /// determined by the channel's target queue,
    /// and no subsequently scheduled operations will start
    /// until the barrier block has returned.
    pub fn barrier_async<F>(&self, barrier: F)
    where
        F: 'static + Fn(),
    {
        debug!("schedule barrier, channel: {:?}", self.ptr);

        unsafe { dispatch_io_barrier(self.ptr, block(barrier)) }
    }

    /// Returns the file descriptor underlying a dispatch I/O channel.
    pub fn descriptor(&self) -> dispatch_fd_t {
        unsafe { dispatch_io_get_descriptor(self.ptr) }
    }

    /// Set a high water mark on the I/O channel for all operations.
    pub fn set_high_water(&self, high_water: usize) {
        debug!(
            "set high water mark to {}, channel: {:?}",
            high_water, self.ptr
        );

        unsafe { dispatch_io_set_high_water(self.ptr, high_water) }
    }

    /// Set a low water mark on the I/O channel for all operations.
    pub fn set_low_water(&self, low_water: usize) {
        debug!(
            "set low water mark to {}, channel: {:?}",
            low_water, self.ptr
        );

        unsafe { dispatch_io_set_low_water(self.ptr, low_water) }
    }

    /// Set a interval at which I/O handlers are to be enqueued
    /// on the I/O channel for all operations.
    pub fn set_interval(&self, internal: Duration, flags: dispatch_io_interval_flags_t) {
        debug!("set internal to {:?}, channel: {:?}", internal, self.ptr);

        unsafe {
            dispatch_io_set_interval(
                self.ptr,
                internal
                    .as_secs()
                    .checked_mul(1_000_000_000)
                    .and_then(|dur| dur.checked_add(internal.subsec_nanos() as u64))
                    .unwrap_or(u64::max_value()),
                flags,
            )
        }
    }
}

unsafe impl Sync for Channel {}
unsafe impl Send for Channel {}

impl Clone for Channel {
    fn clone(&self) -> Self {
        unsafe {
            dispatch_retain(self.ptr);
        }
        Channel { ptr: self.ptr }
    }
}

impl Drop for Channel {
    fn drop(&mut self) {
        unsafe {
            dispatch_release(self.ptr);
        }
    }
}

impl Queue {
    /// A dispatch I/O channel representing a stream of bytes.
    ///
    /// Read and write operations on a channel of this type
    /// are performed serially (in order of creation) and
    /// read/write data at the file pointer position
    /// that is current at the time the operation starts executing.
    /// Operations of different type (read vs. write) may be performed simultaneously.
    /// Offsets passed to operations on a channel of this type are ignored.
    pub fn open_stream<F, H>(&self, f: F, cleanup_handler: Option<H>) -> Channel
    where
        F: IntoRawFd,
        H: 'static + Fn(i32),
    {
        Channel::open(ChannelType::Stream, f, self, cleanup_handler)
    }

    /// A dispatch I/O channel representing a random access file.
    ///
    /// Read and write operations on a channel of this type
    /// may be performed concurrently and read/write data at the specified offset.
    /// Offsets are interpreted relative to the file pointer
    /// position current at the time the I/O channel is created.
    /// Attempting to create a channel of this type for a file descriptor
    /// that is not seekable will result in an error.
    pub fn open_file<F, H>(&self, f: F, cleanup_handler: Option<H>) -> Channel
    where
        F: IntoRawFd,
        H: 'static + Fn(i32),
    {
        Channel::open(ChannelType::Random, f, self, cleanup_handler)
    }

    /// Schedule a read operation for asynchronous execution on the specified file descriptor.
    /// The specified handler is enqueued with the data read from the file descriptor
    /// when the operation has completed or an error occurs.
    pub fn read<F, H>(&self, f: &F, length: usize, handler: H)
    where
        F: AsRawFd,
        H: 'static + Fn(io::Result<Data>),
    {
        let fd = f.as_raw_fd() as dispatch_fd_t;

        debug!("read {} bytes with fd: {:?}", length, fd);

        let handler = block(move |ptr: dispatch_data_t, error: i32| {
            let result = if error == 0 {
                let data = Data::borrow(ptr);

                trace!("read {} bytes with fd: {:?}", data.len(), fd);

                Ok(data)
            } else {
                trace!("read failed, error: {}", error);

                Err(io::Error::from_raw_os_error(error))
            };

            handler(result)
        });

        unsafe { dispatch_read(fd, length, self.as_raw(), handler) }
    }

    /// Schedule a write operation for asynchronous execution on the specified file descriptor.
    /// The specified handler is enqueued when the operation has completed or an error occurs.
    pub fn write<F, H>(&self, f: &F, data: &Data, handler: H)
    where
        F: AsRawFd,
        H: 'static + Fn(io::Result<Data>),
    {
        let fd = f.as_raw_fd() as dispatch_fd_t;

        debug!("write {} bytes with fd: {:?}", data.len(), fd);

        let handler = block(move |ptr: dispatch_data_t, error: i32| {
            let result = if error == 0 {
                let data = Data::borrow(ptr);

                trace!(
                    "write finished with fd: {:?}, remaining {} bytes",
                    fd,
                    data.len()
                );

                Ok(data)
            } else {
                trace!("write failed, error: {}", error);

                Err(io::Error::from_raw_os_error(error))
            };

            handler(result)
        });

        unsafe { dispatch_write(fd, data.as_raw(), self.as_raw(), handler) }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Seek, SeekFrom, Write};
    use std::mem;
    use std::sync::{Arc, Barrier};

    use pretty_env_logger;
    use tempfile::tempfile;

    use super::*;
    use QueueAttribute;

    #[test]
    fn test_stream_channel() {
        let _ = pretty_env_logger::try_init();

        let f = tempfile().unwrap();
        let q = Queue::create("test", QueueAttribute::Serial);
        let c = q.open_file(f, Some(|error| trace!("cleanup channel, error: {}", error)));

        let data = q.data(b"hello world");

        c.write(0, &data, &q, move |result| {
            assert!(result.unwrap().is_empty());
        });

        c.read(0, 5, &q, move |result| match result {
            Ok(data) => {
                assert_eq!(data.len(), 5);
            }
            Err(err) => warn!("read channel failed, {}", err),
        });

        let barrier = Arc::new(Barrier::new(2));
        let b = barrier.clone();
        c.barrier_async(move || {
            trace!("force sync up channel");

            b.wait();
        });

        barrier.wait();

        trace!("sync up channel finished");

        c.close(0);
    }

    #[test]
    fn test_stream_operations() {
        let _ = pretty_env_logger::try_init();

        let mut f = tempfile().unwrap();
        let q = Queue::create("test", QueueAttribute::Concurrent);
        f.write_all(b"hello world").unwrap();
        f.seek(SeekFrom::Start(0)).unwrap();
        f.sync_all().unwrap();

        let barrier = Arc::new(Barrier::new(2));
        let b = barrier.clone();
        q.read(&f, 5, move |result| {
            match result {
                Ok(data) => {
                    assert_eq!(data.len(), 5);
                }
                Err(err) => warn!("read file failed, {}", err),
            }

            b.wait();
        });

        q.barrier_async(move || {
            trace!("force sync up queue");
        });

        barrier.wait();

        trace!("sync up queue finished");

        mem::drop(q);
    }
}
