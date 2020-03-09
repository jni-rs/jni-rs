#![cfg(feature = "invocation")]

use jni::objects::{JInputStream, JOutputStream};
use std::io::{Read, Write};

mod util;
use util::{attach_current_thread, unwrap};

#[test]
pub fn jinputstream_read() {
    let env = attach_current_thread();
    let data = b"this is a long string for testing multiple reads";

    let ba = unwrap(&env, env.byte_array_from_slice(data));
    let bais = unwrap(
        &env,
        env.new_object("java/io/ByteArrayInputStream", "([B)V", &[ba.into()]),
    );

    // Use a very small capacity to test multiple calls to `read`
    let mut stream = unwrap(&env, JInputStream::from_env_with_capacity(2, &env, bais));

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).unwrap();

    assert_eq!(&buf[..], &data[..]);
}

#[test]
pub fn joutputstream_write() {
    let env = attach_current_thread();
    let data = b"this is a long string for testing multiple writes";

    let baos = unwrap(
        &env,
        env.new_object("java/io/ByteArrayOutputStream", "()V", &[]),
    );

    // Use a very small capacity to test multiple calls to `write`
    let mut stream = unwrap(&env, JOutputStream::from_env_with_capacity(2, &env, baos));

    stream.write_all(data).unwrap();
    stream.flush().unwrap(); // no-op, just to test method call

    let ba = unwrap(&env, env.call_method(baos, "toByteArray", "()[B", &[]))
        .l()
        .unwrap();

    let buf = unwrap(&env, env.convert_byte_array(ba.into_inner()));

    assert_eq!(&buf[..], &data[..]);
}
