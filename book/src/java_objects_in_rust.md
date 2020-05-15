# Java Objects in Rust

At this point, suppose we’ve concluded that we need to run `counter_increment`
on a large number of objects at once. We might add an API to the Java side like
so:


```java
class NativeAPI {
    ...
    static native void increment_all(NativeCounter[] counters)
}
```

On the Rust side, our corresponding signature will be:

```rust
#[no_mangle]
pub extern "system" fn Java_com_github_jni_1rs_jnibook_NativeAPI_increment_all(
    env: JNIEnv,
    _class: JClass,
    counters: jobjectArray,
) {
    let length = env.get_array_length(pointers)
        .expect("TODO: proper error handling");
    for index in 0..length {
        let item = env.get_object_array_element(counters)
            .expect("TODO: proper error handling");
        let pointer = env.get_field(item, "ptr", "Z")
            .expect("TODO: proper error handling");
        // TODO: Obtain the Rust object and invoke it.
    }
}
```

## Exercises

1. Fill in the code to invoke `increment()` on the underlying Rust object.
2. If this code is run with a large enough object array, it will crash. That’s
   because the call to `get_object_array_element` creates a local reference, and
   only a small number of local references are permitted at a time (the exact
   number may vary based on your OS). Fix the code using one of the following
   approaches:
    1. Releasing the local reference when you’re done with it, using
       `[delete_local_ref](https://docs.rs/jni/0.15.0/jni/struct.JNIEnv.html#method.delete_local_ref)`,
       as you might in C++.
    2. Give each iteration its own local reference frame using
       `[with_local_frame](https://docs.rs/jni/0.15.0/jni/struct.JNIEnv.html#method.with_local_frame)`,
       so that all references within that frame are released by the next
       iteration.
    3. Use the
       `[AutoLocal](https://docs.rs/jni/0.15.0/jni/struct.JNIEnv.html#method.auto_local)`
       helper implemented within the JNI crate, so you don’t have to manually
       track the usage of that reference at all.

## Notes

The
[`AutoLocal`](https://docs.rs/jni/0.15.0/jni/struct.JNIEnv.html#method.with_local_frame)
wrapper uses Rust’s version of RAII (Resource Acquisition Is Initialization) to
call `delete_local_ref` when the object goes out of scope. By relying on the
compiler’s ownership and borrowing checks, AutoLocal safely tracks the local
usage of your objects and prevents you from releasing a reference too early. How
does it do that? `AutoLocal` takes ownership of the reference when you
initialize it, so you can’t have any other references to one (without using
unsafe code). The method `AutoLocal.as_obj()` will hand out `JObject` references
*only as long as* those references are shorter-lived than the `AutoLocal`
itself.

This is a contrived example (we would probably convert to `long` on the Java
side), but many non-trivial uses of JNI will end up interacting with objects in
a loop. When you need to do that, consider wrapping your objects in `AutoLocal`
to efficiently and safely track local references and avoid overflowing the
stack.

