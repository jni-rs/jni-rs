// This is the interface to the JVM that we'll
// call the majority of our methods on.
use jni::{AttachGuard, JNIEnv, JNIVersion};

// These objects are what you should use as arguments to your native function.
// They carry extra lifetime information to prevent them escaping from the
// current local frame (which is the scope within which local (temporary)
// references to Java objects remain valid)
use jni::objects::{GlobalRef, JClass, JObject, JString};

use jni::objects::JByteArray;
use jni::sys::{jint, jlong};

use std::{sync::mpsc, thread, time::Duration};

// This `#[no_mangle]` keeps rust from "mangling" the name and making it unique
// for this crate. The name follows a strict naming convention so that the
// JNI implementation will be able to automatically find the implementation
// of a native method based on its name.
//
// The `'caller_frame` lifetime here represents the fact that the given local reference
// arguments belong to the callers JNI stack frame. This is just for illustration
// though since it not normally necessary to name these explicitly - all that
// matters is that the name must not be `'static` and the name will not match
// the lifetime associated with any new JNI stack frames.
//
// Whenever we get a `JNIEnv`, this will have an associated lifetime for the
// current JNI stack frame, so that new local references can't be moved out
// of that stack frame.
//
// FIXME: update these notes about the lifetime for the return type...
//
// Alternatively we could instead return the `jni::sys::jstring` type instead
// which would represent the same thing as a raw pointer, without any lifetime,
// and at the end use `.into_raw()` to convert a local reference with a lifetime
// into a raw pointer.
#[no_mangle]
pub extern "system" fn Java_HelloWorld_hello<'caller_frame>(
    // This is a raw `JNIEnv` pointer that represents an implicit thread
    // attachment to the Java VM.
    // Before we can use this, we need to wrap it into an [`AttachGuard`]
    // as an explicit representation of our thread attachment.
    env: *mut jni::sys::JNIEnv,
    // this is the class that owns our static method. Not going to be used, but
    // still needs to have an argument slot
    _class: JClass<'caller_frame>,
    input: JString<'caller_frame>,
) -> jni::sys::jstring {
    let mut guard = unsafe { AttachGuard::from_unowned(env) };
    let env = guard.current_frame_env();

    // First, we have to get the string out of java. Check out the `strings`
    // module for more info on how this works.
    let input: String = env
        .get_string(&input)
        .expect("Couldn't get java string!")
        .into();

    // Then we have to create a new java string to return. Again, more info
    // in the `strings` module.
    let output = env
        .new_string(format!("Hello, {}!", input))
        .expect("Couldn't create java string!");
    output.into_raw()
}

#[no_mangle]
pub extern "system" fn Java_HelloWorld_helloByte<'caller_frame>(
    env: *mut jni::sys::JNIEnv,
    _class: JClass,
    input: JByteArray,
) -> jni::sys::jarray {
    let mut guard = unsafe { AttachGuard::from_unowned(env) };
    let env = guard.current_frame_env();

    // First, we have to get the byte[] out of java.
    let _input = env.convert_byte_array(&input).unwrap();

    // Then we have to create a new java byte[] to return.
    let buf = [1; 2000];
    let output = env.byte_array_from_slice(&buf).unwrap();
    output.into_raw()
}

#[no_mangle]
pub extern "system" fn Java_HelloWorld_factAndCallMeBack(
    env: *mut jni::sys::JNIEnv,
    _class: JClass,
    n: jint,
    callback: JObject,
) {
    let mut guard = unsafe { AttachGuard::from_unowned(env) };
    let env = guard.current_frame_env();

    let i = n as i32;
    let res: jint = (2..i + 1).product();

    env.call_method(callback, "factCallback", "(I)V", &[res.into()])
        .unwrap();
}

struct Counter {
    count: i32,
    callback: GlobalRef,
}

impl Counter {
    pub fn new(callback: GlobalRef) -> Counter {
        Counter {
            count: 0,
            callback: callback,
        }
    }

    pub fn increment(&mut self, env: &mut JNIEnv) {
        self.count = self.count + 1;
        env.call_method(
            &self.callback,
            "counterCallback",
            "(I)V",
            &[self.count.into()],
        )
        .unwrap();
    }
}

#[no_mangle]
pub unsafe extern "system" fn Java_HelloWorld_counterNew(
    env: *mut jni::sys::JNIEnv,
    _class: JClass,
    callback: JObject,
) -> jlong {
    let mut guard = unsafe { AttachGuard::from_unowned(env) };
    let env = guard.current_frame_env();

    let global_ref = env.new_global_ref(callback).unwrap();
    let counter = Counter::new(global_ref);

    Box::into_raw(Box::new(counter)) as jlong
}

#[no_mangle]
pub unsafe extern "system" fn Java_HelloWorld_counterIncrement(
    env: *mut jni::sys::JNIEnv,
    _class: JClass,
    counter_ptr: jlong,
) {
    let mut guard = unsafe { AttachGuard::from_unowned(env) };
    let env = guard.current_frame_env();

    let counter = &mut *(counter_ptr as *mut Counter);

    counter.increment(env);
}

#[no_mangle]
pub unsafe extern "system" fn Java_HelloWorld_counterDestroy(
    _env: *mut jni::sys::JNIEnv,
    _class: JClass,
    counter_ptr: jlong,
) {
    let _boxed_counter = Box::from_raw(counter_ptr as *mut Counter);
}

#[no_mangle]
pub extern "system" fn Java_HelloWorld_asyncComputation(
    env: *mut jni::sys::JNIEnv,
    _class: JClass,
    callback: JObject,
) {
    let mut guard = unsafe { AttachGuard::from_unowned(env) };
    let env = guard.current_frame_env();

    // `JNIEnv` cannot be sent across thread boundaries. To be able to use JNI
    // functions in other threads, we must first obtain the `JavaVM` interface
    // which, unlike `JNIEnv` is `Send`.
    let jvm = env.get_java_vm().unwrap();

    // We need to obtain global reference to the `callback` object before sending
    // it to the thread, to prevent it from being collected by the GC.
    let callback = env.new_global_ref(callback).unwrap();

    // Use channel to prevent the Java program to finish before the thread
    // has chance to start.
    let (tx, rx) = mpsc::channel();

    let _ = thread::spawn(move || {
        // Signal that the thread has started.
        tx.send(()).unwrap();

        // Use the `JavaVM` interface to attach a `JNIEnv` to the current thread.
        // Safety: although we can technically see the existing guard for the current
        // thread we can't access them from here since guards aren't `Send/Sync`
        let mut guard = unsafe { jvm.attach_current_thread(JNIVersion::V1_4).unwrap() };
        let env = guard.current_frame_env();

        for i in 0..11 {
            let progress = (i * 10) as jint;
            // Now we can use all available `JNIEnv` functionality normally.
            env.call_method(&callback, "asyncCallback", "(I)V", &[progress.into()])
                .unwrap();
            thread::sleep(Duration::from_millis(100));
        }

        // The current thread is detached automatically when `env` goes out of scope.
    });

    // Wait until the thread has started.
    rx.recv().unwrap();
}
