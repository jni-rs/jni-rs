use std::cmp;
use std::io;

use crate::{
    errors::Result,
    objects::{AutoLocal, JMethodID, JObject},
    signature::{JavaType, Primitive},
    sys::jsize,
    JNIEnv,
};

const DEFAULT_BUF_SIZE: usize = 8 * 1024; // same as std::io

/// Wrapper for JObjects that implement `java.io.OutputStream`.
pub struct JOutputStream<'a: 'b, 'b> {
    internal: JObject<'a>,
    buffer: AutoLocal<'a, 'b>,
    buffer_len: usize,
    write: JMethodID<'a>,
    flush: JMethodID<'a>,
    env: &'b JNIEnv<'a>,
}

impl<'a: 'b, 'b> std::ops::Deref for JOutputStream<'a, 'b> {
    type Target = JObject<'a>;

    fn deref(&self) -> &Self::Target {
        &self.internal
    }
}

impl<'a: 'b, 'b> From<JOutputStream<'a, 'b>> for JObject<'a> {
    fn from(other: JOutputStream<'a, 'b>) -> JObject<'a> {
        other.internal
    }
}

impl<'a: 'b, 'b> JOutputStream<'a, 'b> {
    /// Wrap an environment and an object that implements
    /// `java.io.OutputStream`, specifying the capacity of the internal
    /// buffer.
    pub fn from_env_with_capacity(
        capacity: usize,
        env: &'b JNIEnv<'a>,
        obj: JObject<'a>,
    ) -> Result<JOutputStream<'a, 'b>> {
        let class = env.auto_local(env.find_class("java/io/OutputStream")?);

        let write = env.get_method_id(&class, "write", "([BII)V")?;
        let flush = env.get_method_id(&class, "flush", "()V")?;

        let buffer = env.auto_local(env.new_byte_array(capacity as jsize)?);

        Ok(JOutputStream {
            internal: obj,
            buffer,
            buffer_len: capacity,
            write,
            flush,
            env,
        })
    }
    /// Wrap an environment and an object that implements `java.io.OutputStream`.
    pub fn from_env(env: &'b JNIEnv<'a>, obj: JObject<'a>) -> Result<JOutputStream<'a, 'b>> {
        Self::from_env_with_capacity(DEFAULT_BUF_SIZE, env, obj)
    }
}

impl<'a: 'b, 'b> io::Write for JOutputStream<'a, 'b> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let count = cmp::min(self.buffer_len, buf.len());
        let buf = unsafe { &*(buf as *const [u8] as *const [i8]) };
        self.env
            .set_byte_array_region(*self.buffer.as_obj(), 0, &buf[..count])
            .and_then(|()| {
                self.env.call_method_unchecked(
                    self.internal,
                    self.write,
                    JavaType::Primitive(Primitive::Void),
                    &[
                        self.buffer.as_obj().into(),
                        0.into(),
                        (count as jsize).into(),
                    ],
                )?;
                Ok(count)
            })
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }
    fn flush(&mut self) -> io::Result<()> {
        self.env
            .call_method_unchecked(
                self.internal,
                self.flush,
                JavaType::Primitive(Primitive::Void),
                &[],
            )
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(())
    }
}
