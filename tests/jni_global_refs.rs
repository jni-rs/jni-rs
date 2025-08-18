#![cfg(feature = "invocation")]

use std::{
    sync::{Arc, Barrier, OnceLock},
    thread::spawn,
};

use jni::{
    objects::{AutoLocal, GlobalRef, JObject, JValue},
    sys::jint,
};

mod util;
use util::{attach_current_thread, unwrap};

#[test]
pub fn global_ref_works_in_other_threads() {
    const ITERS_PER_THREAD: usize = 10_000;

    let mut env = attach_current_thread();
    let mut join_handlers = Vec::new();

    let atomic_integer = {
        let local_ref = AutoLocal::new(
            unwrap(
                env.new_object(
                    "java/util/concurrent/atomic/AtomicInteger",
                    "(I)V",
                    &[JValue::from(0)],
                ),
                &env,
            ),
            &env,
        );
        unwrap(env.new_global_ref(&local_ref), &env)
    };

    static ATOMIC_INT: OnceLock<GlobalRef<JObject<'static>>> = OnceLock::new();
    ATOMIC_INT.set(atomic_integer).unwrap();

    // Test with a different number of threads (from 2 to 8)
    for thread_num in 2..9 {
        let barrier = Arc::new(Barrier::new(thread_num));

        for _ in 0..thread_num {
            let barrier = barrier.clone();
            let atomic_integer = ATOMIC_INT.get().unwrap();

            let jh = spawn(move || {
                let mut env = attach_current_thread();
                barrier.wait();
                for _ in 0..ITERS_PER_THREAD {
                    unwrap(
                        unwrap(
                            env.call_method(atomic_integer, "incrementAndGet", "()I", &[]),
                            &env,
                        )
                        .i(),
                        &env,
                    );
                }
            });
            join_handlers.push(jh);
        }

        for jh in join_handlers.drain(..) {
            jh.join().unwrap();
        }

        let atomic_integer = ATOMIC_INT.get().unwrap();
        let expected = (ITERS_PER_THREAD * thread_num) as jint;
        assert_eq!(
            expected,
            unwrap(
                unwrap(
                    env.call_method(atomic_integer, "getAndSet", "(I)I", &[JValue::from(0)]),
                    &env,
                )
                .i(),
                &env,
            )
        );
    }
}
