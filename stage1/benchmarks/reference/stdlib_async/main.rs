use std::sync::mpsc;
use std::thread;

fn compute(value: i32) -> i32 {
    value * 7
}

fn main() {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        tx.send(compute(6)).unwrap();
    });
    println!("{}", rx.recv().unwrap());
}
