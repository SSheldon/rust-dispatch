use std::cell::UnsafeCell;
use std::sync::Arc;

use {Group, Queue};

struct FutureCell<T>(UnsafeCell<Option<T>>);

impl<T> FutureCell<T> {
    fn new() -> FutureCell<T> {
        FutureCell(UnsafeCell::new(None))
    }

    unsafe fn get(&self) -> T {
        (*self.0.get()).take().unwrap()
    }

    unsafe fn set(&self, value: T) {
        *self.0.get() = Some(value);
    }
}

// This is a lie, but we'll ensure that & methods are never called concurrently
unsafe impl<T: Send> Sync for FutureCell<T> { }

pub struct Future<T> {
    group: Group,
    value: Arc<FutureCell<T>>,
}

impl<T: 'static + Send> Future<T> {
    pub fn new<F>(queue: &Queue, work: F) -> Future<T>
            where F: 'static + Send + FnOnce() -> T {
        let value = Arc::new(FutureCell::new());
        let group = Group::create();

        let result = value.clone();
        group.async(queue, move || unsafe {
            // Since the group is entered here, we're guaranteed that no one
            // is trying to get the value so it's safe to set
            result.set(work());
        });

        Future { group: group, value: value }
    }

    pub fn wait(self) -> T {
        self.group.wait();
        unsafe {
            // Since the group is empty, we're guaranteed that the value has
            // finished being set
            self.value.get()
        }
    }

    pub fn map<U, F>(self, queue: &Queue, work: F) -> Future<U>
            where F: 'static + Send + FnOnce(T) -> U, U: 'static + Send {
        let value = Arc::new(FutureCell::new());
        let group = Group::create();

        let guard = group.enter();
        let result = value.clone();
        let input = self.value.clone();
        self.group.notify(queue, move || unsafe {
            // Since the original group is being notified, the group is empty
            // and we're guaranteed that the value has finished being set
            let input = input.get();
            // Since the new group is entered here, we're guaranteed that no
            // one is trying to get the value so it's safe to set
            result.set(work(input));
            drop(guard);
        });

        Future { group: group, value: value }
    }
}

#[cfg(test)]
mod tests {
    use {Queue, QueueAttribute};
    use super::*;

    #[test]
    fn test_wait() {
        let q = Queue::create("", QueueAttribute::Concurrent);
        let future = Future::new(&q, || "Hello, world!".to_string());
        assert!(future.wait() == "Hello, world!");
    }

    #[test]
    fn test_map() {
        let q = Queue::create("", QueueAttribute::Concurrent);
        let future = Future::new(&q, || "Hello, world!".to_string());
        let future = future.map(&q, |s| s.len());
        assert!(future.wait() == "Hello, world!".len());
    }
}
