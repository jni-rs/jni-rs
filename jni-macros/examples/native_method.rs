//! This example demonstrates the key features of the `native_method!` macro
//!
//! The `native_method!` macro creates compile-time type-checked `NativeMethod` descriptors
//! that can be registered with the JVM.
//!
//! Run with: `cargo run --example native_method`

use jni::errors::LogErrorAndDefault;
use jni::objects::{JClass, JIntArray, JObject, JString};
use jni::sys::jint;
use jni::{Env, EnvUnowned, NativeMethod, jni_sig, jni_str, native_method};

#[path = "utils/lib.rs"]
mod utils;

struct RustThing {
    pub message: String,
}

// Sometimes bindings need to pass raw pointers around as Java long values, so
// we define a simple wrapper type as an example.
// (this is not a well-considered design for real-world use)
#[repr(transparent)]
#[derive(Copy, Clone)]
struct ThingHandle(*const RustThing);
impl ThingHandle {
    pub fn new(thing: RustThing) -> Self {
        let boxed = Box::new(thing);
        ThingHandle(Box::into_raw(boxed))
    }

    unsafe fn as_ref(&self) -> &RustThing {
        unsafe { &*self.0 }
    }

    // Safety: only convert back to Box (to drop) when sure handle is no longer shared.
    pub unsafe fn into_box(self) -> Box<RustThing> {
        unsafe { Box::from_raw(self.0 as *mut RustThing) }
    }
}
// In order to pass ThingHandle values to/from Java as jlong, we need From<ThingHandle> for jlong
impl From<ThingHandle> for jni::sys::jlong {
    fn from(handle: ThingHandle) -> Self {
        handle.0 as jni::sys::jlong
    }
}

// Create an array of native methods using the native_method! macro
//
// Note: the use of `extern` (and with the provision of a `java_type` name) here means there will
// also be an `extern "system"` ABI function exported with a mangled JNI name that the JVM could
// resolve within a shared library without necessarily registering the methods explicitly.
//
// Since this example isn't linked as a shared library loaded by the JVM, it registers the methods
// explicitly with `env.register_native_methods()`.
const NATIVE_METHODS: &[NativeMethod] = &[
    // Instance methods - shorthand syntax
    // Without a `rust_type` the binding will use `JObject` for 'this'
    native_method! {
        java_type = "com.example.NativeMethodOverview",
        extern fn native_add(a: jint, b: jint) -> jint,
    },
    native_method! {
        java_type = "com.example.NativeMethodOverview",
        extern fn native_concatenate(a: JString, b: JString) -> JString,
    },
    native_method! {
        java_type = "com.example.NativeMethodOverview",
        type_map = {
            unsafe ThingHandle => long,
        },
        extern fn native_process_handle(handle: ThingHandle) -> JString,
    },
    // Raw native method (no catch_unwind, receives EnvUnowned directly)
    native_method! {
        java_type = "com.example.NativeMethodOverview",
        raw extern fn native_raw(value: jint) -> jint,
    },
    // Native method with direct function implementation
    native_method! {
        java_type = "com.example.NativeMethodOverview",
        extern fn native_with_function(value: jint) -> jint,
        fn = native_with_function_impl,
    },
    // Native method with catch_unwind disabled
    native_method! {
        java_type = "com.example.NativeMethodOverview",
        extern fn native_no_unwind(value: jint) -> jint,
        fn = native_no_unwind,
        catch_unwind = false,
    },
    // Native method with custom error policy
    native_method! {
        java_type = "com.example.NativeMethodOverview",
        extern fn native_custom_error_policy(value: jint) -> jint,
        fn = native_custom_error_policy,
        error_policy = LogErrorAndDefault,
    },
    // Static methods
    native_method! {
        java_type = "com.example.NativeMethodOverview",
        static extern fn native_greet(name: JString) -> JString,
    },
    native_method! {
        java_type = "com.example.NativeMethodOverview",
        static extern fn native_echo_int_array(arr: jint[]) -> jint[],
    },
];

// Implementation functions for native methods

fn native_add<'local>(
    _env: &mut Env<'local>,
    _this: JObject<'local>,
    a: jint,
    b: jint,
) -> Result<jint, jni::errors::Error> {
    Ok(a + b)
}

