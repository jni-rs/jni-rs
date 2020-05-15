# Panics and Results in Rust

The default error handling behaviors between Rust and Java are, unfortunately,
not particularly compatible. Invoking a `panic!` across a FFI boundary causes
undefined behavior. Usually, that behavior will be some kind of native crash,
but the compiler has been known to ignore the panic and proceed with execution
instead.

Luckily, panicking is also a discouraged mechanism for handling errors in Rust.
The best practice is to return one of two standard enums: `Result<T, E>`, if
your code is fallible and returns a `T` on success and an `E` on error; or
`Option<T>` if your code may or may not return something.

This allows us to update a callsite and, if we encounter the unsuccessful case
of a `Result` or an `Option`, call the method `env.throw()` and bail out
gracefully.

## Example: Throwing an Exception on Divide by Zero

```rust
#[no_mangle]
pub extern "system" fn Java_com_github_jni_1rs_jnibook_NativeAPI_divide(
    env: JNIEnv,
    _class: JClass,
    numerator: jint,
    divisor: jint,
) -> jint {
    if let Some(result) = numerator.checked_div(divisor) {
        // `checked_div` is a special version of division that returns `None` if the
        // divisor is zero. Since it returned `Some`, we know it is not.
        result
    } else {
        // Divisor is zero -- throw an exception!
        env.throw_new("java/lang/ArithmeticException", "Attempting to divide by zero.");
        // It doesn't matter what we return now, since the JNI will see that an
        // exception was thrown.
        0
    }
}
```

## Exercises

1. Instead of waiting for the operation to complete, identify an illegal divisor
   ahead of time and throw an `IllegalArgumentException` instead.
2. The function
   `[catch_unwind](https://doc.rust-lang.org/std/panic/fn.catch_unwind.html)`
   will allow you to execute code and recover from a call to `panic!`. This is
   discouraged since it won’t catch all panics (in some cases, a panic may just
   abort the process). However, to be really safe, update the code so that all
   unexpected panics will be caught through `catch_unwind` and turned into
   exceptions.
3. If you have a lot of different JNI functions, you might have to translate a
   lot of results in this way. Is it possible to reduce code duplication? If
   you’re an experienced Rustacean, you might try writing a macro or
   encapsulating this behavior into a trait.

