#![cfg(feature = "invocation")]
mod util;

use jni_macros::bind_java_type;

// First, declare a minimal binding for a dependency type
// Using java.util.ArrayList as a real Java class we can reference
bind_java_type! {
    rust_type = JDep,
    java_type = "java.util.ArrayList"
}

// Now try to bind another type that refers to JDep within its type_map
// but with the WRONG java type name. This should fail at runtime because
// JDep actually maps to "java.util.ArrayList", not "java.util.HashMap"
bind_java_type! {
    rust_type = JTestType,
    java_type = "java.util.Vector",
    methods {
        fn test_method(dep: JDep) -> void,
    },
    type_map = {
        JDep => "java.util.HashMap", // This is WRONG! JDep is actually java.util.ArrayList
    }
}

#[test]
#[should_panic(expected = "Type mapping mismatch")]
fn test_incorrect_type_mapping_panics() {
    // Use attach_current_thread to get an Env reference for the runtime test
    let result = util::attach_current_thread(|env| {
        // Try to access the API, which will trigger the type mapping checks
        // We need to actually initialize the API to trigger the checks
        let loader = jni::refs::LoaderContext::None;
        let _ = JTestTypeAPI::get(env, &loader);

        // This should panic before we get here due to the type mapping assertion
        Ok(())
    });

    // If we got an error from attach_current_thread itself, unwrap it
    // (which will panic with the wrong message)
    result.unwrap();
}

// Add a test with correct type mapping to verify it works properly
bind_java_type! {
    rust_type = JDepCorrect,
    java_type = "java.util.LinkedList"
}

bind_java_type! {
    rust_type = JTestTypeCorrect,
    java_type = "java.util.Vector",
    type_map = {
        JDepCorrect => "java.util.LinkedList", // This is CORRECT!
    }
}

#[test]
fn test_correct_type_mapping_passes() {
    // Use attach_current_thread to get an Env reference for the runtime test
    let result = util::attach_current_thread(|env| {
        // Try to access the API, which will trigger the type mapping checks
        // This should succeed because the type mapping is correct
        let loader = jni::refs::LoaderContext::None;
        let _api = JTestTypeCorrectAPI::get(env, &loader)?;

        // If we get here, the type mapping check passed
        Ok(())
    });

    // Unwrap should succeed
    result.unwrap();
}

#[repr(transparent)]
struct MyHandle(*mut std::os::raw::c_void);

// It's enough to check that this compiles
bind_java_type! {
    rust_type = JHandleType,
    java_type = "com.example.HandleType",
    type_map = {
        unsafe MyHandle => long,
    }
}
