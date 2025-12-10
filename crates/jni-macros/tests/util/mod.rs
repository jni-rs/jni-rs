use std::sync::{Arc, Once};

use jni::{Env, InitArgsBuilder, JNIVersion, JavaVM, errors::Result};

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
pub fn attach_current_thread<F, T>(callback: F) -> jni::errors::Result<T>
where
    F: FnOnce(&mut Env) -> jni::errors::Result<T>,
{
    jvm().attach_current_thread(|env| callback(env))
}

#[allow(dead_code)]
pub fn attach_current_thread_for_scope<F, T>(callback: F) -> jni::errors::Result<T>
where
    F: FnOnce(&mut Env) -> jni::errors::Result<T>,
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

pub fn print_exception(env: &Env) {
    let exception_occurred = env.exception_check();
    if exception_occurred {
        env.exception_describe();
    }
}

#[allow(dead_code)]
pub fn unwrap<T>(res: Result<T>, env: &Env) -> T {
    res.unwrap_or_else(|e| {
        print_exception(env);
        panic!("{:#?}", e);
    })
}

// Generic helper function to load any test class
#[allow(dead_code)]
pub fn load_test_class(
    env: &mut Env,
    out_dir: &std::path::Path,
    class_name: &str,
) -> jni::errors::Result<()> {
    let class_path = out_dir.join(format!("com/example/{}.class", class_name));
    assert!(
        class_path.exists(),
        "{}.class not found at {:?}",
        class_name,
        class_path
    );

    let class_bytes = std::fs::read(&class_path)
        .unwrap_or_else(|_| panic!("Failed to read {}.class", class_name));

    let class_loader = jni::objects::JClassLoader::get_system_class_loader(env)
        .expect("Failed to get system class loader");

    let class_internal_name = format!("com/example/{}", class_name);
    let class_jni = jni::strings::JNIString::new(class_internal_name.as_str());

    env.define_class(Some(&class_jni), &class_loader, &class_bytes)
        .unwrap_or_else(|_| panic!("Failed to define {} class", class_name));

    Ok(())
}

// Helper function to set up test output directory
#[allow(dead_code)]
pub fn setup_test_output(test_name: &str) -> std::path::PathBuf {
    let out_dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("jni_macros_tests")
        .join(test_name);

    // Clean up any existing output
    let _ = std::fs::remove_dir_all(&out_dir);
    std::fs::create_dir_all(&out_dir).expect("Failed to create test output directory");

    out_dir
}
