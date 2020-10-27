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
To get a native Java function to correctly bind to a Rust function underneath,
we need to ensure that it's named properly, and that the parameters and return
type match. First, we'll discuss getting the Rust function name.

### Naming the Rust Function
JNI has a long list of rules that it uses to encode the package path, class name
and function name into a valid C function name. Fortunately, you don't have to
learn them all, as `javac` can be used to generate a C function for you, which
you can then copy for use with Rust. For this example, we'll work with
`NativeAPI` and the `verify_link` function.

To get a name for a function, follow these steps.

1. Add a `native` function to a Java class. Remember that the package path,
   class name, and function name are all encoded into the C function name, so
   make sure those are as you want them. 
2. Run `javac -h . NativeAPI.java` to produce C headers. (We assume that the
   class resides in `NativeAPI.java`, but it doesn't have to.)
3. Copy the C function name out of the header file it produced.

Upon completing these steps, you will see a file called
`com_github_jni_rs_jnibook_NativeAPI.h` that contains the following:

```c
/* Header for class com_github_jni_rs_jnibook_NativeAPI */

#ifndef _Included_com_github_jni_rs_jnibook_NativeAPI
#define _Included_com_github_jni_rs_jnibook_NativeAPI
#ifdef __cplusplus
extern "C" {
#endif
/*
 * Class:     com_github_jni_rs_jnibook_NativeAPI
 * Method:    verify_link
 * Signature: ()I
 */
JNIEXPORT jint JNICALL Java_com_github_jni_1rs_jnibook_NativeAPI_verify_1link
  (JNIEnv *, jclass);

#ifdef __cplusplus
}
#endif
#endif
```

`Java_com_github_jni_1rs_jnibook_NativeAPI_verify_1link` is the name of the Rust
function that corresponds to `verify_link`. `Java_` identifies that the function
is for Java, followed by the path, classname, and `verify_1link`. You may wonder
what the `_1` is for: it's the JNI way of encoding underscores, since `.` uses
`_`. Now that you know how to name the Rust functions that Java will use, we'll
discuss the parameters and return types.

### Parameters
The first argument to every JNI function is `JNIEnv`, which is an object you can
use to call into the JVM. The second argument is a reference to `this`, which is
a `JClass` for static methods and `JObject` for instance methods. The next
arguments map 1:1 with parameters specified in the Java method's signature. For
example, if the signature was `void add(int a, int b)`, then the 3rd and 4th
arguments in the native function would be of type `jni::sys::jint`.

Since `verify_link` is static and takes no additional arguments, the function
should be written as:

```rust
use jni::objects::JClass;
use jni::JNIEnv;

// Although no arguments are used in this function, the first two arguments must
// always be in the native method's signature.
pub fn Java_com_github_jni_1rs_jnibook_NativeAPI_verify_1link(
    _env: JNIEnv,
    _class: JClass
) {}
```

Now that the parameters are correct, we need to give some hints to the compiler
to ensure that none of our work is undone. The next section wraps up the last of
the details we need to cover for calling Rust from Java.

### Specifying the ABI and Disabling Mangling
Thus far, we've given the Rust function a very specific name. Since the compiler
leverages a technique called name mangling that assigns a unique name to each
function, we have to annotate JNI methods with `#[no_mangle]` to ensure that the
name is exactly as we've specified. Otherwise, the name would be transformed to
something that Java wouldn't expect.

Secondly, we need to specify the ABI. ABIs (Application Binary Interfaces)
standardize low-level details, so that object code built using different
compilers may still rely upon one another. In Rust, functions default to using
the "Rust" ABI, which is usually desirable, but not for calling Rust from Java.

Therefore, it's necessary to explicitly set the ABI using `pub extern "system"`.
For further information, see
[extern](https://doc.rust-lang.org/std/keyword.extern.html) and
[abi](https://doc.rust-lang.org/beta/reference/items/external-blocks.html#abi).

Applying these two rules, we wind up with:

```rust
use jni::objects::JClass;
use jni::JNIEnv;

// Added no_mangle
#[no_mangle]
pub extern "system" fn Java_com_github_jni_1rs_jnibook_NativeAPI_verify_1link(
    _env: JNIEnv,
    _class: JClass
) {}
```

With that, we're done. You'll find that’s exactly what the Rust starter code
contains.

# Dividing with Native Code

Now that you've seen an example applied via `verify_link`, it's time to get
familiar with writing your own signatures. On the Java side, add a signature to
`NativeAPI` with this signature:

```java 
static native int divide(int a, int b);
```

Your goal is to implement this method in Rust. If you get stuck, refer to the
walkthrough below: however, try to make it as far as you can by following the
previous example.

## Rust-side Walkthrough

Next, we'll add the Rust code that handles division. Add these imports to the
Rust project, wherever you like:

```rust
use jni::objects::{JClass, JObject, JValue};

use jni::sys::jint;
use jni::JNIEnv;
```

On the Java-side, declare `static native int divide(int a, int b);` in
`NativeAPI`. Then use javac to get the function name:

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

Add the required JNI function arguments, the two `jint` parameters, and the
return type:

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

Finally, specify the ABI using `pub extern "system"`.

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
deallocated, the `JNIEnv` instance will no longer be valid. [JNI00]

[JNI00]: https://docs.oracle.com/en/java/javase/11/docs/specs/jni/design.html
