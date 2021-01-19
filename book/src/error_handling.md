# Error Handling with JNI

In this section, we'll discuss what happens when an Exception is thrown, what
you can do about it, and strategies for handling `Result` and `Option` in native
methods.

## Handling Java Exceptions from Native Methods

[The Java Exceptions section of the JNI
Spec](https://docs.oracle.com/en/java/javase/11/docs/specs/jni/design.html#java-exceptions)
is the primary resource you should use to understand what is allowed when there
is an Exception that's caused by calling into Java from the native method. The
highlights include:

1. Whenever you call into Java, it's possible for an Exception to be thrown.
2. JNI offers a few APIs so that native methods may check for the presence of,
   retrieve, or clear Exceptions.
2. When a JNI call to Java throws an Exception, the native method must either
   return early (so that the calling Java code may handle the Exception), or
   handle and clear the Exception itself.
3. Only a few APIs are safe to call when there is a pending Exception, such as
   those related to Exception and resource release. Refer to the spec for the
   full list.
   
## Java Exceptions and `JNIEnv`
`JNIEnv` is the main interface that native methods use to interact with Java.
Since they can all fail, they all return `Result` types. Refer to the [docs on
`JNIEnv`](https://docs.rs/jni/0.18.0/jni/struct.JNIEnv.html) to understand how
it handles errors. Note that the methods on `JNIEnv` never clear Exceptions.

## Throwing Exceptions from Rust

Java can't handle Rust `Result` or `Option` types, but it can handle exceptions.
The Rust side can use
[`throw_new`](https://docs.rs/jni/0.18.0/jni/struct.JNIEnv.html#method.throw_new)
to throw exceptions, so that Java can decide on how to recover. For example:

```rust
#[no_mangle]
pub extern "system" fn Java_jni_1rs_1book_NativeAPI_divide(
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
        // A dummy value must be returned, since the native method retains control
        // even after throwing an Exception.
        0
    }
}
```


    
## Wrapping Result Handling into a Function

It can be helpful to wrap common error handling into one function, so that there
is less repeated JNI code. For ease of implementation, we'll use
`anyhow::Error`, and make the caller pass the `dummy_value`, which is returned
only on error paths. The `JNIEnv` APIs all throw `Result<T,E>`, and the
`JavaException` variant marks that an exception has already been thrown - so for
that variant alone, we will skip throwing an Exception. You may come up with
your own Error hierarchies, which could have a different logic.

```rust
fn try_java<F, T>(env: JNIEnv, dummy_value: T, f: F) -> T
where
    F: FnOnce() -> Result<T, anyhow::Error>,
{
    match f() {
        Ok(s) => s,
        Err(e) => {
            if let Some(e) = e.downcast_ref::<jni::errors::Error>() {
                match e {
                    // Since JavaException implies an Exception has already been thrown, 
                    // don't throw another one.
                    jni::errors::Error::JavaException => {},
                    _ => {
                        let _ = env.throw_new("java/lang/RuntimeException", e.to_string());
                    }
                }
            } else {
            // This branch handles all other error types, such as those returned by
            // all non-JNI code
                let _ = env.throw_new("java/lang/RuntimeException", e.to_string());
            }
            default_value
        }
    }
}
```

Finally, we can apply this to the checked division example:

```rust
use anyhow::Context;

#[no_mangle]
pub extern "system" fn Java_jni_1rs_1book_NativeAPI_divide(
    env: JNIEnv,
    _class: JClass,
    numerator: jint,
    divisor: jint,
) -> jint {
    try_java(env, 0, || {
    // context() moves self (an Option<T>) into a Result<T,E>, with context.
        numerator.checked_div(divisor).context("Attempted to divide by zero.")
    })
}
```

## Panic Handling

Note however, that `try_java` wrapper doesn't help with `panic!`, which leads to
undefined behavior when it propagates to Java (usually a crash). Fortunately,
it's possible to set `panic` handlers to catch them.

## Handling Java Exceptions in Native Methods

As mentioned earlier, there are APIs for clearing and checking Exceptions.
`exception_check` indicates whether there is a pending Exception, and
`exception_occurred` returns the `JThrowable` object. Use
[`call_method`](https://docs.rs/jni/0.18.0/jni/struct.JNIEnv.html#method.call_method)
to retrieve cause and description, if you need it.
[`exception_describe`](https://docs.rs/jni/0.18.0/jni/struct.JNIEnv.html#method.exception_describe)
can also be helpful during debugging, for collecting the exception and backtrace
from stderr (or other system error reporting channel).

```rust 
#[no_mangle]
pub extern "system" fn Java_jni_1rs_1book_NativeAPI_exception_1clearing(
    env: JNIEnv,
    _class: JClass
) {
    try_java(env, (), || {
        env.throw_new("java/lang/RuntimeException", "Any exception from Java")?;
        // Check if an exception has been thrown, without getting a handle to
        // the exception itself.
        let pending_exception : bool = env.exception_check()?;
        if pending_exception {
            // exception_occurred can be used to get a handle to the exception
            // object, if desired. The handle can then be used to get causes,
            // via call_method on JNIEnv.
            let _exception_object = env.exception_occurred()?;
            // Clear the exception we just threw
            env.exception_clear()?;
        }
        Ok(())
    })
}
```

## Summary

We've explored a few examples of exception handling via returning early and
clearing them from the native side, so that you can make your way through the
rest of the book. In most cases in the book, you'll want to rely on `try_java`
(or your own implementation of it), because it will allow you to write Rust code
that works with `Result` or `Option` without repeating Exception mappings.
