use std::sync::{Arc, Once, ONCE_INIT};

use error_chain::ChainedError;
use jni::{InitArgsBuilder, JNIEnv, JNIVersion, JavaVM};
use jni::errors::Result;


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

pub fn print_exception(env: &JNIEnv) {
    let exception_occurred = env.exception_check()
        .unwrap_or_else(|e| panic!(format!("{:?}", e)));
    if exception_occurred {
        env.exception_describe()
            .unwrap_or_else(|e| panic!(format!("{:?}", e)));
    }
}

pub fn unwrap<T>(env: &JNIEnv, res: Result<T>) -> T {
    res.unwrap_or_else(|e| {
        print_exception(&env);
        panic!(format!("{}", e.display_chain().to_string()));
    })
}

