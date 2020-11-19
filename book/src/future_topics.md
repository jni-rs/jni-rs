# Future Topics

The Counter exercise needs a follow-up that discusses in more detail how to
eliminate data races, use-after-free, double-frees, and guarantee that native
resources are eventually freed. This has been deferred at the moment, in favor
of focusing more on the Rust side.

jni-rs has additional abstractions that haven't been discussed, like executors.
JNI also has a few features that need their own sections (critical APIs, and
local and global references).

There are also performance optimizations that haven't been covered. It is more
efficient to issue calls from Java to native code, than it is to go from native
code to Java, and it's possible to cache class/methodIds/fieldIds.

There is open work that could simplify portions of the book. For example, [a
native peer registry](https://github.com/jni-rs/jni-rs/issues/84) can help
guarantee that raw pointers passed in the [counter
exercise](./counter_exercise.md) are valid and of the correct type, and
["auto-generate native Java-interfacing
files"](https://github.com/jni-rs/jni-rs/issues/81) could simplify the current
steps for creating JNI functions, which involves translating C header files into
Rust code.

There are also opportunities to talk about more about handling shared
resources between native resources and Java proxies.
