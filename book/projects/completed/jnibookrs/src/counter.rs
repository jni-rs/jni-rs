// ANCHOR: complete
use jni::objects::JClass;
use jni::sys::{jint, jlong};
use jni::JNIEnv;
// TODO
// ANCHOR_END: complete


// ------ DISCUSSION SECTION ------

// Stub for discussion
#[cfg(feature="counter_discussion")]
struct Counter {
}

#[cfg(feature="counter_discussion")]
impl Counter {
    fn get(&self) -> i32 {
        0
    }
}

trait CounterTrait {

}

struct FerrisCounter {

}

impl CounterTrait for FerrisCounter {}

#[cfg(feature = "DOES_NOT_COMPILE")]
// ANCHOR: ferris_counter
fn ferris_new_1counter(_env: JNIEnv, _class: JClass) -> jlong {
    let boxed_counter: Box<dyn CounterTrait> = Box::new(FerrisCounter::new());
    Box::into_raw(boxed_counter) as jlong
}
// ANCHOR_END: ferris_counter

// Bad examples
#[cfg(feature="counter_discussion")]
// ANCHOR: discussion_2_2
/* Don't do this! */ #[no_mangle]
pub extern "system" fn Java_jni_1rs_1book_NativeAPI_counter_1get(
    _env: JNIEnv,
    _class: JClass,
    ptr: jlong,
) -> jint {
    let boxed_counter = unsafe { Box::from_raw(ptr as *mut Counter) };
    let counter_value = boxed_counter.get();
    Box::into_raw(boxed_counter);
    return counter_value;
}
// ANCHOR_END: discussion_2_2
