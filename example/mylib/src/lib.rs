extern crate jni;

// This is the interface to the JVM that we'll
// call the majority of our methods on.
use jni::JNIEnv;

// These objects are what you should use as arguments to your native function.
// They carry extra lifetime information to prevent them escaping this context
// and getting used after being GC'd.
use jni::objects::{GlobalRef, JClass, JObject, JString};

// This is just a pointer. We'll be returning it from our function.
// We can't return one of the objects with lifetime information because the
// lifetime checker won't let us.
use jni::sys::{jint, jlong, jstring};

use std::thread;
use std::time::Duration;
use std::sync::mpsc;

// This keeps rust from "mangling" the name and making it unique for this crate.
#[no_mangle]
// This turns off linter warnings because
// the name doesn't conform to conventions.
#[allow(non_snake_case)]
pub extern "system" fn Java_HelloWorld_hello(env: JNIEnv,
                                             // this is the class that owns our
                                             // static method. Not going to be
                                             // used, but still needs to have
                                             // an argument slot
                                             _class: JClass,
                                             input: JString)
                                             -> jstring {
    // First, we have to get the string out of java. Check out the `strings`
    // module for more info on how this works.
    let input: String =
        env.get_string(input).expect("Couldn't get java string!").into();

    // Then we have to create a new java string to return. Again, more info
    // in the `strings` module.
    let output = env.new_string(format!("Hello, {}!", input))
        .expect("Couldn't create java string!");

    // Finally, extract the raw pointer to return.
    output.into_inner()
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn Java_HelloWorld_factAndCallMeBack(env: JNIEnv,
                                                         _class: JClass,
                                                         n: jint,
                                                         callback: JObject) {
    let i = n as i32;
    let res: jint = (2..i + 1).product();

    env.call_method(callback, "factCallback", "(I)V", &[res.into()]).unwrap();
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

    pub fn increment(&mut self, env: JNIEnv) {
        self.count = self.count + 1;
        env.call_method(self.callback.as_obj(),
                         "counterCallback",
                         "(I)V",
                         &[self.count.into()])
            .unwrap();
    }
}


#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "system" fn Java_HelloWorld_counterNew(env: JNIEnv,
                                                         _class: JClass,
                                                         callback: JObject)
                                                         -> jlong {
    let global_ref = env.new_global_ref(callback).unwrap();
    let counter = Counter::new(global_ref);

    Box::into_raw(Box::new(counter)) as jlong
}

#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "system" fn Java_HelloWorld_counterIncrement(
    env: JNIEnv,
    _class: JClass,
    counter_ptr: jlong
){
    let counter = &mut *(counter_ptr as *mut Counter);

    counter.increment(env);
}

#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "system" fn Java_HelloWorld_counterDestroy(
    _env: JNIEnv,
    _class: JClass,
    counter_ptr: jlong
){
    let _boxed_counter = Box::from_raw(counter_ptr as *mut Counter);
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn Java_HelloWorld_asyncComputation(
    env: JNIEnv,
    _class: JClass,
    callback: JObject,
) {
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
        let env = jvm.attach_current_thread().unwrap();

        // Then use the `callback` with this newly obtained `JNIEnv`.
        let callback = callback.as_obj();

        for i in 0..11 {
            let progress = (i * 10) as jint;
            // Now we can use all available `JNIEnv` functionality normally.
            env.call_method(callback, "asyncCallback", "(I)V", &[progress.into()])
                .unwrap();
            thread::sleep(Duration::from_millis(100));
        }

        // The current thread is detached automatically when `env` goes out of scope.
    });

    // Wait until the thread has started.
    rx.recv().unwrap();
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
