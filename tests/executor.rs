#![cfg(feature = "invocation")]

use std::{
    sync::{Arc, Barrier},
    thread::spawn,
};

use jni::{sys::jint, Executor};

mod util;
use util::{jvm, AtomicIntegerProxy};

#[test]
fn single_thread() {
    let executor = Executor::new(jvm().clone());
    test_single_thread(executor);
}

#[test]
fn serialized_threads() {
    let executor = Executor::new(jvm().clone());
    test_serialized_threads(executor);
}

#[test]
fn concurrent_threads() {
    let executor = Executor::new(jvm().clone());
    const THREAD_NUM: usize = 8;
    test_concurrent_threads(executor, THREAD_NUM)
}

fn test_single_thread(executor: Executor) {
    let mut atomic = AtomicIntegerProxy::new(executor, 0).unwrap();
    assert_eq!(0, atomic.get().unwrap());
    assert_eq!(1, atomic.increment_and_get().unwrap());
    assert_eq!(3, atomic.add_and_get(2).unwrap());
    assert_eq!(3, atomic.get().unwrap());
}

fn test_serialized_threads(executor: Executor) {
    let mut atomic = AtomicIntegerProxy::new(executor, 0).unwrap();
    assert_eq!(0, atomic.get().unwrap());
    let jh = spawn(move || {
        assert_eq!(1, atomic.increment_and_get().unwrap());
        assert_eq!(3, atomic.add_and_get(2).unwrap());
        atomic
    });
    let mut atomic = jh.join().unwrap();
    assert_eq!(3, atomic.get().unwrap());
}

fn test_concurrent_threads(executor: Executor, thread_num: usize) {
    const ITERS_PER_THREAD: usize = 10_000;

    let mut atomic = AtomicIntegerProxy::new(executor, 0).unwrap();
    let barrier = Arc::new(Barrier::new(thread_num));
    let mut threads = Vec::new();

    for _ in 0..thread_num {
        let barrier = Arc::clone(&barrier);
        let mut atomic = atomic.clone();
        let jh = spawn(move || {
            barrier.wait();
            for _ in 0..ITERS_PER_THREAD {
                atomic.increment_and_get().unwrap();
            }
        });
        threads.push(jh);
    }
    for jh in threads {
        jh.join().unwrap();
    }
    let expected = (ITERS_PER_THREAD * thread_num) as jint;
    assert_eq!(expected, atomic.get().unwrap());
}
