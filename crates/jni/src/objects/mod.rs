mod jobject;
pub use self::jobject::*;

mod jthrowable;
pub use self::jthrowable::*;

mod jstack_trace_element;
pub use self::jstack_trace_element::*;

mod jclass;
pub use self::jclass::*;

mod jclass_loader;
pub use self::jclass_loader::*;

mod jstring;
pub use self::jstring::*;

mod jcollection;
pub use self::jcollection::*;

mod jset;
pub use self::jset::*;

mod jiterator;
pub use self::jiterator::*;

mod jmap;
pub use self::jmap::*;

mod jlist;
pub use self::jlist::*;

mod jbytebuffer;
pub use self::jbytebuffer::*;

mod jthread;
pub use self::jthread::*;

/// Primitive Array types
mod jobject_array;
pub use self::jobject_array::*;

mod type_array;
pub use self::type_array::*;

/// Primitive Array types
mod jprimitive_array;
pub use self::jprimitive_array::*;

#[doc(hidden)]
#[deprecated(
    since = "0.22.0",
    note = "Please use `jni::JValue*` instead of `jni::objects::JValue*`."
)]
pub use crate::jvalue::*;

#[doc(hidden)]
#[deprecated(
    since = "0.22.0",
    note = "Please use ID types under `jni::ids::*` instead of `jni::objects::*`."
)]
pub use crate::ids::*;

#[doc(hidden)]
#[deprecated(
    since = "0.22.0",
    note = "Please use reference types under `jni::refs::*` instead of `jni::objects::*`."
)]
pub use crate::refs::*;

#[doc(hidden)]
#[deprecated(
    since = "0.22.0",
    note = "Please use array elements types under `jni::elements::*` instead of `jni::objects::*`."
)]
pub use crate::elements::*;

// Provides a way to validate all our object bindings in a unit test, considering
// that the `J<Foo>API::get` functions are only exported with `pub(crate)` visibility.
//
// If any typos in method names or incorrect method or field signatures will result
// in a panic here when the binding initialization fails.
#[doc(hidden)]
pub fn _test_jni_init(env: &crate::Env, loader: &crate::refs::LoaderContext) {
    JByteBufferAPI::get(env, loader).expect("Failed to initialize JByteBufferAPI bindings");
    JClassLoaderAPI::get(env, loader).expect("Failed to initialize JClassLoaderAPI bindings");
    JClassAPI::get(env, loader).expect("Failed to initialize JClassAPI bindings");
    JCollectionAPI::get(env, loader).expect("Failed to initialize JCollectionAPI bindings");
    JIteratorAPI::get(env, loader).expect("Failed to initialize JIteratorAPI bindings");
    JListAPI::get(env, loader).expect("Failed to initialize JListAPI bindings");
    JMapAPI::get(env, loader).expect("Failed to initialize JMapAPI bindings");
    JMapEntryAPI::get(env, loader).expect("Failed to initialize JMapEntryAPI bindings");
    JObjectArrayAPI::<JString>::get(env, loader)
        .expect("Failed to initialize JObjectArrayAPI<JString> bindings");
    JObjectAPI::get(env).expect("Failed to initialize JObjectAPI bindings");
    JPrimitiveArrayAPI_jboolean::get(env, loader)
        .expect("Failed to initialize JPrimitiveArrayAPI_jboolean bindings");
    JSetAPI::get(env, loader).expect("Failed to initialize JSetAPI bindings");
    JStackTraceElementAPI::get(env, loader)
        .expect("Failed to initialize JStackTraceElementAPI bindings");
    JStringAPI::get(env, loader).expect("Failed to initialize JStringAPI bindings");
    JThreadAPI::get(env, loader).expect("Failed to initialize JThreadAPI bindings");
    JThrowableAPI::get(env, loader).expect("Failed to initialize JThrowableAPI bindings");
}
