pub mod sys;

#[cfg(not(feature = "sys-only"))]
#[macro_use]
extern crate log;

#[cfg(not(feature = "sys-only"))]
#[macro_use]
extern crate error_chain;

#[cfg(not(feature = "sys-only"))]
extern crate combine;

#[cfg(not(feature = "sys-only"))]
extern crate cesu8;

#[cfg(not(feature = "sys-only"))]
#[macro_use]
mod macros;

// errors. do you really need an explanation?
#[cfg(not(feature = "sys-only"))]
pub mod errors;

// parser for method signatures
#[cfg(not(feature = "sys-only"))]
pub mod signature;

// wrappers arount jni pointer types that add lifetimes and other functionality.
#[cfg(not(feature = "sys-only"))]
pub mod jvalue;
#[cfg(not(feature = "sys-only"))]
pub mod jmethodid;
#[cfg(not(feature = "sys-only"))]
pub mod jobject;
#[cfg(not(feature = "sys-only"))]
pub mod jthrowable;
#[cfg(not(feature = "sys-only"))]
pub mod jclass;
#[cfg(not(feature = "sys-only"))]
pub mod jstring;
#[cfg(not(feature = "sys-only"))]
pub mod jmap;

// String types for sending to/from the jvm
#[cfg(not(feature = "sys-only"))]
pub mod ffi_str;
#[cfg(not(feature = "sys-only"))]
pub mod java_str;

// For when you want to store a reference to a java object
#[cfg(not(feature = "sys-only"))]
pub mod global_ref;

// Actual communication with the JVM
#[cfg(not(feature = "sys-only"))]
pub mod desc;
#[cfg(not(feature = "sys-only"))]
pub mod jnienv;
