#![cfg(feature = "invocation")]
extern crate error_chain;
extern crate jni;

use jni::objects::AutoLocal;
use jni::objects::GlobalRef;
use jni::objects::JValue;
use jni::sys::jint;

mod util;
use util::*;


const VALUE: jint = 42;


/// The specification does not provide what should happen when a deleted reference is accessed.
/// [https://docs.oracle.com/javase/8/docs/technotes/guides/jni/spec/functions.html#Call_type_Method_routines]
///
/// So, we just test, that an error occurs.
///
/// But worse, the reference can again become a "valid" that makes this test fragile
/// [https://docs.oracle.com/javase/8/docs/technotes/guides/jni/spec/functions.html#GetObjectRefType]
///
/// "Since references are typically implemented as pointers to memory data structures that can
/// potentially be reused by any of the reference allocation services in the VM, once deleted,
/// it is not specified what value the GetObjectRefType will return".
///
/// *To avoid race condition this test routine should remain in a separate binary file.*

#[test]
pub fn global_ref_is_dropped() {
    let env = jvm().attach_current_thread().unwrap();

    let global_obj = {
        let local_ref = AutoLocal::new(&env, unwrap(&env, env.new_object(
            "java/util/concurrent/atomic/AtomicInteger",
            "(I)V",
            &[JValue::from(VALUE)]
        )));
        let global_ref = unwrap(&env, env.new_global_ref(local_ref.as_obj()));

        let res = env.call_method(global_ref.as_obj(), "get", "()I", &[]);
        assert_eq!(VALUE, unwrap(&env, unwrap(&env, res).i()));

        let obj = global_ref.as_obj().into_inner();

        // check that the other object still works
        let global_ref = unsafe { GlobalRef::from_raw(env.get_java_vm().unwrap(), obj) };
        let res = env.call_method(global_ref.as_obj(), "get", "()I", &[]);
        assert_eq!(VALUE, unwrap(&env, unwrap(&env, res).i()));
        std::mem::forget(global_ref);

        obj
    }; // << - here global and local references should already be deleted

    let global_ref = unsafe { GlobalRef::from_raw(env.get_java_vm().unwrap(), global_obj) };
    let res = env.call_method(global_ref.as_obj(), "get", "()I", &[]);

    assert!(res.is_err());
}
