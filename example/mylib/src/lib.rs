#![deny(improper_ctypes_definitions)]

use jni::env::JNIEnvUnowned;

// These objects are what you should use as arguments to your native function.
// They carry extra lifetime information to prevent them escaping from the
// current local frame (which is the scope within which local (temporary)
// references to Java objects remain valid)
use jni::objects::{GlobalRef, JClass, JObject, JString};

use jni::objects::JByteArray;
use jni::sys::{jint, jlong};

use std::{sync::mpsc, thread, time::Duration};

// This `#[no_mangle]` keeps rust from "mangling" the name and making it unique
// for this crate. The name follows a strict naming convention so that the JNI
// implementation will be able to automatically find the implementation of a
// native method based on its name.
//
// The `'caller_frame` lifetime here represents the fact that the given local
// reference arguments belong to the callers JNI stack frame. By explicitly
// naming this lifetime it's possible to associate new local references with the
// same lifetime and return those to the caller.
//
// Note that, giving JNI stack frames a lifetime name and explicitly tracking
// thread attachments are important safety features for `jni-rs`.
//
// Safety:
//
// This is only safe if the signature matches the ABI that the JVM expects
//
// The lifetime of the caller frame must not be declared as `'static`
#[no_mangle]
pub extern "system" fn Java_HelloWorld_hello<'caller_frame>(
    // This `unowned_env` represents the fact that the JVM has implicitly
    // attached the current thread to the JVM (so you don't need to call
    // `JavaVM::attach_current_thread` before using JNI)
    //
    // Always use `JNIEnvUnowned` to capture raw `jni::sys::JNIEnv` pointers passed
    // to native methods, so that you can associate the pointer with a JNI stack
    // frame lifetime and safely use `JNIEnvUnowned::with_env`.
    mut unowned_env: JNIEnvUnowned<'caller_frame>,
    // this is the class that owns our static method. Not going to be used, but
    // still needs to have an argument slot
    _class: JClass<'caller_frame>,
    input: JString<'caller_frame>,
) -> JString<'caller_frame> {
    // Before we can start using the [`JNIEnv`] API we need to tell `jni-rs`
    // about the "unowned" thread attachment and map the raw pointer into a
    // non-transparent [`JNIEnv`] that is (internally) associated with a thread
    // attachment guard.
    unowned_env
        .with_env(|env| -> jni::errors::Result<_> {
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
            Ok(output)
        })
        .unwrap_or_else(|e| {
            eprintln!("Error: {:#?}", e);
            Default::default()
        })
}

#[no_mangle]
pub extern "system" fn Java_HelloWorld_helloByte<'caller_frame>(
    mut unowned_env: JNIEnvUnowned<'caller_frame>,
    _class: JClass,
    input: JByteArray,
) -> JByteArray<'caller_frame> {
    unowned_env
        .with_env(|env| -> jni::errors::Result<_> {
            // First, we have to get the byte[] out of java.
            let _input = env.convert_byte_array(&input).unwrap();

            // Then we have to create a new java byte[] to return.
            let buf = [1; 2000];
            let output = env.byte_array_from_slice(&buf).unwrap();
            Ok(output)
        })
        .unwrap_or_else(|e| {
            eprintln!("Error: {:#?}", e);
            Default::default()
        })
}

#[no_mangle]
pub extern "system" fn Java_HelloWorld_factAndCallMeBack(
    mut unowned_env: JNIEnvUnowned,
    _class: JClass,
    n: jint,
    callback: JObject,
) {
    unowned_env
        .with_env(|env| -> jni::errors::Result<_> {
            let i = n as i32;
            let res: jint = (2..i + 1).product();

            env.call_method(callback, "factCallback", "(I)V", &[res.into()])
                .unwrap();
            Ok(())
        })
        .unwrap_or_else(|e| {
            eprintln!("Error: {:#?}", e);
            Default::default()
        })
}

struct Counter {
    count: i32,
    callback: GlobalRef<JObject<'static>>,
}

impl Counter {
    pub fn new(callback: GlobalRef<JObject<'static>>) -> Counter {
        Counter {
            count: 0,
            callback: callback,
        }
    }

    pub fn increment(&mut self, env: &mut jni::env::JNIEnv) {
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
    mut unowned_env: JNIEnvUnowned,
    _class: JClass,
    callback: JObject,
) -> jlong {
    unowned_env
        .with_env(|env| -> jni::errors::Result<_> {
            let global_ref = env.new_global_ref(callback).unwrap();
            let counter = Counter::new(global_ref);

            Ok(Box::into_raw(Box::new(counter)) as jlong)
        })
        .unwrap_or_else(|e| {
            eprintln!("Error: {:#?}", e);
            Default::default()
        })
}

#[no_mangle]
pub unsafe extern "system" fn Java_HelloWorld_counterIncrement(
    mut unowned_env: JNIEnvUnowned,
    _class: JClass,
    counter_ptr: jlong,
) {
    unowned_env
        .with_env(|env| -> jni::errors::Result<_> {
            let counter = &mut *(counter_ptr as *mut Counter);

            counter.increment(env);
            Ok(())
        })
        .unwrap_or_else(|e| {
            eprintln!("Error: {:#?}", e);
            Default::default()
        })
}

#[no_mangle]
pub unsafe extern "system" fn Java_HelloWorld_counterDestroy(
    _unowned_env: JNIEnvUnowned,
    _class: JClass,
    counter_ptr: jlong,
) {
    let _boxed_counter = Box::from_raw(counter_ptr as *mut Counter);
}

#[no_mangle]
pub extern "system" fn Java_HelloWorld_asyncComputation(
    mut unowned_env: JNIEnvUnowned,
    _class: JClass,
    callback: JObject,
) {
    unowned_env
        .with_env(|env| -> jni::errors::Result<_> {
            // `JNIEnv` cannot be sent across thread boundaries. To be able to use JNI
            // functions in other threads, we must first obtain the `JavaVM` interface
            // which, unlike `JNIEnv` is `Send`.
            let jvm = env.get_java_vm();

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
                jvm.attach_current_thread(|env| -> jni::errors::Result<()> {
                    for i in 0..11 {
                        let progress = (i * 10) as jint;
                        // Now we can use all available `JNIEnv` functionality normally.
                        env.call_method(&callback, "asyncCallback", "(I)V", &[progress.into()])
                            .unwrap();
                        thread::sleep(Duration::from_millis(100));
                    }
                    Ok(())
                })
                .unwrap();

                // The current thread is detached automatically when `env` goes out of scope.
            });

            // Wait until the thread has started.
            rx.recv().unwrap();

            Ok(())
        })
        .unwrap_or_else(|e| {
            eprintln!("Error: {:#?}", e);
            Default::default()
        })
}
