use std::sync::{Arc, Once};

use jni::{
    env::JNIEnv, errors::Result, objects::JValue, sys::jint, InitArgsBuilder, JNIVersion, JavaVM,
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
pub fn attach_current_thread<F, T>(callback: F) -> jni::errors::Result<T>
where
    F: FnOnce(&mut JNIEnv) -> jni::errors::Result<T>,
{
    jvm().attach_current_thread(|env| callback(env))
}

#[allow(dead_code)]
pub fn attach_current_thread_for_scope<F, T>(callback: F) -> jni::errors::Result<T>
where
    F: FnOnce(&mut JNIEnv) -> jni::errors::Result<T>,
{
    jvm().attach_current_thread_for_scope(|env| callback(env))
}

#[allow(dead_code)]
pub fn is_thread_attached() -> bool {
    jvm()
        .is_thread_attached()
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
