# Linking and Dividing

This section introduces everything thats needed to write JNI functions. Upon
finishing this chapter, you'll have written a method that divides two integers
using Rust.

# Whats in NativeAPI?

`NativeAPI` is a class that lives in `jnibookjava`, the Java side of the example
code that accompanies this book. It uses `System.loadLibrary` to locate the
`jnibookrs` lib, and specifies native methods using the `native` keyword. The
native method called `verify_link` was used earlier to ensure that the project
was setup correctly, with environment variables.

For style reasons, we always add native methods to `NativeAPI`. It doesn't need
to be that way, it's just a convention that we follow here.

```java
package com.github.jni_rs.jnibook;

class NativeAPI {

   private static final Throwable INIT_ERROR;

   // The static block will be executed the first time the NativeAPI
   // class is used.
   static {
       Throwable error = null;
       try {
           System.loadLibrary("jnibookrs");
       } catch (Throwable t) {
           error = t;
       }
       INIT_ERROR = error;
   }

   private NativeAPI() {
       // Not instantiable
   }

   static native int verify_link();
}
```


## Function Linking Rules

JNI identifies the functions it should link to using the package path, class
name, and method name by following [these
rules](https://docs.oracle.com/en/java/javase/11/docs/specs/jni/design.html).
Since `.` isn’t allowed in C function names, the Java function's package path's
`.` is mapped to `_`. However, since `_` could appear in a function name, it
needs a different representation. So `_` gets replaced by `_1` before replacing
`.` with `_`. And finally, the function name has to be prefixed with `Java_`, to
avoid conflicts with other languages that support similar naming schemes.

## LookingGlass - Function Linking Rules

Since the naming rules are esoteric, you'll practice them as they are introduced
using an example called `LookingGlass`. This example is purely for discussion
and it won't be necessary to code it. We'll return back to the `LookingGlass`
example for "Parameters" and "ABI and disabling mangling."

The definition of `LookingGlass` on the Java side will be:

```java
package com.github.imaginarypackagename;

class LookingGlass {

    native static void test_call();
}
```

And now, lets apply the naming rules discussed in "Function Linking Rules." We'll start with this code in Rust:

```rust
fn test_call() {

}
```

Prefix the Rust function name with `Java.` +
   `com.github.imaginarypackagename.` + `LookingGlass`.

```rust
fn Java.com.github.imaginarypackagename.LookingGlass.test_call() {

}
```

Then, replace `_` with `_1`.

```rust
fn Java.com.github.imaginarypackagename.LookingGlass.test_1call() {

}
```

Finally, replace `.` with `_`.

```rust
fn Java_com_github_imaginarypackagename_LookingGlass_test_1call() {

}
```

Now the native function is named correctly for JNI.

## Parameters and Return Types
`JNIEnv` and `JObject` are arguments one and two of every JNI Function. When the
native method is static, the second argument can be tagged more specifically as
`JClass`. `JNIEnv` is an object you can use to call into the JVM, while the
second argument corresponds to the `this` reference in Java for nonstatic
methods. For static native methods, the second argument refers to the class
containing the static native method.

```rust
// Imports for `JClass and JEnv`
use jni::objects::JClass;
use jni::JNIEnv;
```

The next arguments map 1:1 with parameters specified in the Java method's
signature. For example, if the signature was `void add(int a, int b)`, then the
3rd and 4th arguments in the native function would be of type `jni::sys::jint`.
For reference, these are the jni-rs types that you can include in your JNI
signatures:

```
// from jni-rs
pub type jint = i32;
pub type jlong = i64;
pub type jbyte = i8;
pub type jboolean = u8;
pub type jchar = u16;
pub type jshort = i16;
pub type jfloat = f32;
pub type jdouble = f64;
pub type jsize = jint;

pub enum _jobject {}
pub type jobject = *mut _jobject;
pub type jclass = jobject;
pub type jthrowable = jobject;
pub type jstring = jobject;
pub type jarray = jobject;
pub type jbooleanArray = jarray;
pub type jbyteArray = jarray;
pub type jcharArray = jarray;
pub type jshortArray = jarray;
pub type jintArray = jarray;
pub type jlongArray = jarray;
pub type jfloatArray = jarray;
pub type jdoubleArray = jarray;
pub type jobjectArray = jarray;
pub type jweak = jobject;
```

You'll notice that all of these are type aliases for existing signatures. In
reality, JNI only passes down `jobject`, `jarray`, and various primitive types.
Whenever possible, you should use the `jni-rs` type aliases above to take advantage of
stronger typing and methods in the `jni-rs` project.

## LookingGlass - Parameters and Return Types

Since `test_call` in `LookingGlass` is static and takes no additional arguments,
the function should be written as:

```rust
use jni::objects::JClass;
use jni::JNIEnv;

pub fn Java_com_github_imaginarypackagename_LookingGlass_test_1call(
    _env: JNIEnv,
    _class: JClass
) {}
```

Although no arguments are used in this function, the first two arguments must
always be in the native method's signature. The return type is void, and so
nothing needs to be added to the Rust signature.

Hypothetically, if the method returned an integer, it would look something like
this:

```rust
use jni::objects::JClass;
use jni::sys::jint;
use jni::JNIEnv;

pub fn Java_com_github_imaginarypackagename_LookingGlass_test_1call_1with_1jint(
    _env: JNIEnv,
    _class: JClass
) -> jint {
    123 as jint
}
```

By the way, `LookingGlass` is almost done. Specifying ABIs and Disabling
Mangling is last, and much less work.

## Specifying the ABI and Disabling Mangling
ABIs (Application Binary Interfaces) standardize low-level details, so that
object code built using different compilers may still rely upon one another. In
Rust, functions default to using the "Rust" ABI, which is usually desirable, but
not for shared libraries.

Therefore, it's necessary to explicitly set the ABI using `pub extern "system"`.
For further information, see https://doc.rust-lang.org/std/keyword.extern.html
and https://doc.rust-lang.org/beta/reference/items/external-blocks.html#abi

Lastly, name mangling is a compiler technique that assigns a unique name to each
function. FFI functions must have mangling disabled using `#[no_mangle]`, or
else the name would be transformed beyond what `JNI` expects.


## LookingGlass - Specifying the ABI and Disabling Mangling

First step, specify the ABI.

```rust
use jni::objects::JClass;
use jni::JNIEnv;

// Added pub extern "system"
pub extern "system" fn Java_com_github_imaginarypackagename_LookingGlass_test_1call(
    _env: JNIEnv,
    _class: JClass
) {}
```

Second, disable mangling.

```rust
use jni::objects::JClass;
use jni::JNIEnv;

// Added no_mangle
#[no_mangle]
pub extern "system" fn Java_com_github_imaginarypackagename_LookingGlass_test_1call(
    _env: JNIEnv,
    _class: JClass
) {}
```

With that, we're done with `LookingGlass`. If you so desired, you could now

# Dividing with Native Code

Now that you've seen an example applied via `LookingGlass`, it's time to get
familiar with writing your own signatures. On the Java side, add a signature
to `NativeAPI` with this signature:

```java 
static native int divide(int a, int b);
```

Your goal is to implement this method in Rust. If you get stuck, refer to the
walkthrough below: however, try to make it as far as you can by following the
rules described with the `LookingGlass` example.

## Rust-side Walkthrough

Next, we'll add the Rust code that handles division. Add these imports to the
Rust project, wherever you like:

```rust
use jni::objects::{JClass, JObject, JValue};

use jni::sys::jint;
use jni::JNIEnv;
```

Now, we're going to walk through some steps to transform a `fn divide(i32,i32)->i32` into one that works with JNI. Start with this:

```rust
pub fn divide(a: i32, b:i32) -> i32 {
    a/b
}
```

Follow the function naming rules discussed earlier:

```rust
pub fn Java_com_github_jni_1rs_jnibook_NativeAPI_divide(jint a, jint b) -> jint {
    a/b
}
```

Normally, Rust mangles the name of functions as part of the compilation step.
Set `#[no_mangle]` to ensure that doesn't happen.

```rust
#[no_mangle]
pub fn Java_com_github_jni_1rs_jnibook_NativeAPI_divide(jint a, jint b) -> jint {
    a/b
}
```

Add the required JNI function arguments, and the two `jint` parameters:

```rust
#[no_mangle]
pub fn Java_com_github_jni_1rs_jnibook_NativeAPI_divide(
    _env: JNIEnv,
    _class: JClass,
    jint a, 
    jint b) -> jint {
    a/b
}
```

Finally, set the ABI using `pub extern “system”`. 

```rust
#[no_mangle] 
pub extern "system" fn Java_com_github_jni_1rs_jnibook_NativeAPI_divide(
    _env: JNIEnv,
    _class: JClass,
    a: jint,
    b: jint
) -> jint {
    a/b
}
```

Now that you're probably done, write a unit test on the Java side to verify that
division works as expected.

# Tip: JNIEnv Lifetimes

`JNIEnv` instances share the lifetime of the calling thread, which means you
should avoid holding onto `JNIEnv` instances. Once the calling thread is
deallocated, it the `JNIEnv` instance will no longer be valid. [JNI00]

[JNI00]: https://docs.oracle.com/en/java/javase/11/docs/specs/jni/design.html
