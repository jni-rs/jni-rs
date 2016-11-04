#[macro_use]
extern crate log;

extern crate jni_sys;

#[macro_use]
extern crate error_chain;

extern crate combine;

extern crate cesu8;

#[macro_use]
mod macros;

// Re-export of the jni_sys types
pub mod sys;

// errors. do you really need an explanation?
pub mod errors;

// parser for method signatures
pub mod signature;

// wrappers arount jni pointer types that add lifetimes and other functionality.
pub mod jvalue;
pub mod jmethodid;
pub mod jobject;
pub mod jthrowable;
pub mod jclass;
pub mod jstring;
pub mod jmap;

// String types for sending to/from the jvm
pub mod ffi_str;
pub mod java_str;

// For when you want to store a reference to a java object
pub mod global_ref;

// Actual communication with the JVM
pub mod desc;
pub mod jnienv;
