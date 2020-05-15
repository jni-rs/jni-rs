# Building a Counter

This chapter introduces the boxing techniques (pun not intended) for exposing a
Rust-backed Counter through JNI.

## Define the Java Counter API
First, we need to define that API that Java will expose. For now, lets say that
you can fetch the counter and increment it by one. The counter has a few jobs:
construction, `increment()` , `get()`, and `close()`. Add a stub that looks like
this:

```java
class Counter {
    public Counter() {}

    public int get() {
    }

    public int increment() {
    }

    public void close() {
    }
}
```

Internally, the class will rely on Rust for all methods. Give it a try yourself:
create some stubs of native functions in `NativeAPI` and use them to the Counter
class.
## Counter Java Solution

The counter needs to store a pointer back to native memory, so that the native
methods can find the memory it owns. Instantiating a new `Counter` object will
allocate the native Counter, and store a pointer to it in `long ptr` within the
Java object. `ptr` will then be passed down to each of the native methods, for
fetching the counter, incrementing it, and closing it.

```java
class Counter {
    // The pointer to the counter in the native (Rust's) heap
    private long ptr;

    // Don't worry if you came up with different names for the native methods
    public Counter() {
        this.ptr = NativeAPI.counter_new();
    }

    public int get() {
        return NativeAPI.counter_get(ptr);
    }

    public int increment() {
        return NativeAPI.counter_increment(ptr);
    }

    public void close() {
        NativeAPI.counter_destroy(ptr);
    }
}
```

## Exercises

1. Does garbage collecting a `Counter` free any native memory allocated by
   `NativeAPI.new_counter`?
2. What function signatures does the Rust library need for
   `NativeAPI.counter_destroy(long)`, `NativeAPI.counter_get(long)`, and
   `NativeAPI.counter_increment(long)` to work?

## Rust Side

Now that the Java interface is defined, it's time to implement the Rust side. There are two steps:

1. Implement the `Counter` in normal Rust.
2. Introduce the JNI functions that Java uses to interact with the Counter.

### Implement the Counter

We'll implement the counter first using normal Rust.

```rust
struct Counter {
    count: i32,
}

impl Counter {
    fn new() -> Self {
        Counter {
            count: 0,
        }
    }

    fn increment(&mut self) -> i32 {
        self.count += 1;
        self.count
    }

    fn get(&self) -> i32 {
        self.count
    }
}
```

### Introducing the JNI functions for the Counter

Recall that we picked these APIs on the Java side:

```
static native long counter_new();
static native int counter_get(long ptr);
static native int counter_increment(long ptr);
static native void counter_destroy(long ptr);
```

So, let's start by defining the native signatures: 

```rust
use jni::objects::JClass;
use jni::sys::{jint, jlong};
use jni::JNIEnv;

#[no_mangle]
pub extern "system" fn Java_com_github_jni_1rs_jnibook_NativeAPI_counter_1new(
    _env: JNIEnv,
    _class: JClass
) -> jlong {
    unimplemented!();
}

#[no_mangle]
pub extern "system" fn Java_com_github_jni_1rs_jnibook_NativeAPI_counter_1get(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong) -> jint {
    unimplemented!();
}

#[no_mangle]
pub extern "system" fn Java_com_github_jni_1rs_jnibook_NativeAPI_counter_1increment(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong) -> jint {
    unimplemented!();
}

#[no_mangle]
pub extern "system" fn Java_com_github_jni_1rs_jnibook_NativeAPI_counter_1destroy(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong) {
    unimplemented!();
}
```

Now, we have a few goals for the implementation.

1. `counter_1new` should allocate a counter on the heap, and return a valid
   pointer to it.
2. `counter_1new`, `counter_1increment`, and `counter_1get` should not deallocate the counter.
3. `counter_1destroy` should deallocate the counter.

We'll need to use [`Box`](https://doc.rust-lang.org/std/boxed/index.html) to
meet these goals. `Box` is used for storing objects on the heap. Boxed objects
can't outlive their boxes, except through through `Box::into_raw` and
`Box::leak`. These two functions drop the `Box` without dropping the object, and
return a pointer or reference to it. In this case, `Box::into_raw` is more
appropriate, since we need a pointer. It will be used in every counter related
JNI function, except `counter_1destroy`.

