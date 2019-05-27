use std::sync::{
    Arc,
    Once,
    ONCE_INIT,
};

use error_chain::ChainedError;
use jni::errors::Result;
use jni::{InitArgsBuilder, JNIEnv, JNIVersion, JavaVM, AttachGuard};

pub fn jvm() -> &'static Arc<JavaVM> {
    static mut JVM: Option<Arc<JavaVM>> = None;
    static INIT: Once = ONCE_INIT;

    INIT.call_once(|| {
        let jvm_args = InitArgsBuilder::new()
            .version(JNIVersion::V8)
            .option("-Xcheck:jni")
            .option("-Xdebug")
            .build()
            .unwrap_or_else(|e| panic!("{}", e.display_chain().to_string()));

        let jvm =
            JavaVM::new(jvm_args).unwrap_or_else(|e| panic!("{}", e.display_chain().to_string()));

        unsafe {
            JVM = Some(Arc::new(jvm));
        }
    });

    unsafe { JVM.as_ref().unwrap() }
}

pub fn attach_current_thread() -> AttachGuard<'static> {
    jvm()
        .attach_current_thread()
        .expect("failed to attach jvm thread")
}

pub fn print_exception(env: &JNIEnv) {
    let exception_occurred = env.exception_check().unwrap_or_else(|e| panic!("{:?}", e));
    if exception_occurred {
        env.exception_describe()
            .unwrap_or_else(|e| panic!("{:?}", e));
    }
}

#[allow(dead_code)]
pub fn unwrap<T>(env: &JNIEnv, res: Result<T>) -> T {
    res.unwrap_or_else(|e| {
        print_exception(&env);
        panic!("{}", e.display_chain().to_string());
    })
}
