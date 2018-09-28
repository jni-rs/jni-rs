#![cfg(feature = "invocation")]
#![feature(test)]

extern crate test;

extern crate jni;
#[macro_use]
extern crate lazy_static;


use jni::{InitArgsBuilder, JavaVM, JNIEnv, JNIVersion};
use jni::descriptors::Desc;
use jni::objects::{JClass, JMethodID, JObject, JStaticMethodID, JValue};
use jni::signature::{JavaType, Primitive};
use jni::sys::jint;

static CLASS_MATH: &str = "java/lang/Math";
static CLASS_OBJECT: &str = "java/lang/Object";
static METHOD_MATH_ABS: &str = "abs";
static METHOD_OBJECT_HASH_CODE: &str = "hashCode";
static METHOD_CTOR: &str = "<init>";
static SIG_OBJECT_CTOR: &str = "()V";
static SIG_MATH_ABS: &str = "(I)I";
static SIG_OBJECT_HASH_CODE: &str = "()I";


#[inline(never)]
fn native_abs(x: i32) -> i32 {
    x.abs()
}

fn jni_abs_safe(env: &JNIEnv, x: jint) -> jint {
    let x = JValue::from(x);
    let v = env.call_static_method(CLASS_MATH, METHOD_MATH_ABS, SIG_MATH_ABS, &[x]).unwrap();
    v.i().unwrap()
}

fn jni_call_static_unsafe<'c, C>(env: &JNIEnv<'c>, class: C, method_id: JStaticMethodID<'c>, x: jint) -> jint
    where
        C: Desc<'c, JClass<'c>>,
{
    let x = JValue::from(x);
    let ret = JavaType::Primitive(Primitive::Int);
    let v = env.call_static_method_unsafe(class, method_id, ret, &[x]).unwrap();
    v.i().unwrap()
}

fn jni_hash_safe(env: &JNIEnv, obj: JObject) -> jint {
    let v = env.call_method(obj, METHOD_OBJECT_HASH_CODE, SIG_OBJECT_HASH_CODE, &[]).unwrap();
    v.i().unwrap()
}

fn jni_call_unsafe<'m, M>(env: &JNIEnv<'m>, obj: JObject, method_id: M) -> jint
    where
        M: Desc<'m, JMethodID<'m>>,
{
    let ret = JavaType::Primitive(Primitive::Int);
    let v = unsafe {
        env.call_method_unsafe(obj, method_id, ret, &[]).unwrap()
    };
    v.i().unwrap()
}


#[cfg(test)]
mod tests {
    use std::rc::Rc;
    use std::sync::Arc;
    use test::{Bencher, black_box};
    use super::*;

    lazy_static! {
        static ref VM: JavaVM = {
            let args = InitArgsBuilder::new()
                .version(JNIVersion::V8)
                .build().unwrap();
            JavaVM::new(args).unwrap()
        };
    }

    #[bench]
    fn native_call_function(b: &mut Bencher) {
        b.iter(|| {
            let _ = native_abs(black_box(-3));
        });
    }

    #[bench]
    fn jni_call_static_method_safe(b: &mut Bencher) {
        let env = VM.attach_current_thread().unwrap();

        b.iter(|| jni_abs_safe(&env, -3));
    }

    #[bench]
    fn jni_call_static_method_unsafe_str(b: &mut Bencher) {
        let env = VM.attach_current_thread().unwrap();
        let class = CLASS_MATH;
        let method_id = env.get_static_method_id(class, METHOD_MATH_ABS, SIG_MATH_ABS).unwrap();

        b.iter(|| jni_call_static_unsafe(&env, class, method_id, -3));
    }

    #[bench]
    fn jni_call_static_method_unsafe_jclass(b: &mut Bencher) {
        let env = VM.attach_current_thread().unwrap();
        let class: JClass = CLASS_MATH.lookup(&env).unwrap();
        let method_id = env.get_static_method_id(class, METHOD_MATH_ABS, SIG_MATH_ABS).unwrap();

        b.iter(|| jni_call_static_unsafe(&env, class, method_id, -3));
    }

    #[bench]
    fn jni_call_object_method_safe(b: &mut Bencher) {
        let env = VM.attach_current_thread().unwrap();
        let s = env.new_string("").unwrap();
        let obj = black_box(JObject::from(s));

        b.iter(|| jni_hash_safe(&env, obj));
    }

    #[bench]
    fn jni_call_object_method_unsafe(b: &mut Bencher) {
        let env = VM.attach_current_thread().unwrap();
        let s = env.new_string("").unwrap();
        let obj = black_box(JObject::from(s));
        let method_id = env.get_method_id(
            obj, METHOD_OBJECT_HASH_CODE, SIG_OBJECT_HASH_CODE).unwrap();

        b.iter(|| jni_call_unsafe(&env, obj, method_id));
    }