We'll also need to be able to translate `ptr` back into a `Box` via
[`Box::from_raw`](https://doc.rust-lang.org/std/boxed/struct.Box.html#method.from_raw),
in each of `counter_1increment`, `counter_1get`, and `counter_1destroy`.
`Box::into_raw` and `Box::from_raw` are frequently necessary at language
boundaries, since they enable the other language to play a role in memory
management. By virtue of letting multiple languages play together with memory,
it's easier to make memory errors at this boundary than elsewhere - either in
pure Java, or in Pure Rust.

## Implement Counter Allocation

As described earlier, we'll allocate the Counter on the heap (through a `Box`),
and then drop the `Box` and return a raw pointer as a `jlong`, which the Java
side will store in the field `long ptr`.

```rust
#[no_mangle]
pub extern "system" fn Java_com_github_jni_1rs_jnibook_NativeAPI_counter_1new(
    _env: JNIEnv,
    _class: JClass
) -> jlong {
    let boxed_counter = Box::new(Counter::new());
    Box::into_raw(boxed_counter) as jlong
}
```

## Implement Increment and Get

To implement increment, we'll need to convert `ptr` back into a `Box<Counter>`,
and then call increment, drop the box through `into_raw` to about deallocating
the counter, and finally return the incremented value.

```rust
#[no_mangle]
pub extern "system" fn Java_com_github_jni_1rs_jnibook_NativeAPI_counter_1increment(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong
) -> jint {
    let mut boxed_counter = unsafe { Box::from_raw(ptr as *mut Counter) };
    let counter_value = boxed_counter.increment();
    Box::into_raw(boxed_counter);
    return counter_value
}
```

Implementing get is similar.

```rust
#[no_mangle]
pub extern "system" fn Java_com_github_jni_1rs_jnibook_NativeAPI_counter_1get(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong
) -> jint {
    let boxed_counter = unsafe { Box::from_raw(ptr as *mut Counter) };
    let counter_value = boxed_counter.get();
    Box::into_raw(boxed_counter);
    return counter_value
}
```

## Implement Destroy

To implement destroy, convert the pointer to a `Box`, and let it go out of
scope. The memory is freed by the `Drop` implementation of `Box`.

```rust
#[no_mangle]
pub extern "system" fn Java_com_github_jni_1rs_jnibook_NativeAPI_counter_1destroy(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong
) {
    unsafe { Box::from_raw(ptr as *mut Counter) };
```

## Full Rust Implementation

Putting it all together, the full implementation Counter looks like:

```rust
use jni::objects::JClass;
use jni::sys::{jint, jlong};
use jni::JNIEnv;

struct Counter {
    count: i32,
}

impl Counter {
    fn new() -> Self {
        Counter { count: 0 }
    }

    fn increment(&mut self) -> i32 {
        self.count += 1;
        self.count
    }

    fn get(&self) -> i32 {
        self.count
    }
}

#[no_mangle]
pub extern "system" fn Java_com_github_jni_1rs_jnibook_NativeAPI_counter_1new(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    let boxed_counter = Box::new(Counter::new());
    Box::into_raw(boxed_counter) as jlong
}

#[no_mangle]
pub extern "system" fn Java_com_github_jni_1rs_jnibook_NativeAPI_counter_1increment(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) -> jint {
    let mut boxed_counter = unsafe { Box::from_raw(ptr as *mut Counter) };
    let counter_value = boxed_counter.increment();
    Box::into_raw(boxed_counter);
    return counter_value;
}

#[no_mangle]
pub extern "system" fn Java_com_github_jni_1rs_jnibook_NativeAPI_counter_1get(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) -> jint {
    let boxed_counter = unsafe { Box::from_raw(ptr as *mut Counter) };
    let counter_value = boxed_counter.get();
    Box::into_raw(boxed_counter);
    return counter_value;
}

#[no_mangle]
pub extern "system" fn Java_com_github_jni_1rs_jnibook_NativeAPI_counter_1destroy(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) {
    unsafe { Box::from_raw(ptr as *mut Counter) };
}
```

## Summary

In this section, we've covered how to let a heap-allocated Rust object outlive
its stack for the purpose of cross-language memory management. We will return to
this API in the [Cleaning](./cleaning.md) section, where we will see how we can
make it harder to run into memory leaks, and in [Java Objects in
Rust](./java_objects_in_rust.md), where we will discuss using arrays of `Counter`.

## Exercises

1. What happens if destroy is never called, or gets called multiple times?
2. Does the memory created by Rust count towards heap size limits set through
   the JVM?
3. What are some different techniques for making the counter thread-safe?
