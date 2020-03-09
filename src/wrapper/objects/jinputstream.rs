use crate::{
    errors::Result,
    objects::{AutoLocal, JMethodID, JObject},
    signature::{JavaType, Primitive},
    sys::jsize,
    JNIEnv,
};
use std::io;

const DEFAULT_BUF_SIZE: usize = 8 * 1024; // same as std::io

/// Wrapper for JObjects that implement `java.io.InputStream`.
pub struct JInputStream<'a: 'b, 'b> {
    internal: JObject<'a>,
    buffer: AutoLocal<'a, 'b>,
    read: JMethodID<'a>,
    env: &'b JNIEnv<'a>,
}

impl<'a: 'b, 'b> std::ops::Deref for JInputStream<'a, 'b> {
    type Target = JObject<'a>;

    fn deref(&self) -> &Self::Target {
        &self.internal
    }
}

impl<'a: 'b, 'b> From<JInputStream<'a, 'b>> for JObject<'a> {
    fn from(other: JInputStream<'a, 'b>) -> JObject<'a> {
        other.internal
    }
}

impl<'a: 'b, 'b> JInputStream<'a, 'b> {
    /// Wrap an environment and an object that implements
    /// `java.io.InputStream`, specifying the capacity of the internal
    /// buffer.
    pub fn from_env_with_capacity(
        capacity: usize,
        env: &'b JNIEnv<'a>,
        obj: JObject<'a>,
    ) -> Result<JInputStream<'a, 'b>> {
        let class = env.auto_local(env.find_class("java/io/InputStream")?);

        let read = env.get_method_id(&class, "read", "([B)I")?;

        let buffer = env.auto_local(env.new_byte_array(capacity as jsize)?);

        Ok(JInputStream {
            internal: obj,
            buffer,
            read,
            env,
        })
    }
    /// Wrap an environment and an object that implements `java.io.InputStream`.
    pub fn from_env(env: &'b JNIEnv<'a>, obj: JObject<'a>) -> Result<JInputStream<'a, 'b>> {
        Self::from_env_with_capacity(DEFAULT_BUF_SIZE, env, obj)
    }
}

impl<'a: 'b, 'b> io::Read for JInputStream<'a, 'b> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.env
            .call_method_unchecked(
                self.internal,
                self.read,
                JavaType::Primitive(Primitive::Int),
                &[self.buffer.as_obj().into()],
            )
            .and_then(|count| {
                let count = count.i().unwrap();
                if count == -1 {
                    // EOF - signalled by a count of -1 in Java and 0 in Rust
                    Ok(0)
                } else {
                    let count = count as usize;
                    let buf = unsafe { &mut *(buf as *mut [u8] as *mut [i8]) };
                    self.env
                        .get_byte_array_region(*self.buffer.as_obj(), 0, &mut buf[..count])?;
                    Ok(count)
                }
            })
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }
}
