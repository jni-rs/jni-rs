# Introduction

The JNI(Java Native Interface) allows Java code that runs inside a Java Virtual
Machine (VM) to interoperate with applications and libraries written in other
programming languages, such as C, C++, assembly, and Rust. This book introduces
JNI programming with Rust via `jni-rs`, a library for working with JNI. It's
intended for people that are new to JNI, and would like to learn through
programming exercises. Suggestions, corrections and improvements are welcome and
wanted.

JNI has a lot of sharp edges. Native code needs to be written for each supported
platform so that Java can interact with native functions, and misuse can
lead to resource leaks or JVM crashes. Due to the drawbacks of JNI, it is often
worth considering alternatives (like reimplementation in one language or
IPC/RPC). Eventually, [Project
Panama](https://openjdk.java.net/projects/panama/) will make it [unnecessary to
write JNI-like
bindings](https://github.com/openjdk/panama-foreign/blob/foreign-jextract/doc/panama_jextract.md)
for some applications. If you need to write JNI code, hopefully you can have fun
doing it in Rust.

## Prerequisites
Knowledge of the following will be helpful for completing the entire book, but
don't be afraid if something is unfamiliar:

- `Result` and `Option` handling in Rust
- Defining Rust `struct`s and `fn`s
- Java exception handling
- Creating functions and classes in Java

Although `jni-rs` is compatible with JVMs since 1.5, this tutorial is written
for Java 8+. You'll also need a recent version of Cargo and Rust. To verify you
have all the tools, run these commands from your terminal:

```bash
which cargo
java -version
rustc -V
```

If any are missing, refer to the documentation for setup:

* [Cargo and Rust Installation](https://rustup.rs/)
* Java: your distribution's instructions

Finally, it's recommended that you install `cargo-watch` to rerun Java tests and
Rust tests in response to source code changes. [Follow the cargo watch
installation instructions ](https://github.com/passcod/cargo-watch) to install
it.

# JNI Environment Setup
In this section, you'll setup your environment so that you can run a Rust
function from Java. You'll begin by fetching the source code associated with
this book, which includes a minimal gradle project and cargo crate. Everything
you'll do is from scratch, since the purpose of the templates is merely to help
you validate environment variables are setup properly without other problems in
the way.


Download the `jni-rs` source using `git`.

```
git clone https://github.com/jni-rs/jni-rs
```

The starter projects are in: 

* `jni-rs/book/projects/starter/jnibookgradle`
* `jni-rs/book/projects/starter/jnibookrs`

## Build the Rust Project
Open `jnibookrs` and invoke `cargo build`. You should see a shared library in
your `target/debug` directory, which will be linked to from Java and
periodically rebuilt for the rest of the book. The name of the shared library
depends on your operating system, but it should be recognizable as containing
the string `jnibookrs`, and one of the suffixes `dylib`, `so`, or `dll` (for
OSX, GNU/Linux and Windows respectively).
## Java Setup
The book uses `jnibookjava` to refer to the Java project that you're using, such
as `jnibookgradle`. Next, you'll configure `jnibookjava` so that it can locate
native libraries.

### Locating Shared Libraries
Java needs a hint to locate shared libraries at runtime. You may provide the
hint through `build.gradle` or at the CLI.

```
test {
    systemProperty "java.library.path", "/path/to/jnibookrs/target/debug"
}
```

```bash
# in the jnibookgradle directory
./gradlew test --info
```

or:

```
./gradlew test --info -Djava.library.path=/path/to/jnibookrs/target/debug
```

If an `UnsatisfiedLinkError` is thrown, then verify that your path is listed in
the exception message. If it's not, then you need to ensure that
`java.library.path` is set. Otherwise, you must ensure it contains the shared
library.

```
# An example of an UnsatisfiedLinkError with java.library.path not set

java.lang.UnsatisfiedLinkError: no jnibookrs in java.library.path: [/usr/java/packages/lib, /usr/lib64, /lib64, /lib, /usr/lib]

    at java.base/java.lang.ClassLoader.loadLibrary(ClassLoader.java:2660)
    at java.base/java.lang.Runtime.loadLibrary0(Runtime.java:829)
    at java.base/java.lang.System.loadLibrary(System.java:1867)
    at jni_rs_book.NativeAPI.<clinit>(NativeAPI.java:10)
```

Assuming that the test succeeded, configure `java.library.path` for your Java
editor as well, so that you can easily run and debug unit tests.


Finally, we're going to be using both Java and Rust in this book, and it's worth
getting a fast test loop running. Consider using [cargo-watch to rerun Java
tests and rebuild the library](https://github.com/passcod/cargo-watch), to make
things easier for you.

```
# From the jnibookrs source directory
cargo watch -w . -w ../jnibookgradle/src -x 'build' -s 'cd ../jnibookgradle && ./gradlew test --info -Djava.library.path=/absolute_path_to/jnibookrs/target/debug'
```
