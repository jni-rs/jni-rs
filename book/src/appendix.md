# Appendix
The appendix currently contains troubleshooting and setup information. In the
future, it may be expanded to include other details.
## java.lang.UnsatisfiedLinkError
```
java.lang.UnsatisfiedLinkError: no jnibookrs in java.library.path: [/usr/java/packages/lib, /usr/lib64, /lib64, /lib, /usr/lib]

    at java.base/java.lang.ClassLoader.loadLibrary(ClassLoader.java:2660)
    at java.base/java.lang.Runtime.loadLibrary0(Runtime.java:829)
    at java.base/java.lang.System.loadLibrary(System.java:1867)
    at com.github.jni_rs.jnibook.NativeAPI.<clinit>(NativeAPI.java:10)
```

Read the directories listed in the Exception message. It tells you where
Java looked for the native library. If you don’t see your path listed, then you
need to try a different variable. Otherwise, you must ensure it contains the
native library.

If you're using OS X, check whether you're using a recent version. Since El
Capitan, SIP prevents setting `LD_LIBRARY_PATH`. If you aren't sure which
version you're using, you can click the Apple icon and "About this Mac" to find
out. Refer to [MacOS version
history](https://en.wikipedia.org/wiki/MacOS_version_history) to tell whether
yours is newer than El Capitan.

Your Mac most likely doesn't allow you to set `LD_LIBRARY_PATH` if you bought it
new after 2014. If you're on OSX El Capitan or newer (and have SIP enabled),
ensure you're using `java.library.path` and are setting it as a Java property.
If you're on older versions of OSX, ensure you're using `DYLD_LIBRARY_PATH` and
not `LD_LIBRARY_PATH`.

## Setting Java Properties

This section provides further details on how to set Java properties for various
Java build systems.

### Gradle
Java properties can be set for tests, like so:

```
test {
    // Only bother with this if you're on Yosemite or newer.
    systemProperty "java.library.path", "/path/to/jnibookrs/target/debug"
}
```

## IntelliJ Guided Setup

This section covers setting environment variables for IntelliJ. Setup your run
configuration to use the same variables and values as the CLI. Then, run a unit
test to verify that everything works as expected.

```bash
# For Linux/Windows:
LD_LIBRARY_PATH=/path/to/jnibookrs/target/debug;RUST_BACKTRACE=1
# For OSX:
DYLD_LIBRARY_PATH=/path/to/jnibookrs/target/debug;RUST_BACKTRACE=1
```

If you need to set `java.library.path` instead, then set it as a Java property
instead of as an environment variable.

## Debugging
### CLion Instructions
If you're using CLion, follow ["Attaching to a Local
   Process"](https://www.jetbrains.com/help/clion/attaching-to-local-process.html)


## Function Linking Rules The Hard Way

JNI identifies the functions it should link to using the package path, class
name, and method name by following [these
rules](https://docs.oracle.com/en/java/javase/11/docs/specs/jni/design.html).
Since `.` isn’t allowed in C function names, the Java function's package path's
`.` is mapped to `_`. However, since `_` could appear in a function name, it
needs a different representation. So `_` gets replaced by `_1` before replacing
`.` with `_`. And finally, the function name has to be prefixed with `Java_`, to
avoid conflicts with other languages that support similar naming schemes.

### LookingGlass - Function Linking Rules

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
