## Division Exercise Solution
On the Java-side, declare a native `divide` function in `NativeAPI`. Then use
`javac` to get the name of the function that should be added to the Rust
project.

```rust,noplaypen
{{#include ../projects/starter/jnibookrs/src/division.rs:imports}}

{{#include ../projects/starter/jnibookrs/src/division.rs:division_0}}
```

Add the required `JNIEnv` and `JClass` arguments.

```rust,noplaypen
{{#include ../projects/completed/jnibookrs/src/division.rs:division_1}}
```

Annotate the function with `#[no_mangle]`.

```rust,noplaypen
{{#include ../projects/completed/jnibookrs/src/division.rs:division_2}}
```

Specify the ABI using `pub extern "system"`.

```rust,noplaypen
{{#include ../projects/completed/jnibookrs/src/division.rs:division_3}}
```

Finally, write a unit test to verify that division works as expected.

```java
{{#include ../projects/completed/jnibookgradle/src/test/java/jni_rs_book/DivisionTest.java:happy_path}}
}
```

If you also attempt to divide by 0, it's likely that the JVM will crash. This is
due to a panic across the FFI boundary, which leads to undefined behavior.

```java
{{#include ../projects/completed/jnibookgradle/src/test/java/jni_rs_book/DivisionTest.java:divide_by_zero}}
```

Instead of the test passing, you'll see a message similar to this:

```
thread '<unnamed>' panicked at 'attempt to divide by zero', src/division.rs:14:5
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
fatal runtime error: failed to initiate panic, error 5
```
