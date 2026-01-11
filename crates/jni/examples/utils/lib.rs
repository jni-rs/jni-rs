#![cfg(feature = "invocation")]
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::Once;

use jni::{Env, InitArgsBuilder, JNIVersion, JavaVM};

pub fn jvm() -> JavaVM {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let jvm_args = InitArgsBuilder::new()
            .version(JNIVersion::V1_8)
            .option("-Xcheck:jni")
            .build()
            .unwrap_or_else(|e| panic!("{:#?}", e));

        let _jvm = JavaVM::new(jvm_args).unwrap_or_else(|e| panic!("{:#?}", e));
    });
    JavaVM::singleton().expect("Failed to get singleton JVM")
}

pub fn attach_current_thread<F, T>(callback: F) -> jni::errors::Result<T>
where
    F: FnOnce(&mut Env) -> jni::errors::Result<T>,
{
    jvm().attach_current_thread(|env| callback(env))
}

fn setup_javac_output_dir() -> PathBuf {
    let build_dir = if let Some(dir) = option_env!("JAVA_BUILD_DIR") {
        PathBuf::from(dir)
    } else {
        let target_dir = if let Some(target_dir) = option_env!("CARGO_TARGET_DIR") {
            PathBuf::from(target_dir)
        } else if let Some(out_dir) = option_env!("OUT_DIR") {
            // OUT_DIR is something like .../target/debug/build/jni-macros-xxxx/out
            let mut path = PathBuf::from(out_dir);
            while path.file_name().is_some() {
                if path.file_name().unwrap() == "target" {
                    break;
                }
                path.pop();
            }
            let cachedir_tag = path.join("CACHEDIR.TAG");
            if !Path::exists(&cachedir_tag) {
                panic!("Couldn't find 'target' directory from OUT_DIR");
            }
            path
        } else {
            PathBuf::from("target")
        };
        target_dir.join("examples-java-classes")
    };

    std::fs::create_dir_all(&build_dir)
        .unwrap_or_else(|e| panic!("Failed to create javac output directory: {}", e));

    build_dir
}

fn jni_macros_dir() -> PathBuf {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set in environment");
    PathBuf::from(manifest_dir)
}

pub fn compile_class(name: &str) -> Vec<PathBuf> {
    let out_dir = setup_javac_output_dir();
    let jni_macros_dir = jni_macros_dir();

    let path = jni_macros_dir
        .join("examples/java/com/example")
        .join(name)
        .with_extension("java");
    let files = javac::Build::new()
        .file(path)
        .output_dir(&out_dir)
        .cargo_metadata(false) // don't output build.rs lines like "cargo:rerun-if-changed=..."
        .compile();

    println!("Compiled Java class {name}.java as: {:?}", files);
    files
}

pub fn define_class(env: &mut Env, class_path: &Path) -> jni::errors::Result<()> {
    use jni::strings::JNIStr;

    let class_bytes = std::fs::read(class_path)
        .unwrap_or_else(|_| panic!("Failed to read class file: {}", class_path.display()));

    let class_loader = jni::objects::JClassLoader::get_system_class_loader(env)?;

    env.define_class(Option::<&JNIStr>::None, &class_loader, &class_bytes)?;

    Ok(())
}

pub fn load_class(env: &mut Env, name: &str) -> jni::errors::Result<()> {
    let files = compile_class(name);
    for class_file in &files {
        define_class(env, class_file)?;
    }
    Ok(())
}

/// Define a stub Reference type for use in examples, without depending on
/// the bind_java_type! macro.
#[macro_export]
macro_rules! define_stub_type {
    ($rust_type:ident, $java_type:literal) => {
        #[repr(transparent)]
        #[derive(Default)]
        struct $rust_type<'local>(JObject<'local>);
        impl<'local> AsRef<JObject<'local>> for $rust_type<'local> {
            fn as_ref(&self) -> &JObject<'local> {
                &self.0
            }
        }
        impl<'local> From<$rust_type<'local>> for JObject<'local> {
            fn from(value: $rust_type<'local>) -> Self {
                value.0
            }
        }
        unsafe impl Reference for $rust_type<'_> {
            type Kind<'local> = $rust_type<'local>;
            type GlobalKind = $rust_type<'static>;

            fn as_raw(&self) -> jni2::sys::jobject {
                self.0.as_raw()
            }

            fn class_name() -> std::borrow::Cow<'static, jni2::strings::JNIStr> {
                std::borrow::Cow::Borrowed(jni::jni_str!($java_type))
            }

            fn lookup_class<'caller>(
                env: &Env<'_>,
                loader_context: &jni2::refs::LoaderContext,
            ) -> jni2::errors::Result<
                impl std::ops::Deref<Target = jni2::refs::Global<JClass<'static>>> + 'caller,
            > {
                static CLASS: std::sync::OnceLock<jni::objects::Global<jni::objects::JClass>> =
                    std::sync::OnceLock::new();

                let class = if let Some(class) = CLASS.get() {
                    class
                } else {
                    env.with_local_frame(4, |env| -> jni::errors::Result<_> {
                        let class: jni::objects::JClass =
                            loader_context.load_class_for_type::<Self>(env, false)?;
                        let global_class = env.new_global_ref(&class)?;
                        let _ = CLASS.set(global_class);
                        Ok(CLASS.get().unwrap())
                    })?
                };

                Ok(class)
            }

            unsafe fn kind_from_raw<'env>(reference: jni2::sys::jobject) -> Self::Kind<'env> {
                unsafe { $rust_type(JObject::kind_from_raw(reference)) }
            }

            unsafe fn global_kind_from_raw(global_ref: jni2::sys::jobject) -> Self::GlobalKind {
                unsafe { $rust_type(JObject::global_kind_from_raw(global_ref)) }
            }
        }
        impl<'local> $rust_type<'local> {
            #[allow(dead_code)]
            pub fn new(env: &mut Env<'local>) -> jni2::errors::Result<Self> {
                let class = Self::lookup_class(env, &jni2::refs::LoaderContext::default())?;
                let class: &JClass = class.as_ref();
                let obj = env.new_object(class, jni::jni_sig!("()V"), &[])?;
                Ok(Self(obj))
            }
        }
    };
}
