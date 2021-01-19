# Introduction

The JNI(Java Native Interface) allows Java code that runs inside a Java Virtual
Machine (VM) to interoperate with applications and libraries written in other
programming languages, such as C, C++, assembly, and Rust. This book introduces
JNI programming with Rust via `jni-rs`, a library for working with JNI. It's
intended for people that are new to JNI, and would like to learn through
programming exercises. Suggestions, corrections and improvements are welcome and
wanted.

JNI has a lot of sharp edges. Special code needs to be written so that Java can
call native functions (which must be built for each supported platform), and
misuse can lead to resource leaks or JVM crashes. Due to the drawbacks of JNI,
it is often worth considering alternatives (like reimplementation in one
language or IPC/RPC). Eventually, [Project
Panama](https://openjdk.java.net/projects/panama/) may make it unnecessary to
write JNI code for some applications. If you need to write JNI code, hopefully
you can have fun doing it in Rust.

## Prerequisites
With respect to experience, only the basics of Rust and Java are necessary to
finish the tutorial. It helps to be good at debugging Java Exceptions.

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
The document uses `jnibookjava` to refer to the Java project that you're using,
such as `jnibookgradle`. Next, you'll configure `jnibookjava` so that it can
locate native libraries.

### Locating Shared Libraries
Java needs a hint to be able to locate shared libraries at runtime. To do so,
you may either set it in `build.gradle` or at the CLI.

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

If you get an `UnsatisfiedLinkError`, like below:

```
java.lang.UnsatisfiedLinkError: no jnibookrs in java.library.path: [/usr/java/packages/lib, /usr/lib64, /lib64, /lib, /usr/lib]

    at java.base/java.lang.ClassLoader.loadLibrary(ClassLoader.java:2660)
    at java.base/java.lang.Runtime.loadLibrary0(Runtime.java:829)
    at java.base/java.lang.System.loadLibrary(System.java:1867)
    at com.github.jni_rs.jnibook.NativeAPI.<clinit>(NativeAPI.java:10)
```

Then verify that your path is listed in the Exception message. If it's not, then you
need to ensure that `java.library.path` is set. Otherwise, you must ensure it
contains the shared library.

Assuming that the test succeeded, configure the path for your Java editor as
well, so that you can easily run and debug unit tests.
