#![cfg(feature = "invocation")]

use std::{
    sync::{Arc, Barrier, OnceLock},
    thread::spawn,
};

use jni::{
    objects::{Global, IntoAutoLocal as _, JObject, JValue},
    sys::jint,
};

mod util;
use util::{attach_current_thread, unwrap};

#[test]
pub fn global_ref_works_in_other_threads() {
    const ITERS_PER_THREAD: usize = 10_000;

    let atomic_integer = attach_current_thread(|env| {
        let local_ref = unwrap(
            env.new_object(
                c"java/util/concurrent/atomic/AtomicInteger",
                c"(I)V",
                &[JValue::from(0)],
            ),
            env,
        )
        .auto();
        env.new_global_ref(&local_ref)
    })
    .unwrap();

    static ATOMIC_INT: OnceLock<Global<JObject<'static>>> = OnceLock::new();
    ATOMIC_INT.set(atomic_integer).unwrap();

    let mut join_handlers = Vec::new();

    // Test with a different number of threads (from 2 to 8)
    for thread_num in 2..9 {
        let barrier = Arc::new(Barrier::new(thread_num));

        for _ in 0..thread_num {
            let barrier = barrier.clone();
            let atomic_integer = ATOMIC_INT.get().unwrap();

            let jh = spawn(move || {
                attach_current_thread(|env| {
                    barrier.wait();
                    for _ in 0..ITERS_PER_THREAD {
                        unwrap(
                            unwrap(
                                env.call_method(atomic_integer, c"incrementAndGet", c"()I", &[]),
                                env,
                            )
                            .i(),
                            env,
                        );
                    }
                    Ok(())
                })
                .unwrap();
            });
            join_handlers.push(jh);
        }

        for jh in join_handlers.drain(..) {
            jh.join().unwrap();
        }

        attach_current_thread(|env| {
            let atomic_integer = ATOMIC_INT.get().unwrap();
            let expected = (ITERS_PER_THREAD * thread_num) as jint;
            assert_eq!(
                expected,
                unwrap(
                    unwrap(
                        env.call_method(atomic_integer, c"getAndSet", c"(I)I", &[JValue::from(0)]),
                        env,
                    )
                    .i(),
                    env,
                )
            );
            Ok(())
        })
        .unwrap();
    }
}
