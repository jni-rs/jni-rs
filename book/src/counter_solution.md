# Counter Solution

This section walks through one possible implementation of a Rust-backed Counter
for Java. The Java side is purposefully oversimplified, so that we may focus on
Rust (which would not change, with a more correct Java implementation).

## Java Counter API
First, we need to add a `close()` to the counter, so that we can free native
resources.

```java
// Autocloseable is used to allow try-with-resources
class Counter implements Autocloseable {

    // Allocate native resources
    public Counter() {}
    
    // Get the current count
    public int get() {
    }

    // Increment and return the new count
    public int increment() {
    }

    // Free native resources
    public void close() {
    }
}
```

We'll also need to store a pointer to the Rust Counter struct. We'll add a `long
ptr` field, which will be initialized on construction. `ptr` will then be passed
down to each of the native methods, so that the count may be fetched or
incremented, or dropped.

```java
// Mandatory disclaimer: In practice, you'll find that the Java side is usually
// much more involved than what is presented here, so that it can avoid
// double-frees, use-after-free, and guarantee cleanup. We intentionally put this
// off for now, because good solutions won't involve any Rust code, and they ramp
// up the difficulty.
class Counter implements Autocloseable {
    // The pointer to the counter in the native (Rust's) heap.
    private final long ptr;

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
        // Note the following problems with this implementation of close:
        //
        // 1. Close is not idempotent, as encouraged in the Autocloseable docs
        // 2. The counter may be double freed.
        // 3. Close may race with calls to get or increment
        // 4. It's not guaranteed that a caller will call close.
        NativeAPI.counter_destroy(ptr);
    }
}
```

## Rust Side

Now, it's time to implement the Rust side. There are two steps:

1. Implement the `Counter` in normal Rust.
2. Introduce the JNI functions, so Java may interact with it.

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

We'll use these APIs on the Java side:

```
static native long counter_new();
static native int counter_get(long ptr);
static native int counter_increment(long ptr);
static native void counter_destroy(long ptr);
```

So, let's start by defining the native signatures: 

```rust,noplaypen
use jni::objects::JClass;
use jni::sys::{jint, jlong};
use jni::JNIEnv;

/// allocate and initialize a counter on the heap, then return a valid pointer to it.
#[no_mangle]
pub extern "system" fn Java_jni_1rs_1book_NativeAPI_counter_1new(
    _env: JNIEnv,
    _class: JClass
) -> jlong {
    unimplemented!();
}

/// Given a pointer to a counter, fetch the count. The counter should not be dropped.
#[no_mangle]
pub extern "system" fn Java_jni_1rs_1book_NativeAPI_counter_1get(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong) -> jint {
    unimplemented!();
}

/// Given a pointer to a counter, increment the count. The counter should not be dropped.
#[no_mangle]
pub extern "system" fn Java_jni_1rs_1book_NativeAPI_counter_1increment(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong) -> jint {
    unimplemented!();
}

/// Given a pointer to a counter, drop it.
#[no_mangle]
pub extern "system" fn Java_jni_1rs_1book_NativeAPI_counter_1destroy(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong) {
    unimplemented!();
}
```

