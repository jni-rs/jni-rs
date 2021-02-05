# Division Hint

One approach relies on these steps:

1. Use `javac` to generate the function name.
2. Decorate the function with `#[no_mangle]` and `pub extern "system"`.
3. Add parameters for dividing. The type [`jni::sys::jint`](https://docs.rs/jni-sys/0.3.0/src/jni_sys/lib.rs.html#10) may help.
4. Remember to include the `JNIEnv` and `JClass` parameters, and to test the
   solution from Java.
5. If an `UnsatisfiedLinkError` is thrown, [refer to the
   introduction](./introduction.md##locating-shared-libraries).
