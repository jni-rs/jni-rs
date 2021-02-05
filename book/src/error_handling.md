# Error Handling with JNI

In this section, we'll discuss what happens when an exception is thrown or a
panic is raised, what you can do about it, and strategies for handling `Result`
and `Option` in native methods.

## Handling Java Exceptions from Native Methods

[The Java Exceptions section of the JNI
Spec](https://docs.oracle.com/en/java/javase/11/docs/specs/jni/design.html#java-exceptions)
is the primary resource you should use to understand what is allowed when there
is an exception that's caused by calling into Java from the native method. The
highlights include:

1. Whenever you call into Java, it's possible for an exception to be thrown.
2. JNI offers a few APIs so that native methods may check for the presence of,
   retrieve, or clear exceptions.
2. When a JNI call to Java throws an exception, the native method must either
   return early (so that the calling Java code may handle the exception), or
   handle and clear the exception itself.
3. Only a few APIs are safe to call when there is a pending exception, such as
   those related to exception and resource release. Refer to the spec for the
   full list.
   
## Java Exceptions and `JNIEnv`
`JNIEnv` is the main interface that native methods use to interact with Java.
Since they can all fail, they all return `Result` types. Refer to the [docs on
`JNIEnv`](https://docs.rs/jni/0.18.0/jni/struct.JNIEnv.html) to understand how
it handles errors. Note that the methods on `JNIEnv` never clear exceptions.

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
        // even after throwing an exception.
        0
    }
}
```


    
## Wrapping Result Handling into a Function

It can be helpful to wrap common error handling into one function, so that there
is less repeated JNI code. For ease of implementation, we'll use
`anyhow::Error`, and make the caller pass the `default_value` that will be
returned on error paths. To catch unwinding panics, we'll use `catch_unwind` [as
described in the
Rustonomicon.](https://doc.rust-lang.org/nomicon/ffi.html#ffi-and-panics)

```rust
{{#include ../projects/completed/jnibookrs/src/error.rs:try_java}}
```

We could improve our error handler to attach panic cause information to the
exception, or translate different Error types into different exceptions types.
For now, we will apply it to the checked division example, and verify that our
previous tests now pass.

```rust,noplaypen
{{#include ../projects/completed/jnibookrs/src/division.rs:try_java_imports}}

{{#include ../projects/completed/jnibookrs/src/division.rs:try_java}}
```

```java
{{#include ../projects/completed/jnibookgradle/src/test/java/jni_rs_book/DivisionTest.java:complete}}
```

## Handling Java Exceptions in Native Methods

As mentioned earlier, there are APIs for checking and clearing Exceptions.
`exception_check` indicates whether there is a pending exception,
`exception_occurred` returns the `JThrowable` object.
[`exception_describe`](https://docs.rs/jni/0.18.0/jni/struct.JNIEnv.html#method.exception_describe)
can also be helpful during debugging, since it will print the exception and
backtrace to stderr (or other system error reporting channel). Finally,
[`exception_clear`](https://docs.rs/jni/0.18.0/jni/struct.JNIEnv.html#method.exception_clear)
clears the pending exception.

You might wonder whether there's a way to access an exception cause or
description from native code. Doing so requires
[`call_method`](https://docs.rs/jni/0.18.0/jni/struct.JNIEnv.html#method.call_method)
(or the unchecked variant), since there are no specialized methods for
retrieving or setting them.

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
            // Both exception_occurred and exception_check leave
            // the exception pending.
            //
            // Clear the exception we just threw.
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
(or your own implementation of it), since it will let you write shorter code.
More advanced handlers may translate `Error` enums into different types of Java
exceptions, attach the original causes of panics, or attempt to save on the
number of calls to `exception_check()`.

There's quite a lot to think about with respect to Error handling, and perhaps you can make some improvements on top of what is suggested here.
