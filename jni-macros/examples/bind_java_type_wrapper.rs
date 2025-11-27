//! Example demonstrating a wrapper macro that can inject the jni crate path and
//! a type_map into bind_java_type! invocations.
//!
//! Notably the wrapper doesn't need to know anything about the syntax of
//! bind_java_type! and it doesn't stop you from adding additional type_map
//! entries.
//!
//! This could be useful in a workspace with:
//! - Multiple jni version dependencies
//! - Custom types that are used across many bindings

extern crate jni as jni2;

use jni2::bind_java_type;
use jni2::refs::LoaderContext;

#[path = "utils/lib.rs"]
mod utils;

// First, define some simple bindings for types we'll reference in the example type_maps
bind_java_type! {
    JBuiltinType => "com.example.CommonBuiltinType",
    constructors { fn new() }
}
bind_java_type! {
    JCustomType => "com.example.CustomType",
    constructors { fn new() }
}

/// Example wrapper macro that always uses a custom jni path and provides common type mappings
macro_rules! my_bind_java_type {
    ($($tt:tt)*) => {
        ::jni2::bind_java_type! {
            jni = ::jni2,
            type_map = {
                // Common types shared across all bindings
                // These must be defined before the wrapper is used
                crate::JBuiltinType => "com.example.CommonBuiltinType",
                typealias JBuiltinType => crate::JBuiltinType,
            },
            $($tt)*
        }
    };
}

// An example binding with additional type_map entries
my_bind_java_type! {
    rust_type = JBindJavaTypeWrapper,
    java_type = "com.example.BindJavaTypeWrapper",

    // Additional type mappings - these are merged with the wrapper's type_map
    type_map = {
        JCustomType => "com.example.CustomType",
    },

    constructors {
        fn new(),
    },

    methods {
        // Can use types from both wrapper and local type_map
        fn mix_types(builtin: JBuiltinType, custom: JCustomType) -> JString,
    },

    fields {
        builtin_field: JBuiltinType,
        custom_field: JCustomType,
    }
}

fn main() {
    utils::attach_current_thread(|env| {
        utils::load_class(env, "BindJavaTypeWrapper")?;

        let _loader_context = LoaderContext::default();

        let _instance = JBindJavaTypeWrapper::new(env)?;
        println!("Instantiated JBindJavaTypeWrapper (based on merged type_map)");

        // Test mix_types method
        println!("\n--- Testing mix_types method ---");
        let builtin_obj = JBuiltinType::new(env)?;
        let custom_obj = JCustomType::new(env)?;
        let s = _instance.mix_types(env, builtin_obj, custom_obj)?;
        println!("Result: {}", s.try_to_string(env)?);

        Ok(())
    })
    .expect("Failed to run example");
}
