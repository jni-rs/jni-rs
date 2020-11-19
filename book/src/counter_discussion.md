# Counter Discussion

1. Ferris has a `Box<dyn CounterTrait>` that they would like to use from Java.
Can they pass it to Java, like this? Why or why not? If not, does the error
surface at compile time, or runtime?

```rust
fn ferris_new_1counter(_env: JNIEnv, _class: JClass) {
    let boxed_counter: Box<dyn CounterTrait> = Box::new(FerrisCounter::new());
    Box::into_raw(boxed_counter) as jlong
}
```

2. Whats wrong with this?
   
```rust 
#[no_mangle]
pub extern "system" fn Java_jni_1rs_1book_NativeAPI_counter_1get(
    _env: JNIEnv,
    _class: JClass,
    ptr: &Counter,
) -> jint {
     counter.get()
}
```
