#![cfg(feature = "invocation")]

use std::{
    sync::{Arc, Barrier, OnceLock},
    thread::spawn,
};

use jni::{
    objects::{AutoLocal, JObject, JValue, WeakRef},
    sys::jint,
    JNIEnv,
};

mod util;
use util::{attach_current_thread, unwrap};

#[test]
pub fn weak_ref_works_in_other_threads() {
    const ITERS_PER_THREAD: usize = 10_000;

    let mut env = attach_current_thread();
    let mut join_handlers = Vec::new();

    let atomic_integer_local = AutoLocal::new(
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
    let atomic_integer_weak = unwrap(env.new_weak_ref(&atomic_integer_local), &env);

    static ATOMIC_INT: OnceLock<WeakRef<JObject<'static>>> = OnceLock::new();
    ATOMIC_INT.set(atomic_integer_weak).unwrap();

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
                    let atomic_local = unwrap(atomic_integer.upgrade_local(&mut env), &env)
                        .expect("AtomicInteger shouldn't have been GC'd yet");
                    let atomic_integer = env.auto_local(atomic_local);
                    unwrap(
                        unwrap(
                            env.call_method(&atomic_integer, "incrementAndGet", "()I", &[]),
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

        let expected = (ITERS_PER_THREAD * thread_num) as jint;
        assert_eq!(
            expected,
            unwrap(
                unwrap(
                    env.call_method(
                        &atomic_integer_local,
                        "getAndSet",
                        "(I)I",
                        &[JValue::from(0)]
                    ),
                    &env,
                )
                .i(),
                &env,
            )
        );
    }
}

#[allow(deprecated)]
#[test]
fn weak_ref_is_actually_weak() {
    let mut env = attach_current_thread();

    // This test uses `with_local_frame` to work around issue #109.

    fn run_gc(env: &mut JNIEnv) {
        unwrap(
            env.with_local_frame(1, |env| {
                env.call_static_method("java/lang/System", "gc", "()V", &[])?;
                Ok(())
            }),
            env,
        );
    }

    for _ in 0..100 {
        let obj_local = unwrap(
            env.with_local_frame_returning_local(2, |env| {
                env.new_object("java/lang/Object", "()V", &[])
            }),
            &env,
        );
        let obj_local = env.auto_local(obj_local);

        let obj_weak = unwrap(env.new_weak_ref(&obj_local), &env);

        let obj_weak2 =
            unwrap(obj_weak.clone_in_jvm(&mut env), &env).expect("weak ref should not be null");

        run_gc(&mut env);

        for obj_weak in &[&obj_weak, &obj_weak2] {
            {
                let obj_local_from_weak = unwrap(obj_weak.upgrade_local(&mut env), &env)
                    .expect("object shouldn't have been GC'd yet");
                let obj_local_from_weak = env.auto_local(obj_local_from_weak);

                assert!(!obj_local_from_weak.is_null());
                assert!(env.is_same_object(&obj_local_from_weak, &obj_local));
            }

            {
                let obj_global_from_weak = unwrap(obj_weak.upgrade_global(&env), &env)
                    .expect("object shouldn't have been GC'd yet");

                assert!(!obj_global_from_weak.is_null());
                assert!(env.is_same_object(&obj_global_from_weak, &obj_local));
            }

            assert!(obj_weak.is_same_object(&env, &obj_local));

            assert!(
                !obj_weak.is_garbage_collected(&env),
                "`is_garbage_collected` returned incorrect value"
            );
        }

        assert!(obj_weak.is_weak_ref_to_same_object(&env, &obj_weak2));

        drop(obj_local);
        run_gc(&mut env);

        for obj_weak in &[&obj_weak, &obj_weak2] {
            {
                let obj_local_from_weak = unwrap(obj_weak.upgrade_local(&mut env), &env);

                assert!(
                    obj_local_from_weak.is_none(),
                    "object should have been GC'd"
                );
            }

            {
                let obj_global_from_weak = unwrap(obj_weak.upgrade_global(&env), &env);

                assert!(
                    obj_global_from_weak.is_none(),
                    "object should have been GC'd"
                );
            }

            assert!(
                obj_weak.is_garbage_collected(&env),
                "`is_garbage_collected` returned incorrect value"
            );
        }

        assert!(obj_weak.is_weak_ref_to_same_object(&env, &obj_weak2));
    }
}
