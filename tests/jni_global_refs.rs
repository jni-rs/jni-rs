#![cfg(feature = "invocation")]
extern crate error_chain;
extern crate jni;

use std::sync::{Arc, Barrier, Once, ONCE_INIT};
use std::thread::spawn;

use error_chain::ChainedError;
use jni::{InitArgsBuilder, JNIEnv, JNIVersion, JavaVM};
use jni::errors::Result;
use jni::objects::AutoLocal;
use jni::objects::JValue;
use jni::sys::jint;


pub fn jvm() -> &'static Arc<JavaVM> {
    static mut JVM: Option<Arc<JavaVM>> = None;
    static INIT: Once = ONCE_INIT;


    INIT.call_once(|| {
        let jvm_args = InitArgsBuilder::new()
            .version(JNIVersion::V8)
            .option("-Xcheck:jni")
            .option("-Xdebug")
            .build()
            .unwrap_or_else(|e| {
                panic!(format!("{}", e.display_chain().to_string()));
            });

        let jvm = JavaVM::new(jvm_args).unwrap_or_else(|e| {
            panic!(format!("{}", e.display_chain().to_string()));
        });

        unsafe {
            JVM = Some(Arc::new(jvm));
        }
    });

    unsafe { JVM.as_ref().unwrap() }
}

fn print_exception(env: &JNIEnv) {
    let exception_occurred = env.exception_check()
        .unwrap_or_else(|e| panic!(format!("{:?}", e)));
    if exception_occurred {
        env.exception_describe()
            .unwrap_or_else(|e| panic!(format!("{:?}", e)));
    }
}

fn unwrap<T>(env: &JNIEnv, res: Result<T>) -> T {
    res.unwrap_or_else(|e| {
        print_exception(&env);
        panic!(format!("{}", e.display_chain().to_string()));
    })
}

#[test]
pub fn global_ref_works_in_other_threads() {
    const ITERS_PER_THREAD: usize = 10_000;

    let env = jvm().attach_current_thread().unwrap();
    let mut join_handlers = Vec::new();

    let atomic_integer = {
        let local_ref = AutoLocal::new(&env, unwrap(&env, env.new_object(
            "java/util/concurrent/atomic/AtomicInteger",
            "(I)V",
            &[JValue::from(0)]
        )));
        unwrap(&env, env.new_global_ref(local_ref.as_obj()))
    };

    // Test with a different number of threads (from 2 to 8)
    for thread_num in 2..9 {
        let barrier = Arc::new(Barrier::new(thread_num));

        for _ in 0..thread_num {
            let barrier = barrier.clone();
            let mut atomic_integer = atomic_integer.clone();

            let jh = spawn(move || {
                let env = jvm().attach_current_thread().unwrap();
                barrier.wait();
                for _ in 0..ITERS_PER_THREAD {
                    unwrap(&env, unwrap(&env, env.call_method(
                        atomic_integer.as_obj(), "incrementAndGet", "()I", &[])).i());
                }
            });
            join_handlers.push(jh);
        };

        for jh in join_handlers.drain(..) {
            jh.join().unwrap();
        }

        let expected = (ITERS_PER_THREAD * thread_num) as jint;
        assert_eq!(expected, unwrap(&env, unwrap(&env, env.call_method(
            atomic_integer.as_obj(), "getAndSet", "(I)I", &[JValue::from(0)])).i()));
    }
}


#[test]
pub fn attached_detached_global_refs_works() {
    let env = jvm().attach_current_thread().unwrap();

    let local_ref = AutoLocal::new(&env, unwrap(&env, env.new_object(
        "java/util/concurrent/atomic/AtomicInteger",
        "(I)V",
        &[JValue::from(0)]
    )));

    // Test several global refs to the same object work
    let global_ref_1 = unwrap(&env, env.new_global_ref_attached(local_ref.as_obj()));
    {
        let global_ref_2 = unwrap(&env, env.new_global_ref_attached(local_ref.as_obj()));
        assert_eq!(1, unwrap(&env, unwrap(&env, env.call_method(
            global_ref_2.as_obj(), "incrementAndGet", "()I", &[])).i()));

        // Test detached & re-attached global ref works
        let global_ref_2 = unwrap(&env, global_ref_2.detach());
        let global_ref_2 = global_ref_2.attach(&env);
        assert_eq!(2, unwrap(&env, unwrap(&env, env.call_method(
            global_ref_2.as_obj(), "incrementAndGet", "()I", &[])).i()));

        // Test the first global ref unaffected by another global ref to the same object detached
        unwrap(&env, global_ref_2.detach());
    }
    assert_eq!(3, unwrap(&env, unwrap(&env, env.call_method(
        global_ref_1.as_obj(), "incrementAndGet", "()I", &[])).i()));
}
