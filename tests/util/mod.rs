use std::sync::{Arc, Once};

use jni::{
    errors::{Error, JniError, Result}, objects::JValue, sys::jint, AttachGuard, InitArgsBuilder, JNIEnv, JNIVersion,
    JavaVM,
};

mod example_proxy;

#[allow(unused_imports)]
pub use example_proxy::AtomicIntegerProxy;

pub fn jvm() -> &'static Arc<JavaVM> {
    static mut JVM: Option<Arc<JavaVM>> = None;
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        let jvm_args = InitArgsBuilder::new()
            .version(JNIVersion::V1_8)
            .option("-Xcheck:jni")
            .build()
            .unwrap_or_else(|e| panic!("{:#?}", e));

        let jvm = JavaVM::new(jvm_args).unwrap_or_else(|e| panic!("{:#?}", e));

        unsafe {
            JVM = Some(Arc::new(jvm));
        }
    });

    #[allow(static_mut_refs)]
    unsafe {
        JVM.as_ref().unwrap()
    }
}

#[allow(dead_code)]
pub fn call_java_abs(env: &mut JNIEnv, value: i32) -> i32 {
    env.call_static_method(
        "java/lang/Math",
        "abs",
        "(I)I",
        &[JValue::from(value as jint)],
    )
    .unwrap()
    .i()
    .unwrap()
}

#[allow(dead_code)]
pub unsafe fn attach_current_thread_for_scope<'local>() -> AttachGuard {
    // Safety: the caller must ensure that no other mutable `JNIEnv` in scope,
    // so we aren't creating an opportunity for local references to be created
    // in association with the wrong stack frame.
    unsafe {
        jvm()
            .attach_current_thread_for_scope(JNIVersion::V1_4)
            .expect("failed to attach jvm thread")
    }
}

#[allow(dead_code)]
pub unsafe fn attach_current_thread<'local>() -> AttachGuard {
    // Safety: the caller must ensure that no other mutable `JNIEnv` in scope,
    // so we aren't creating an opportunity for local references to be created
    // in association with the wrong stack frame.
    unsafe {
        jvm()
            .attach_current_thread(JNIVersion::V1_4)
            .expect("failed to attach jvm thread permanently")
    }
}

#[allow(dead_code)]
pub fn is_thread_attached() -> bool {
    // Safety:
    // Assumes tests are only run against a JavaVM that implements JNI >= 1.4
    //
    // We aren't materialising an `AttachGuard` while we already have access to
    // a guard or mutable `JNIEnv` in this scope.
    unsafe { jvm().get_env_attachment(JNIVersion::V1_4) }
        .map(|_| true)
        .or_else(|jni_err| match jni_err {
            Error::JniCall(JniError::ThreadDetached) => Ok(false),
            _ => Err(jni_err),
        })
        .expect("An unexpected JNI error occurred")
}

#[allow(dead_code)]
pub fn detach_current_thread() -> Result<()> {
    jvm().detach_current_thread()
}

pub fn print_exception(env: &JNIEnv) {
    let exception_occurred = env.exception_check();
    if exception_occurred {
        env.exception_describe();
    }
}

#[allow(dead_code)]
pub fn unwrap<T>(res: Result<T>, env: &JNIEnv) -> T {
    res.unwrap_or_else(|e| {
        print_exception(env);
        panic!("{:#?}", e);
    })
}