## Implement Counter Destroy and New
`destroy` accepts a raw pointer to the Counter, and converts it to a `Box` using
[`Box::from_raw`](https://doc.rust-lang.org/std/boxed/struct.Box.html#method.from_raw),
and return from the method to drop it.

```rust,noplaypen
#[no_mangle]
pub extern "system" fn Java_jni_1rs_1book_NativeAPI_counter_1destroy(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong
) {
    unsafe { Box::from_raw(ptr as *mut Counter) };
}
```

`new` initializes the `Counter` on the heap with `Box::new`, then uses
 [`Box::into_raw`](https://doc.rust-lang.org/std/boxed/struct.Box.html#method.into_raw)
 to retrieve a raw pointer without calling `drop` (so, the pointer will remain
 valid until something else `drop`s it or the program exits).

```rust,noplaypen
#[no_mangle]
pub extern "system" fn Java_jni_1rs_1book_NativeAPI_counter_1new(
    _env: JNIEnv,
    _class: JClass
) -> jlong {
    let boxed_counter = Box::new(Counter::new());
    Box::into_raw(boxed_counter) as jlong
}
```

## Implement Increment and Get
There are a few different ways to implement `increment` and `get`. If you would
like to cut to the chase, the best solution is likely [`&mut Counter` and
`&Counter`](./counter_solution.md#mut-counter-and-counter).

### `Box<Counter>`
This solution is hard to maintain, since contributors must remember to ensure
that a function like `Box::into_raw` (or `mem::forget`, or `Box::leak`) is
called in each function that's logically borrowing the memory. It could also
lead to double-frees or use-after-free, since `panic` will lead to the `counter`
getting dropped prematurely. For example, consider this scenario:

1. Java creates a Counter, and owns the pointer.
2. Java calls increment. 
   a. Rust makes a `Box<Counter>`, and panics before `mem::forget`.
   b. `panic!` leads to the `Counter` being dropped.
   c. The program recovers from the panic.
3. Java reenters any function on the Counter. One of two scenarios will happen.
   a. `increment` or `get` are called (use after free)
   b. `destroy` is called again (double free)

An implementation using `Box<Counter>` looks like this:

```rust,noplaypen
#[no_mangle]
pub extern "system" fn Java_jni_1rs_1book_NativeAPI_counter_1increment(
    _env: JNIEnv,
    _class: JClass
) -> jint {
    let boxed_counter = Box::new(Counter::new());
    let counter_value = boxed_counter.increment();
    Box::into_raw(boxed_counter);
    return counter_value;
}

#[no_mangle]
pub extern "system" fn Java_jni_1rs_1book_NativeAPI_counter_1get(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) -> jint {
    let boxed_counter = Box::new(Counter::new());
    let counter_value = boxed_counter.get();
    Box::into_raw(boxed_counter);
    return counter_value;
}
```

### `ManuallyDrop`

A better solution uses
[`std::mem::ManuallyDrop`](https://doc.rust-lang.org/std/mem/struct.ManuallyDrop.html)
to ensure that the counter will not be cleaned up without the caller requesting
it [(even on panic)](https://doc.rust-lang.org/std/ops/trait.Drop.html#panics),
which we leverage here:

```rust,noplaypen
#[no_mangle]
pub extern "system" fn Java_jni_1rs_1book_NativeAPI_counter_1increment(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong
) -> jint {
    let boxed_counter = ManuallyDrop::new(unsafe { Box::from_raw(ptr as *mut Counter) });
    boxed_counter.increment()
}

#[no_mangle]
pub extern "system" fn Java_jni_1rs_1book_NativeAPI_counter_1get(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) -> jint {
    let boxed_counter = ManuallyDrop::new(unsafe { Box::from_raw(ptr as *mut Counter) });
    boxed_counter.get()
}
```

### `&mut Counter` and `&Counter`

Finally, it's best to rely on `&mut Counter` and `&Counter`, since logically
`increment` and `get` are borrowing the pointer, not taking ownership. It also
eliminates `drop` as a concern.

```rust,noplaypen
#[no_mangle]
pub extern "system" fn Java_jni_1rs_1book_NativeAPI_counter_1increment(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) -> jint {
    let counter = unsafe { &mut*(ptr as *mut Counter) };
    let counter_value = counter.increment();
    return counter_value;
}

#[no_mangle]
pub extern "system" fn Java_jni_1rs_1book_NativeAPI_counter_1get(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) -> jint {
    let counter = unsafe { &*(ptr as *mut Counter) };
    let counter_value = counter.get();
    return counter_value;
}
```

## Rust Implementation

Putting it all together, the full implementation Counter on the Rust-side looks
like:

```rust,noplaypen
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
pub extern "system" fn Java_jni_1rs_1book_NativeAPI_counter_1new(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    let boxed_counter = Box::new(Counter::new());
    Box::into_raw(boxed_counter) as jlong
}

#[no_mangle]
pub extern "system" fn Java_jni_1rs_1book_NativeAPI_counter_1increment(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) -> jint {
    let counter = unsafe { &mut*(ptr as *mut Counter) };
    let counter_value = counter.increment();
    return counter_value;
}

#[no_mangle]
pub extern "system" fn Java_jni_1rs_1book_NativeAPI_counter_1get(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) -> jint {
    let counter = unsafe { &*(ptr as *mut Counter) };
    let counter_value = counter.get();
    return counter_value;
}

#[no_mangle]
pub extern "system" fn Java_jni_1rs_1book_NativeAPI_counter_1destroy(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) {
    unsafe { Box::from_raw(ptr as *mut Counter) };
}
```

## Summary
In this exercise, we examined a few ways to wrap a Rust Counter for use from
Java. You should be familiar with how panic can interact with `drop`, and some
approaches for preventing resources from being dropped prematurely. There are
still these problems with the counter:

1. `increment` and `get` may use the counter after its been freed.
2. it can be double freed ( via multiple `close` calls).
3. or perhaps never freed (never call `close`).
4. `close()` is not idempotent, as encouraged in the [`Autocloseable`
   docs](https://docs.oracle.com/javase/8/docs/api/java/lang/AutoCloseable.html#close--).
5. it's not thread-safe; it's possible to call `get` during `destroy`, for
   example.
