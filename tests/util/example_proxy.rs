#![allow(dead_code)]

use std::{ops::Deref, sync::Arc};

use jni::{
    errors::*,
    objects::{Global, JObject, JValue},
    sys::jint,
    JavaVM, DEFAULT_LOCAL_FRAME_CAPACITY,
};

/// A test example of a native-to-JNI proxy
#[derive(Clone)]
pub struct AtomicIntegerProxy {
    inner: Arc<AtomicIntegerProxyInner>,
}

impl Deref for AtomicIntegerProxy {
    type Target = AtomicIntegerProxyInner;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub struct AtomicIntegerProxyInner {
    obj: Global<JObject<'static>>,
}

impl AtomicIntegerProxy {
    /// Creates a new instance of `AtomicIntegerProxy`
    pub fn new(vm: Arc<JavaVM>, init_value: jint) -> Result<Self> {
        vm.attach_current_thread(|env| -> Result<Self> {
            let obj = env.with_local_frame(DEFAULT_LOCAL_FRAME_CAPACITY, |env| {
                let i = env.new_object(
                    c"java/util/concurrent/atomic/AtomicInteger",
                    c"(I)V",
                    &[JValue::from(init_value)],
                )?;
                env.new_global_ref(i)
            })?;
            Ok(AtomicIntegerProxy {
                inner: Arc::new(AtomicIntegerProxyInner { obj }),
            })
        })
    }

    /// Gets a current value from java object
    pub fn get(&mut self) -> Result<jint> {
        let vm = JavaVM::singleton()?;
        vm.attach_current_thread(|env| env.call_method(&*self.obj, c"get", c"()I", &[])?.i())
    }

    /// Increments a value of java object and then gets it
    pub fn increment_and_get(&mut self) -> Result<jint> {
        let vm = JavaVM::singleton()?;
        vm.attach_current_thread(|env| {
            env.call_method(&*self.obj, c"incrementAndGet", c"()I", &[])?
                .i()
        })
    }

    /// Adds some value to the value of java object and then gets a resulting value
    pub fn add_and_get(&mut self, delta: jint) -> Result<jint> {
        let vm = JavaVM::singleton()?;
        vm.attach_current_thread(|env| {
            let delta = JValue::from(delta);
            env.call_method(&*self.obj, c"addAndGet", c"(I)I", &[delta])?
                .i()
        })
    }
}
