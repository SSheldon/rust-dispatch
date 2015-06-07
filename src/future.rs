use std::cell::UnsafeCell;
use std::sync::Arc;

use {Group, GroupGuard, Queue};

struct FutureCell<T>(UnsafeCell<Option<T>>);

impl<T> FutureCell<T> {
    fn new() -> FutureCell<T> {
        FutureCell(UnsafeCell::new(None))
    }

    unsafe fn take(&self) -> Option<T> {
        (*self.0.get()).take()
    }

    unsafe fn get(&self) -> T {
        self.take().unwrap()
    }

    unsafe fn set(&self, value: T) {
        *self.0.get() = Some(value);
    }
}

// This is a lie, but we'll ensure that & methods are never called concurrently
unsafe impl<T: Send> Sync for FutureCell<T> { }

struct Promise<T> {
    result: Arc<FutureCell<T>>,
    _guard: GroupGuard,
}

impl<T: 'static + Send> Promise<T> {
    pub fn fulfill(self, value: T) {
        unsafe {
            // Since the group is entered, we're guaranteed that no one
            // is trying to get the value so it's safe to set
            self.result.set(value);
        }
    }
}

pub struct Future<T> {
    value: Arc<FutureCell<T>>,
    group: Group,
}

impl<T: 'static + Send> Future<T> {
    pub fn new<F>(queue: &Queue, work: F) -> Future<T>
            where F: 'static + Send + FnOnce() -> T {
        let (promise, future) = future();
        queue.async(move || {
            promise.fulfill(work());
        });
        future
    }

    pub fn wait(self) -> T {
        self.group.wait();
        unsafe {
            // Since the group is empty, we're guaranteed that the value has
            // finished being set
            self.value.get()
        }
    }

    fn notify<F>(self, queue: &Queue, work: F)
            where F: 'static + Send + FnOnce(T) {
        let Future { value, group } = self;
        group.notify(queue, move || {
            // Since the original group is being notified, the group is empty
            // and we're guaranteed that the value has finished being set
            let value = unsafe { value.take() };
            if let Some(input) = value {
                work(input);
            }
        });
    }

    pub fn map<U, F>(self, queue: &Queue, work: F) -> Future<U>
            where F: 'static + Send + FnOnce(T) -> U, U: 'static + Send {
        let (promise, future) = future();
        self.notify(queue, move |input| {
            promise.fulfill(work(input));
        });
        future
    }
}

fn future<T: 'static + Send>() -> (Promise<T>, Future<T>) {
    let future = Future {
        value: Arc::new(FutureCell::new()),
        group: Group::create(),
    };

    let promise = Promise {
        result: future.value.clone(),
        _guard: future.group.enter(),
    };

    (promise, future)
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
