#![cfg(feature = "invocation")]

use std::{
    sync::{Arc, Barrier, OnceLock},
    thread::spawn,
};

use jni::{
    objects::{IntoAutoLocal as _, JObject, JValue, Weak},
    sys::jint,
    Env,
};

mod util;
use util::{attach_current_thread, unwrap};

#[test]
pub fn weak_ref_works_in_other_threads() {
    const ITERS_PER_THREAD: usize = 10_000;

    attach_current_thread(|env| {
        let atomic_integer_local = unwrap(
            env.new_object(
                c"java/util/concurrent/atomic/AtomicInteger",
                c"(I)V",
                &[JValue::from(0)],
            ),
            env,
        )
        .auto();
        let atomic_integer_weak = unwrap(env.new_weak_ref(&atomic_integer_local), env);

        static ATOMIC_INT: OnceLock<Weak<JObject<'static>>> = OnceLock::new();
        ATOMIC_INT.set(atomic_integer_weak).unwrap();

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
                            let atomic_integer = unwrap(atomic_integer.upgrade_local(env), env)
                                .expect("AtomicInteger shouldn't have been GC'd yet")
                                .auto();
                            unwrap(
                                unwrap(
                                    env.call_method(
                                        &atomic_integer,
                                        c"incrementAndGet",
                                        c"()I",
                                        &[],
                                    ),
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

            let expected = (ITERS_PER_THREAD * thread_num) as jint;
            assert_eq!(
                expected,
                unwrap(
                    unwrap(
                        env.call_method(
                            &atomic_integer_local,
                            c"getAndSet",
                            c"(I)I",
                            &[JValue::from(0)]
                        ),
                        env,
                    )
                    .i(),
                    env,
                )
            );
        }

        Ok(())
    })
    .unwrap();
}

#[test]
fn weak_ref_is_actually_weak() {
    attach_current_thread(|env| {
        // This test uses `with_local_frame` to work around issue #109.

        fn run_gc(env: &mut Env) {
            unwrap(
                env.with_local_frame(1, |env| {
                    env.call_static_method(c"java/lang/System", c"gc", c"()V", &[])?;
                    Ok(())
                }),
                env,
            );
        }

        for _ in 0..100 {
            let obj_local = unwrap(
                env.with_local_frame_returning_local::<_, JObject, _>(2, |env| {
                    env.new_object(c"java/lang/Object", c"()V", &[])
                }),
                env,
            )
            .auto();

            let obj_weak = unwrap(env.new_weak_ref(&obj_local), env);

            let obj_weak2 =
                unwrap(obj_weak.clone_in_jvm(env), env).expect("weak ref should not be null");

            run_gc(env);

            for obj_weak in &[&obj_weak, &obj_weak2] {
                {
                    let obj_local_from_weak = unwrap(obj_weak.upgrade_local(env), env)
                        .expect("object shouldn't have been GC'd yet")
                        .auto();

                    assert!(!obj_local_from_weak.is_null());
                    assert!(env.is_same_object(&obj_local_from_weak, &obj_local));
                }

                {
                    let obj_global_from_weak = unwrap(obj_weak.upgrade_global(env), env)
                        .expect("object shouldn't have been GC'd yet");

                    assert!(!obj_global_from_weak.is_null());
                    assert!(env.is_same_object(&obj_global_from_weak, &obj_local));
                }

                assert!(env.is_same_object(obj_weak, &obj_local));

                assert!(
                    !obj_weak.is_garbage_collected(env),
                    "`is_garbage_collected` returned incorrect value"
                );
            }

            assert!(env.is_same_object(&obj_weak, &obj_weak2));

            drop(obj_local);
            run_gc(env);

            for obj_weak in &[&obj_weak, &obj_weak2] {
                {
                    let obj_local_from_weak = unwrap(obj_weak.upgrade_local(env), env);

                    assert!(
                        obj_local_from_weak.is_none(),
                        "object should have been GC'd"
                    );
                }

                {
                    let obj_global_from_weak = unwrap(obj_weak.upgrade_global(env), env);

                    assert!(
                        obj_global_from_weak.is_none(),
                        "object should have been GC'd"
                    );
                }

                assert!(
                    obj_weak.is_garbage_collected(env),
                    "`is_garbage_collected` returned incorrect value"
                );
            }

            assert!(env.is_same_object(obj_weak, &obj_weak2));
        }

        Ok(())
    })
    .unwrap();
}