fn native_concatenate<'local>(
    env: &mut Env<'local>,
    _this: JObject<'local>,
    a: JString<'local>,
    b: JString<'local>,
) -> Result<JString<'local>, jni::errors::Error> {
    let a_str = a.try_to_string(env)?;
    let b_str = b.try_to_string(env)?;
    JString::from_str(env, format!("{}{}", a_str, b_str))
}

fn native_process_handle<'local>(
    env: &mut Env<'local>,
    _this: JObject<'local>,
    handle: ThingHandle,
) -> Result<JString<'local>, jni::errors::Error> {
    let thing_ref = unsafe { handle.as_ref() };
    let response = format!("RustThing says: {}", thing_ref.message);
    JString::from_str(env, response)
}

fn native_raw<'local>(
    _unowned_env: EnvUnowned<'local>,
    _this: JObject<'local>,
    value: jint,
) -> jint {
    value * 2
}

fn native_with_function_impl<'local>(
    _env: &mut Env<'local>,
    _this: JObject<'local>,
    value: jint,
) -> Result<jint, jni::errors::Error> {
    Ok(value * 3)
}

fn native_no_unwind<'local>(
    _env: &mut Env<'local>,
    _this: JObject<'local>,
    value: jint,
) -> Result<jint, jni::errors::Error> {
    Ok(value - 1)
}

fn native_custom_error_policy<'local>(
    _env: &mut Env<'local>,
    _this: JObject<'local>,
    value: jint,
) -> Result<jint, jni::errors::Error> {
    if value < 0 {
        Err(jni::errors::Error::JniCall(jni::errors::JniError::Unknown))
    } else {
        Ok(value)
    }
}

fn native_greet<'local>(
    env: &mut Env<'local>,
    _class: JClass<'local>,
    name: JString<'local>,
) -> Result<JString<'local>, jni::errors::Error> {
    let name_str = name.try_to_string(env)?;
    JString::from_str(env, format!("Hello, {}!", name_str))
}

fn native_echo_int_array<'local>(
    _env: &mut Env<'local>,
    _class: JClass<'local>,
    arr: JIntArray<'local>,
) -> Result<JIntArray<'local>, jni::errors::Error> {
    Ok(arr)
}

fn main() {
    utils::attach_current_thread(|env| {
        utils::load_class(env, "NativeMethodOverview")?;

        println!("=== native_method! Overview Example ===\n");

        // Register native methods
        println!("--- Registering Native Methods ---");
        let class = env.find_class(jni_str!("com/example/NativeMethodOverview"))?;
        unsafe {
            env.register_native_methods(&class, NATIVE_METHODS)?;
        }
        println!("Registered {} native methods", NATIVE_METHODS.len());

        // Create an instance
        println!("\n--- Creating Instance ---");
        let obj = env.new_object(&class, &jni_sig!(()->void), &[])?;
        println!("Created NativeMethodOverview instance");

        // Call the test method that exercises all native methods
        println!("\n--- Testing Native Methods ---");
        let test_method = env.get_method_id(
            &class,
            jni_str!("testAllNativeMethods"),
            &jni_sig!(()->JString),
        )?;
        let result = unsafe {
            env.call_method_unchecked(&obj, test_method, jni::signature::ReturnType::Object, &[])?
                .l()?
        };
        let result = unsafe { JString::from_raw(env, result.as_raw()) };
        println!("Test results: {}", result.try_to_string(env)?);

        // Test the handle method separately
        println!("\n--- Testing Handle Method ---");
        let thing = RustThing {
            message: "Hello from Rust!".to_string(),
        };
        let handle = ThingHandle::new(thing);

        // Call native_process_handle via Java
        let handle_value = jni::sys::jvalue { j: handle.into() };
        let handle_method = env.get_method_id(
            &class,
            jni_str!("nativeProcessHandle"),
            &jni_sig!((jlong)->JString),
        )?;
        let response = unsafe {
            env.call_method_unchecked(
                &obj,
                handle_method,
                jni::signature::ReturnType::Object,
                &[handle_value],
            )?
            .l()?
        };
        let response = unsafe { JString::from_raw(env, response.as_raw()) };
        println!(
            "nativeProcessHandle response: {}",
            response.try_to_string(env)?
        );

        // Safety: we are done with the handle, so convert back to Box to drop
        unsafe {
            drop(handle.into_box());
        }

        println!("\n=== Example completed successfully ===");
        Ok(())
    })
    .expect("Failed to run example");
}
