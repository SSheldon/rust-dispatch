use std::cell::UnsafeCell;
use std::sync::Arc;

use {Queue, QueueAttribute};

struct UnsafeSyncCell<T>(UnsafeCell<T>);

impl<T> UnsafeSyncCell<T> {
    fn new(value: T) -> UnsafeSyncCell<T> {
        UnsafeSyncCell(UnsafeCell::new(value))
    }

    unsafe fn get(&self) -> &T {
        &*self.0.get()
    }

    unsafe fn get_mut(&self) -> &mut T {
        &mut *self.0.get()
    }
}

unsafe impl<T> Sync for UnsafeSyncCell<T> { }

#[derive(Clone)]
pub struct ProxyQueue<T> {
    queue: Queue,
    value: Arc<UnsafeSyncCell<T>>,
}

impl<T: 'static + Send> ProxyQueue<T> {
    pub fn new(value: T) -> ProxyQueue<T> {
        ProxyQueue {
            queue: Queue::create("", QueueAttribute::Concurrent),
            value: Arc::new(UnsafeSyncCell::new(value)),
        }
    }

    pub fn exec_mut<F>(&self, work: F)
            where F: 'static + Send + FnOnce(&mut T) {
        let value = self.value.clone();
        self.queue.barrier_async(move || unsafe {
            // Safe to get mut since no other blocks will be executing
            work(value.get_mut());
        })
    }
}

impl<T: 'static + Send + Sync> ProxyQueue<T> {
    pub fn exec<F>(&self, work: F)
            where F: 'static + Send + FnOnce(&T) {
        let value = self.value.clone();
        self.queue.async(move || unsafe {
            // Safe to get since our value is Sync
            work(value.get());
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::channel;
    use super::*;

    fn test_proxy() {
        let proxy = ProxyQueue::new("Hello, world!".to_string());

        proxy.exec_mut(|s| {
            s.push_str(" Bye.")
        });

        let (send, recv) = channel();
        proxy.exec(move |s| {
            send.send(s.clone()).unwrap();
        });

        let result = recv.recv().unwrap();
        assert!(result == "Hello, world! Bye.");
    }
}