    #[bench]
    fn jni_new_object_str(b: &mut Bencher) {
        let env = VM.attach_current_thread().unwrap();
        let class = CLASS_OBJECT;

        b.iter(|| {
            let obj = env.new_object(class, SIG_OBJECT_CTOR, &[]).unwrap();
            env.delete_local_ref(obj).unwrap();
        });
    }

    #[bench]
    fn jni_new_object_by_id_str(b: &mut Bencher) {
        let env = VM.attach_current_thread().unwrap();
        let class = CLASS_OBJECT;
        let ctor_id = env.get_method_id(class, METHOD_CTOR, SIG_OBJECT_CTOR).unwrap();

        b.iter(|| {
            let obj = env.new_object_by_id(class, ctor_id, &[]).unwrap();
            env.delete_local_ref(obj).unwrap();
        });
    }

    #[bench]
    fn jni_new_object_jclass(b: &mut Bencher) {
        let env = VM.attach_current_thread().unwrap();
        let class: JClass = CLASS_OBJECT.lookup(&env).unwrap();

        b.iter(|| {
            let obj = env.new_object(class, SIG_OBJECT_CTOR, &[]).unwrap();
            env.delete_local_ref(obj).unwrap();
        });
    }

    #[bench]
    fn jni_new_object_by_id_jclass(b: &mut Bencher) {
        let env = VM.attach_current_thread().unwrap();
        let class: JClass = CLASS_OBJECT.lookup(&env).unwrap();
        let ctor_id = env.get_method_id(class, METHOD_CTOR, SIG_OBJECT_CTOR).unwrap();

        b.iter(|| {
            let obj = env.new_object_by_id(class, ctor_id, &[]).unwrap();
            env.delete_local_ref(obj).unwrap();
        });
    }

    #[bench]
    fn jni_new_global_ref(b: &mut Bencher) {
        let env = VM.attach_current_thread().unwrap();
        let class = CLASS_OBJECT;
        let obj = env.new_object(class, SIG_OBJECT_CTOR, &[]).unwrap();
        let global_ref = env.new_global_ref(obj).unwrap();
        env.delete_local_ref(obj).unwrap();

        b.iter(|| env.new_global_ref(global_ref.as_obj()).unwrap());
    }

    /// Checks the overhead of checking if exception has occurred.
    ///
    /// Such checks are required each time a Java method is called, but
    /// can be omitted if we call a JNI method that returns an error status.
    ///
    /// See also #58
    #[bench]
    fn jni_check_exception(b: &mut Bencher) {
        let env = VM.attach_current_thread().unwrap();

        b.iter(|| {
            env.exception_check().unwrap()
        });
    }

    #[bench]
    fn jni_get_java_vm(b: &mut Bencher) {
        let env = VM.attach_current_thread().unwrap();

        b.iter(|| {
            let _jvm = env.get_java_vm().unwrap();
        });
    }

    /// A benchmark measuring Push/PopLocalFrame overhead.
    ///
    /// Such operations are *required* if one attaches a long-running
    /// native thread to the JVM because there is no 'return-from-native-method'
    /// event when created local references are freed, hence no way for
    /// the JVM to know that the local references are no longer used in the native code.
    #[bench]
    fn jni_noop_with_local_frame(b: &mut Bencher) {
        // Local frame size actually doesn't matter since JVM does not preallocate anything.
        const LOCAL_FRAME_SIZE: i32 = 32;
        let env = VM.attach_current_thread().unwrap();
        b.iter(|| {
            env.with_local_frame(LOCAL_FRAME_SIZE, || Ok(JObject::null()))
                .unwrap()
        });
    }

    /// A benchmark of the overhead of attaching and detaching a native thread.
    ///
    /// It is *huge* — two orders of magnitude higher than calling a single
    /// Java method using unchecked APIs (e.g., `jni_call_static_unsafe`).
    ///
    #[bench]
    fn jvm_noop_attach_detach_native_thread(b: &mut Bencher) {
        b.iter(|| {
            let env = VM.attach_current_thread().unwrap();
            black_box(&env);
        });
    }

    #[bench]
    fn native_arc(b: &mut Bencher) {
        let env = VM.attach_current_thread().unwrap();
        let class = CLASS_OBJECT;
        let obj = env.new_object(class, SIG_OBJECT_CTOR, &[]).unwrap();
        let global_ref = env.new_global_ref(obj).unwrap();
        env.delete_local_ref(obj).unwrap();
        let arc = Arc::new(global_ref);

        b.iter(|| {
            let _ = black_box(Arc::clone(&arc));
        });
    }

    #[bench]
    fn native_rc(b: &mut Bencher) {
        let _env = VM.attach_current_thread().unwrap();
        let env = VM.get_env().unwrap();
        let rc = Rc::new(env);

        b.iter(|| {
            let _ = black_box(Rc::clone(&rc));
        });
    }
}
