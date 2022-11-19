#![cfg(feature = "invocation")]

use std::{
    sync::{Arc, Barrier},
    thread::spawn,
};

use jni::{
    objects::{AutoLocal, JObject, JValue},
    sys::jint,
    JNIEnv,
};

mod util;
use util::{attach_current_thread, unwrap};

#[test]
pub fn weak_ref_works_in_other_threads() {
    const ITERS_PER_THREAD: usize = 10_000;

    let env = attach_current_thread();
    let mut join_handlers = Vec::new();

    let atomic_integer_local = AutoLocal::new(
        &env,
        unwrap(
            &env,
            env.new_object(
                "java/util/concurrent/atomic/AtomicInteger",
                "(I)V",
                &[JValue::from(0)],
            ),
        ),
    );
    let atomic_integer =
        unwrap(&env, env.new_weak_ref(&atomic_integer_local)).expect("weak ref should not be null");

    // Test with a different number of threads (from 2 to 8)
    for thread_num in 2..9 {
        let barrier = Arc::new(Barrier::new(thread_num));

        for _ in 0..thread_num {
            let barrier = barrier.clone();
            let atomic_integer = atomic_integer.clone();

            let jh = spawn(move || {
                let env = attach_current_thread();
                barrier.wait();
                for _ in 0..ITERS_PER_THREAD {
                    let atomic_integer = env.auto_local(
                        unwrap(&env, atomic_integer.upgrade_local(&env))
                            .expect("AtomicInteger shouldn't have been GC'd yet"),
                    );
                    unwrap(
                        &env,
                        unwrap(
                            &env,
                            env.call_method(&atomic_integer, "incrementAndGet", "()I", &[]),
                        )
                        .i(),
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
                &env,
                unwrap(
                    &env,
                    env.call_method(
                        &atomic_integer_local,
                        "getAndSet",
                        "(I)I",
                        &[JValue::from(0)]
                    )
                )
                .i()
            )
        );
    }
}

#[test]
fn weak_ref_is_actually_weak() {
    let env = attach_current_thread();

    // This test uses `with_local_frame` to work around issue #109.

    fn run_gc(env: &JNIEnv) {
        unwrap(
            env,
            env.with_local_frame(1, || {
                env.call_static_method("java/lang/System", "gc", "()V", &[])?;
                Ok(JObject::null())
            }),
        );
    }

    for _ in 0..100 {
        let obj_local = env.auto_local(unwrap(
            &env,
            env.with_local_frame(2, || env.new_object("java/lang/Object", "()V", &[])),
        ));

        let obj_weak =
            unwrap(&env, env.new_weak_ref(&obj_local)).expect("weak ref should not be null");

        let obj_weak2 =
            unwrap(&env, obj_weak.clone_in_jvm(&env)).expect("weak ref should not be null");

        run_gc(&env);

        for obj_weak in &[&obj_weak, &obj_weak2] {
            {
                let obj_local_from_weak = env.auto_local(
                    unwrap(&env, obj_weak.upgrade_local(&env))
                        .expect("object shouldn't have been GC'd yet"),
                );

                assert!(!obj_local_from_weak.as_obj().is_null());
                assert!(unwrap(
                    &env,
                    env.is_same_object(&obj_local_from_weak, &obj_local)
                ));
            }

            {
                let obj_global_from_weak = unwrap(&env, obj_weak.upgrade_global(&env))
                    .expect("object shouldn't have been GC'd yet");

                assert!(!obj_global_from_weak.as_obj().is_null());
                assert!(unwrap(
                    &env,
                    env.is_same_object(&obj_global_from_weak, &obj_local)
                ));
            }

            assert!(unwrap(&env, obj_weak.is_same_object(&env, &obj_local)));

            assert!(
                !unwrap(&env, obj_weak.is_garbage_collected(&env)),
                "`is_garbage_collected` returned incorrect value"
            );
        }

        assert!(unwrap(
            &env,
            obj_weak.is_weak_ref_to_same_object(&env, &obj_weak2)
        ));

        drop(obj_local);
        run_gc(&env);

        for obj_weak in &[&obj_weak, &obj_weak2] {
            {
                let obj_local_from_weak = unwrap(&env, obj_weak.upgrade_local(&env));

                assert!(
                    obj_local_from_weak.is_none(),
                    "object should have been GC'd"
                );
            }

            {
                let obj_global_from_weak = unwrap(&env, obj_weak.upgrade_global(&env));

                assert!(
                    obj_global_from_weak.is_none(),
                    "object should have been GC'd"
                );
            }

            assert!(
                unwrap(&env, obj_weak.is_garbage_collected(&env)),
                "`is_garbage_collected` returned incorrect value"
            );
        }

        assert!(unwrap(
            &env,
            obj_weak.is_weak_ref_to_same_object(&env, &obj_weak2)
        ));
    }
}
