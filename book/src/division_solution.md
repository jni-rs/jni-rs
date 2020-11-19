## Division Exercise Solution
On the Java-side, declare a native `divide` function in `NativeAPI`. Then use
`javac` to get the name of the function that should be added to the Rust
project.

```rust,noplaypen
use jni::objects::JClass;
use jni::sys::jint;
use jni::JNIEnv;

pub fn Java_jni_1rs_1book_NativeAPI_divide(jint a, jint b) -> jint {
    a/b
}
```

Add the required `JNIEnv` and `JClass` arguments.

```rust,noplaypen
pub fn Java_jni_1rs_1book_NativeAPI_divide(
    _env: JNIEnv,
    _class: JClass,
    jint a, 
    jint b) -> jint {
    a/b
}
```

Annotate the function with `#[no_mangle]`.

```rust,noplaypen
#[no_mangle]
pub fn Java_jni_1rs_1book_NativeAPI_divide(
    _env: JNIEnv,
    _class: JClass,
    jint a, 
    jint b) -> jint {
    a/b
}
```

Specify the ABI using `pub extern "system"`.

```rust,noplaypen
#[no_mangle] 
pub extern "system" fn Java_jni_1rs_1book_NativeAPI_divide(
    _env: JNIEnv,
    _class: JClass,
    a: jint,
    b: jint
) -> jint {
    a/b
}
```

Finally, write a unit test to verify that division works as expected.

```java
import org.junit.Test;

import static org.junit.Assert.assertEquals;

public class DivisionTest {

    @Test
    public void testDivision() {
        assertEquals(NativeAPI.divide(10, 5), 2);
    }
}
```

If you also write a test in `jnibookjava` that divides by zero, and run it. It's
likely that the JVM will crash and you'll see a message similar to this:

```
thread '<unnamed>' panicked at 'attempt to divide by zero', src/division.rs:14:5
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
fatal runtime error: failed to initiate panic, error 5
```

Dividing by zero triggers a panic, which leads to undefined behavior across FFI
boundaries.
