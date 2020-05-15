# Introduction
This book is about building shared libraries in Rust that work with Java via JNI
(Java Native Interface). JNI is a Java API for using shared libraries from Java.
The book is intended for people that know a bit of Rust and Java, and want to
learn how to use them together in-process, hopefully without hitting their head
against the wall. Suggestions, corrections and improvements are welcome and
wanted.

JNI, regardless of the language used to build the shared library, comes with a
lot of sharp edges. The shared library needs to be built for each platform you
support, you have to figure out a distribution strategy for them (e.g., have the
consumer unzip the jar and put in on their load path), and JNI misuse can lead
to resource leaks or JVM crashes. There are also performance implications of
using JNI in certain ways. For example, Java Strings and Buffers sometimes have
to be copied when they're used in the shared library. Due to the drawbacks of
JNI, it's usually more appropriate to build using pure Java.

There are plenty of reasons why you should consider Rust if you need to build a
shared library for Java. Most notably, you can separate JNI code into a specific
layer, so that all benefits you get from Rust in normal application or library
development apply to the core. Within the JNI layer itself, similar benefits
still apply with the caveat that JNI API misuse is still highly possible, and
unsafe Rust is often necessary. Expect the JNI code for Rust to look similar to
what you would see in C, with additional safety when calling into the core of
your library from the Rust JNI functions.

You should check if any of Rust's limitations outweigh the benefits as well.
Shared libraries will be larger and take longer to build than what you would get
from gcc, some systems will never have Rust debuggers, and it's possible that
you need wider platform support than what Rust supports today. If these problems
outweigh the benefits for you, then C may be better option, unless you are
interested in improving the toolchain.

## Prerequisites
For Java, you need experience with defining classes. For Rust, you should have
experience with `Box`, and defining `struct`s and `function`s.

Your system needs Java 11+, recent versions of Cargo and Rust, and one Java
build tool of your choice. The book has Java starter code that relies on
`gradle`, which you can easily adapt to a different build system. It should also
have Java debugger support, preferably exposed to your IDE. You can get by with
Java 8 or greater if you don't mind skipping *Cleaning Up Resource Leaks* or
*Debugging*.

To verify you have everything, run these commands from your terminal:

```bash
which cargo
java -version
rustc -V
gradle -v
```

If any are missing, refer to these pages:

* [Gradle Installation](https://docs.gradle.org/current/userguide/installation.html) (optional, should you choose to adapt the starter code):
* [Cargo and Rust Installation](https://rustup.rs/):
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

* `jni-rs/book/projects/jnibookgradle`
* `jni-rs/book/projects/jnibookrs`

## Build the Rust Project
Open `jnibookrs` and invoke `cargo build`. You should see a shared library in
your `target/debug` directory, which will be linked to and periodically rebuilt
for the rest of the book.
## Java Setup
The document uses `jnibookjava` to refer to the Java project that you're using,
such as `jnibookgradle`. Next, you'll configure `jnibookjava` so that it can
locate locate your shared libraries. This step is very platform dependent. If
you're using Linux, Windows, or OSX Yosemite or older, then you can proceed to
"Java Environment Variables."

### OSX El Capitan and Newer Instructions
This section is only provided as a note for users on El Capitan and newer. It
should only interest you if the JNI software will run on OSX, such as for
testing purposes.

System Integrity Protection (SIP) is a feature that (among other things)
prevents `DYLD_LIBRARY_PATH` and `LD_LIBRARY_PATH` from getting passed to child
processes, so setting these variables as the next section recommends is
pointless with it enabled. Don't disable it just to learn JNI.

Instead, set `java.library.path`. The reason this isn't the recommended method
is because Java properties may only be set once. Which means multiple JNI
libraries would need to compete for the same resource. Fortunately, that doesn't
matter for the exercises here. To do so, update `build.gradle` to have this
definition:

```
// Only bother with this if you're on Yosemite or newer.
test {
    systemProperty "java.library.path", "/path/to/jnibookrs/target/debug"
}
```

Similar settings would need to be applied for other build systems. Next, run the tests:

```bash
gradle test --info
```

If the test passes, continue on to Java Environment Variables. Otherwise, refer
to Troubleshooting in the Appendix. 

### Java Environment Variables

The environment variables that Java uses to find your shared library are very
platform-specific.

1. On Linux and Windows, it's called `LD_LIBRARY_PATH`.
2. On OSX, it's `DYLD_LIBRARY_PATH`. (As a reminder, SIP can interfere with setting this.)

For simplicity, the rest of the document only refers to `LD_LIBRARY_PATH`. If
you're targeting Mac OSX, then you can safely treat is as `DYLD_LIBRARY_PATH`.

Java doesn't support relative paths or interpolation in `LD_LIBRARY_PATH`. That
means that when you configure the path, the string Java receives must not
include `..`, `$`, or `~`. Ideally, the path should be as boring as possible. If
you have readlink, then you may use it to interpolate these paths for Java.

```bash
# If you are inclined, you can tinker with the path and see
# some exceptions on load failure.
LD_LIBRARY_PATH=/path/to/jnibookrs/target/debug gradle test --info
```

```bash
# If you have readlink, it can save you the effort of
# finding the absolute path yourself.
LD_LIBRARY_PATH=`readlink -m ../jnibookrs/target/debug` gradle test --info
```

Optionally, set `RUST_BACKTRACE=1` so that Rust provides stacktraces during
development. When you invoke code from Java, it should typically look something
like this:

```bash
RUST_BACKTRACE=1 LD_LIBRARY_PATH=/path/to/jnibookrs/target/debug gradle test --info 
```

```bash
# with readlink, to resolve the directory to an absolute path.
RUST_BACKTRACE=1 LD_LIBRARY_PATH=`readlink -m ../jnibookrs/target/debug` gradle test --info 
```

Assuming that the test succeeded, you're done! Consider setting up the same
configuration for your Java editor.

Use the Appendix for instructions on:
* [Troubleshooting java.lang.UnsatisfiedLinkError](./appendix.md#javalangunsatisfiedlinkerror)
* [IntelliJ guided setup](./appendix.md#intellij-guided-setup)
