#![allow(dead_code)]

use std::sync::Arc;

use jni::{
    errors::*, objects::{GlobalRef, JValue}, sys::jint, JNIVersion, JavaVM, DEFAULT_LOCAL_FRAME_CAPACITY
};

/// A test example of a native-to-JNI proxy
#[derive(Clone)]
pub struct AtomicIntegerProxy {
    vm: Arc<JavaVM>,
    obj: GlobalRef,
}

impl AtomicIntegerProxy {
    /// Creates a new instance of `AtomicIntegerProxy`
    pub fn new(vm: Arc<JavaVM>, init_value: jint) -> Result<Self> {
        let mut guard = unsafe { vm.attach_current_thread(JNIVersion::V1_4)? };
        let obj = guard.with_env(DEFAULT_LOCAL_FRAME_CAPACITY, |env|{
            let i = env.new_object(
                "java/util/concurrent/atomic/AtomicInteger",
                "(I)V",
                &[JValue::from(init_value)],
            )?;
            env.new_global_ref(i)
        })?;
        Ok(AtomicIntegerProxy { vm, obj })
    }

    /// Gets a current value from java object
    pub fn get(&mut self) -> Result<jint> {
        let mut guard = unsafe { self.vm.attach_current_thread(JNIVersion::V1_4)? };
        guard.with_env(DEFAULT_LOCAL_FRAME_CAPACITY, |env| env.call_method(&self.obj, "get", "()I", &[])?.i())
    }

    /// Increments a value of java object and then gets it
    pub fn increment_and_get(&mut self) -> Result<jint> {
        let mut guard = unsafe { self.vm.attach_current_thread(JNIVersion::V1_4)? };
        guard.with_env(DEFAULT_LOCAL_FRAME_CAPACITY, |env| {
            env.call_method(&self.obj, "incrementAndGet", "()I", &[])?
                .i()
        })
    }

    /// Adds some value to the value of java object and then gets a resulting value
    pub fn add_and_get(&mut self, delta: jint) -> Result<jint> {
        let delta = JValue::from(delta);
        let mut guard = unsafe { self.vm.attach_current_thread(JNIVersion::V1_4)? };
        guard.with_env(DEFAULT_LOCAL_FRAME_CAPACITY, |env| {
            env.call_method(&self.obj, "addAndGet", "(I)I", &[delta])?
                .i()
        })
    }
}
