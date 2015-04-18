extern crate dispatch;

use std::process::exit;
use dispatch::Queue;

fn main() {
    let queue = Queue::main();
    queue.async(|| println!("Hello!"));
    queue.async(|| {
        println!("Goodbye!");
        exit(0);
    });

    dispatch::main();
}
