#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(non_snake_case)]

#[macro_use]
extern crate log;

extern crate jni_sys;

#[macro_use]
extern crate error_chain;

extern crate combine;

extern crate cesu8;

#[macro_use]
mod macros;

mod signature;
pub mod errors;
pub mod desc;
pub mod ffi_str;
pub mod jvalue;
pub mod jmethodid;
pub mod jobject;
pub mod jthrowable;
pub mod jclass;
pub mod jstring;
pub mod java_string;
pub mod global_ref;
pub mod jnienv;
pub mod sys;
