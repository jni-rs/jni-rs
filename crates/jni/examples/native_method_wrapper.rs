#![cfg(feature = "invocation")]
//! Example demonstrating a wrapper macro that can inject the jni crate path and
//! a type_map into native_method! invocations.
//!
//! Notably the wrapper doesn't need to know anything about the syntax of
//! native_method! and it doesn't stop you from adding additional type_map
//! entries.
//!
//! This could be useful in a workspace with:
//! - Multiple jni version dependencies
//! - Common Reference types used across many native methods
//! - Custom handle types that are used across many native methods

extern crate jni as jni2;

use jni2::JValue;
use jni2::objects::{JClass, JObject, JString};
use jni2::refs::LoaderContext;
use jni2::refs::Reference;
use jni2::{Env, NativeMethod};
use std::ffi::{CStr, c_char};

use thiserror::Error;

#[derive(Error, Debug)]
enum MyError {
    #[error("Invalid UTF-8 in handle: {0}")]
    InvalidUtf8(#[from] std::str::Utf8Error),
    #[error("JNI call failed")]
    Jni(#[from] jni2::errors::Error),
}

#[path = "utils/lib.rs"]
mod utils;

// First, define some simple bindings for types we'll reference in the example type_maps
// (macro from utils/lib.rs)
define_stub_type!(JNativeMethodWrapper, "com.example.NativeMethodWrapper");
define_stub_type!(JBuiltinType, "com.example.CommonBuiltinType");
define_stub_type!(JCustomType, "com.example.CustomType");

// Define an FFI handle type that can be added to a common type_map
#[repr(transparent)]
#[derive(Copy, Clone)]
struct CommonHandle(*const c_char);
impl From<CommonHandle> for jni::sys::jlong {
    fn from(handle: CommonHandle) -> Self {
        handle.0 as *const u8 as jni::sys::jlong
    }
}

/// Example wrapper macro that always uses a custom jni path and provides common type mappings
macro_rules! my_native_method {
    ($($tt:tt)*) => {
        ::jni2::native_method! {
            jni = ::jni2,
            type_map = {
                // Common types shared across all native methods
                // These must be defined before the wrapper is used
                crate::JBuiltinType => "com.example.CommonBuiltinType",
                typealias JBuiltinType => crate::JBuiltinType,
                // Common handle types shared across all native methods
                unsafe CommonHandle => long,
            },
            $($tt)*
        }
    };
}
// For consistency, do the same for jni_sig! macro which we'll use to call the methods
macro_rules! my_jni_sig {
    ($($tt:tt)*) => {
        ::jni2::jni_sig!(
            jni = ::jni2,
            type_map = {
                crate::JBuiltinType => "com.example.CommonBuiltinType",
                typealias JBuiltinType => crate::JBuiltinType,
            },
            $($tt)*
        )
    };
}
// and, for consistency, also rename the jni crate for the jni_str macro
macro_rules! my_jni_str {
    ($($tt:tt)*) => {
        ::jni2::jni_str!(
            jni = ::jni2,
            $($tt)*
        )
    };
}

// Create an array of native methods using the wrapper macro
const NATIVE_METHODS: &[NativeMethod] = &[
    // Method using a handle type from the wrapper's type_map
    my_native_method! {
        fn JNativeMethodWrapper::process_resource(handle: CommonHandle) -> JString,
    },
    // Static method using a builtin type from the wrapper's type_map and an additional type_map entry
    my_native_method! {
        type_map = {
            // Additional type mapping specific to this method
            JCustomType => com.example.CustomType,
        },
        static fn JNativeMethodWrapper::mix_types(builtin: JBuiltinType, custom: JCustomType) -> JString,
    },
];

// Implementation functions for native methods

impl JNativeMethodWrapper<'_> {
    fn process_resource<'local>(
        env: &mut Env<'local>,
        _this: JNativeMethodWrapper<'local>,
        handle: CommonHandle,
    ) -> Result<JString<'local>, MyError> {
        // Safely: we assume the handle is a valid CStr pointer for this example
        let c_str = unsafe { CStr::from_ptr(handle.0) };
        let str_slice = c_str.to_str().map_err(MyError::InvalidUtf8)?;
        let response = format!("Processed resource: {}", str_slice);
        Ok(JString::from_str(env, response)?)
    }

    fn mix_types<'local>(
        env: &mut Env<'local>,
        _class: JClass<'local>,
        _builtin: JBuiltinType<'local>,
        _custom: JCustomType<'local>,
    ) -> Result<JString<'local>, jni2::errors::Error> {
        JString::new(env, "Mixed types successfully")
    }
}

fn main() {
    utils::attach_current_thread(|env| {
        utils::load_class(env, "NativeMethodWrapper")?;

        println!("=== native_method! Wrapper Example ===\n");

        // Register native methods
        println!("--- Registering Native Methods (via wrapper macro) ---");
        let class = JNativeMethodWrapper::lookup_class(env, &LoaderContext::default())?;
        let class: &JClass = &class;
        unsafe {
            env.register_native_methods(class, NATIVE_METHODS)?;
        }
        println!("Registered {} native methods", NATIVE_METHODS.len());

        // Create an instance
        println!("\n--- Creating Instance ---");
        let obj = JNativeMethodWrapper::new(env)?;
        println!("Created NativeMethodWrapper instance");

        // Test process_resource with a CStr handle
        println!("\n--- Testing process_resource ---");
        let resource_data = c"my_resource_data";
        let resource_handle = CommonHandle(resource_data.as_ptr());
        let obj = env
            .call_method(
                &obj,
                my_jni_str!("processResource"),
                my_jni_sig!((jlong)->JString),
                &[JValue::Long(resource_handle.into())],
            )?
            .l()?;
        let s = JString::cast_local(env, obj)?;
        println!("Result: {}", s.try_to_string(env)?);

        // Test mix_types with JCommonBuiltinType and JCustomType
        // Note: we refer to CustomType via the full Java type name here so we
        // don't need to pass a type_map for just one usage
        println!("\n--- Testing mix_types ---");
        let builtin_obj = JBuiltinType::new(env)?;
        let custom_obj = JCustomType::new(env)?;
        let result = env
            .call_static_method(
                class,
                my_jni_str!("mixTypes"),
                my_jni_sig!((builtin: JBuiltinType, custom: com.example.CustomType)->JString),
                &[
                    JValue::Object(&builtin_obj.into()),
                    JValue::Object(&custom_obj.into()),
                ],
            )?
            .l()?;
        println!(
            "Result: {}",
            JString::cast_local(env, result)?.try_to_string(env)?
        );

        println!("\n=== Example completed successfully ===");
        Ok(())
    })
    .expect("Failed to run example");
}
