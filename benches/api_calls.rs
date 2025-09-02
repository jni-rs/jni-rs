#![cfg(feature = "invocation")]

use jni_sys::jvalue;
use lazy_static::lazy_static;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use jni::objects::{GlobalRef, IntoAutoLocal as _};
use jni::{
    descriptors::Desc,
    env::JNIEnv,
    objects::{JClass, JMethodID, JObject, JStaticMethodID, JValue},
    signature::{Primitive, ReturnType},
    sys::jint,
    InitArgsBuilder, JNIVersion, JavaVM,
};
use std::rc::Rc;
use std::sync::Arc;

static CLASS_MATH: &str = "java/lang/Math";
static CLASS_OBJECT: &str = "java/lang/Object";
static CLASS_LOCAL_DATE_TIME: &str = "java/time/LocalDateTime";
static METHOD_MATH_ABS: &str = "abs";
static METHOD_OBJECT_HASH_CODE: &str = "hashCode";
static METHOD_CTOR: &str = "<init>";
static METHOD_LOCAL_DATE_TIME_OF: &str = "of";
static SIG_OBJECT_CTOR: &str = "()V";
static SIG_MATH_ABS: &str = "(I)I";
static SIG_OBJECT_HASH_CODE: &str = "()I";
static SIG_LOCAL_DATE_TIME_OF: &str = "(IIIIIII)Ljava/time/LocalDateTime;";

// 32 characters
static TEST_STRING_UNICODE: &str = "_񍷕㳧~δ򗊁᪘׷ġ˥쩽|ņ/򖕡ٶԦ萴퀉֒ٞHy󢕒%ӓ娎񢞊ăꊦȮ񳗌";

#[inline(never)]
fn native_abs(x: i32) -> i32 {
    x.abs()
}

fn jni_abs_safe(env: &mut JNIEnv, x: jint) -> jint {
    let x = JValue::from(x);
    let v = env
        .call_static_method(CLASS_MATH, METHOD_MATH_ABS, SIG_MATH_ABS, &[x])
        .unwrap();
    v.i().unwrap()
}

fn jni_hash_safe(env: &mut JNIEnv, obj: &JObject) -> jint {
    let v = env
        .call_method(obj, METHOD_OBJECT_HASH_CODE, SIG_OBJECT_HASH_CODE, &[])
        .unwrap();
    v.i().unwrap()
}

#[allow(clippy::too_many_arguments)]
fn jni_local_date_time_of_safe<'local>(
    env: &mut JNIEnv<'local>,
    year: jint,
    month: jint,
    day_of_month: jint,
    hour: jint,
    minute: jint,
    second: jint,
    nanosecond: jint,
) -> JObject<'local> {
    let v = env
        .call_static_method(
            CLASS_LOCAL_DATE_TIME,
            METHOD_LOCAL_DATE_TIME_OF,
            SIG_LOCAL_DATE_TIME_OF,
            &[
                year.into(),
                month.into(),
                day_of_month.into(),
                hour.into(),
                minute.into(),
                second.into(),
                nanosecond.into(),
            ],
        )
        .unwrap();
    v.l().unwrap()
}

fn jni_int_call_static_unchecked<'local, C>(
    env: &mut JNIEnv<'local>,
    class: C,
    method_id: JStaticMethodID,
    x: jint,
) -> jint
where
    C: Desc<'local, JClass<'local>>,
{
    let x = JValue::from(x);
    let ret = ReturnType::Primitive(Primitive::Int);
    let v =
        unsafe { env.call_static_method_unchecked(class, method_id, ret, &[x.as_jni()]) }.unwrap();
    v.i().unwrap()
}

fn jni_int_call_unchecked<'local, M>(
    env: &mut JNIEnv<'local>,
    obj: &JObject<'local>,
    method_id: M,
) -> jint
where
    M: Desc<'local, JMethodID>,
{
    let ret = ReturnType::Primitive(Primitive::Int);
    // SAFETY: Caller retrieved method ID + class specifically for this use: Object.hashCode()I
    let v = unsafe { env.call_method_unchecked(obj, method_id, ret, &[]) }.unwrap();
    v.i().unwrap()
}

fn jni_object_call_static_unchecked<'local, C>(
    env: &mut JNIEnv<'local>,
    class: C,
    method_id: JStaticMethodID,
    args: &[jvalue],
) -> JObject<'local>
where
    C: Desc<'local, JClass<'local>>,
{
    // SAFETY: Caller retrieved method ID and constructed arguments
    let v = unsafe { env.call_static_method_unchecked(class, method_id, ReturnType::Object, args) }
        .unwrap();
    v.l().unwrap()
}

lazy_static! {
    static ref VM: JavaVM = {
        let args = InitArgsBuilder::new()
            .version(JNIVersion::V1_8)
            .build()
            .unwrap();
        JavaVM::new(args).unwrap()
    };
}

fn native_call_function(c: &mut Criterion) {
    c.bench_function("native_call_function", |b| {
        b.iter(|| {
            let _ = native_abs(black_box(-3));
        })
    });
}

fn jni_call_static_abs_method_safe(c: &mut Criterion) {
    VM.attach_current_thread_for_scope(|env| -> jni::errors::Result<()> {
        c.bench_function("jni_call_static_abs_method_safe", |b| {
            b.iter(|| jni_abs_safe(env, -3))
        });
        Ok(())
    })
    .unwrap();
}

fn jni_call_static_abs_method_unchecked_str(c: &mut Criterion) {
    VM.attach_current_thread_for_scope(|env| -> jni::errors::Result<()> {
        let class = CLASS_MATH;
        let method_id = env
            .get_static_method_id(class, METHOD_MATH_ABS, SIG_MATH_ABS)
            .unwrap();

        c.bench_function("jni_call_static_abs_method_unchecked_str", |b| {
            b.iter(|| jni_int_call_static_unchecked(env, class, method_id, -3))
        });
        Ok(())
    })
    .unwrap();
}

fn jni_call_static_abs_method_unchecked_jclass(c: &mut Criterion) {
    VM.attach_current_thread_for_scope(|env| -> jni::errors::Result<()> {
        let class = Desc::<JClass>::lookup(CLASS_MATH, env).unwrap();
        let method_id = env
            .get_static_method_id(&class, METHOD_MATH_ABS, SIG_MATH_ABS)
            .unwrap();

        c.bench_function("jni_call_static_abs_method_unchecked_jclass", |b| {
            b.iter(|| jni_int_call_static_unchecked(env, &class, method_id, -3))
        });
        Ok(())
    })
    .unwrap();
}

fn jni_call_static_date_time_method_safe(c: &mut Criterion) {
    VM.attach_current_thread_for_scope(|env| -> jni::errors::Result<()> {
        c.bench_function("jni_call_static_date_time_method_safe", |b| {
            b.iter(|| {
                let obj = jni_local_date_time_of_safe(env, 1, 1, 1, 1, 1, 1, 1);
                env.delete_local_ref(obj);
            })
        });
        Ok(())
    })
    .unwrap();
}

fn jni_call_static_date_time_method_unchecked_jclass(c: &mut Criterion) {
    VM.attach_current_thread_for_scope(|env| -> jni::errors::Result<()> {
        let class = Desc::<JClass>::lookup(CLASS_LOCAL_DATE_TIME, env).unwrap();
        let method_id = env
            .get_static_method_id(&class, METHOD_LOCAL_DATE_TIME_OF, SIG_LOCAL_DATE_TIME_OF)
            .unwrap();

        c.bench_function("jni_call_static_date_time_method_unchecked_jclass", |b| {
            b.iter(|| {
                let obj = jni_object_call_static_unchecked(
                    env,
                    &class,
                    method_id,
                    &[
                        JValue::Int(1).as_jni(),
                        JValue::Int(1).as_jni(),
                        JValue::Int(1).as_jni(),
                        JValue::Int(1).as_jni(),
                        JValue::Int(1).as_jni(),
                        JValue::Int(1).as_jni(),
                        JValue::Int(1).as_jni(),
                    ],
                );
                env.delete_local_ref(obj);
            })
        });
        Ok(())
    })
    .unwrap();
}

fn jni_call_object_hash_method_safe(c: &mut Criterion) {
    VM.attach_current_thread_for_scope(|env| -> jni::errors::Result<()> {
        let s = env.new_string("").unwrap();
        let obj = black_box(JObject::from(s));

        c.bench_function("jni_call_object_hash_method_safe", |b| {
            b.iter(|| jni_hash_safe(env, &obj))
        });
        Ok(())
    })
    .unwrap();
}

fn jni_call_object_hash_method_unchecked(c: &mut Criterion) {
    VM.attach_current_thread_for_scope(|env| -> jni::errors::Result<()> {
        let s = env.new_string("").unwrap();
        let obj = black_box(JObject::from(s));
        let obj_class = env.get_object_class(&obj).unwrap();
        let method_id = env
            .get_method_id(&obj_class, METHOD_OBJECT_HASH_CODE, SIG_OBJECT_HASH_CODE)
            .unwrap();

        c.bench_function("jni_call_object_hash_method_unchecked", |b| {
            b.iter(|| jni_int_call_unchecked(env, &obj, method_id))
        });
        Ok(())
    })
    .unwrap();
}

fn jni_new_object_str(c: &mut Criterion) {
    VM.attach_current_thread_for_scope(|env| -> jni::errors::Result<()> {
        let class = CLASS_OBJECT;

        c.bench_function("jni_new_object_str", |b| {
            b.iter(|| {
                let obj = env.new_object(class, SIG_OBJECT_CTOR, &[]).unwrap();
                env.delete_local_ref(obj);
            })
        });
        Ok(())
    })
    .unwrap();
}

fn jni_new_object_by_id_str(c: &mut Criterion) {
    VM.attach_current_thread_for_scope(|env| -> jni::errors::Result<()> {
        let class = CLASS_OBJECT;
        let ctor_id = env
            .get_method_id(class, METHOD_CTOR, SIG_OBJECT_CTOR)
            .unwrap();

        c.bench_function("jni_new_object_by_id_str", |b| {
            b.iter(|| {
                let obj = unsafe { env.new_object_unchecked(class, ctor_id, &[]) }.unwrap();
                env.delete_local_ref(obj);
            })
        });
        Ok(())
    })
    .unwrap();
}

fn jni_new_object_jclass(c: &mut Criterion) {
    VM.attach_current_thread_for_scope(|env| -> jni::errors::Result<()> {
        let class = Desc::<JClass>::lookup(CLASS_OBJECT, env).unwrap();

        c.bench_function("jni_new_object_jclass", |b| {
            b.iter(|| {
                let obj = env.new_object(&class, SIG_OBJECT_CTOR, &[]).unwrap();
                env.delete_local_ref(obj);
            })
        });
        Ok(())
    })
    .unwrap();
}

fn jni_new_object_by_id_jclass(c: &mut Criterion) {
    VM.attach_current_thread_for_scope(|env| -> jni::errors::Result<()> {
        let class = Desc::<JClass>::lookup(CLASS_OBJECT, env).unwrap();
        let ctor_id = env
            .get_method_id(&class, METHOD_CTOR, SIG_OBJECT_CTOR)
            .unwrap();

        c.bench_function("jni_new_object_by_id_jclass", |b| {
            b.iter(|| {
                let obj = unsafe { env.new_object_unchecked(&class, ctor_id, &[]) }.unwrap();
                env.delete_local_ref(obj);
            })
        });
        Ok(())
    })
    .unwrap();
}

fn jni_new_global_ref(c: &mut Criterion) {
    VM.attach_current_thread_for_scope(|env| -> jni::errors::Result<()> {
        let class = CLASS_OBJECT;
        let obj = env.new_object(class, SIG_OBJECT_CTOR, &[]).unwrap();
        let global_ref = env.new_global_ref(&obj).unwrap();
        env.delete_local_ref(obj);

        c.bench_function("jni_new_global_ref", |b| {
            b.iter(|| env.new_global_ref(&global_ref).unwrap())
        });
        Ok(())
    })
    .unwrap();
}

/// Checks the overhead of checking if exception has occurred.
///
/// Such checks are required each time a Java method is called, but
/// can be omitted if we call a JNI method that returns an error status.
///
/// See also #58
fn jni_check_exception(c: &mut Criterion) {
    VM.attach_current_thread_for_scope(|env| -> jni::errors::Result<()> {
        c.bench_function("jni_check_exception", |b| b.iter(|| env.exception_check()));
        Ok(())
    })
    .unwrap();
}

fn jni_get_java_vm(c: &mut Criterion) {
    VM.attach_current_thread_for_scope(|env| -> jni::errors::Result<()> {
        c.bench_function("jni_get_java_vm", |b| {
            b.iter(|| {
                let _jvm = env.get_java_vm();
            })
        });
        Ok(())
    })
    .unwrap();
}

fn jni_get_string(c: &mut Criterion) {
    VM.attach_current_thread_for_scope(|env| -> jni::errors::Result<()> {
        let string = env.new_string(TEST_STRING_UNICODE).unwrap();

        c.bench_function("jni_get_string", |b| {
            b.iter(|| {
                let s: String = env.get_string(&string).unwrap().into();
                assert_eq!(s, TEST_STRING_UNICODE);
            })
        });
        Ok(())
    })
    .unwrap();
}

fn jni_get_string_unchecked(c: &mut Criterion) {
    VM.attach_current_thread_for_scope(|env| -> jni::errors::Result<()> {
        let string = env.new_string(TEST_STRING_UNICODE).unwrap();

        c.bench_function("jni_get_string_unchecked", |b| {
            b.iter(|| {
                let s: String = unsafe { env.get_string_unchecked(&string) }.unwrap().into();
                assert_eq!(s, TEST_STRING_UNICODE);
            })
        });
        Ok(())
    })
    .unwrap();
}

/// A benchmark measuring Push/PopLocalFrame overhead.
///
/// Such operations are *required* if one attaches a long-running
/// native thread to the JVM because there is no 'return-from-native-method'
/// event when created local references are freed, hence no way for
/// the JVM to know that the local references are no longer used in the native code.
fn jni_noop_with_local_frame(c: &mut Criterion) {
    VM.attach_current_thread_for_scope(|env| -> jni::errors::Result<()> {
        const LOCAL_FRAME_SIZE: usize = 32;
        c.bench_function("jni_noop_with_local_frame", |b| {
            b.iter(|| {
                env.with_local_frame(LOCAL_FRAME_SIZE, |_| -> Result<_, jni::errors::Error> {
                    Ok(())
                })
                .unwrap()
            })
        });
        Ok(())
    })
    .unwrap();
}

/// A benchmark measuring Push/PopLocalFrame overhead while retuning a local reference
fn jni_with_local_frame_returning_local(c: &mut Criterion) {
    VM.attach_current_thread_for_scope(|env| -> jni::errors::Result<()> {
        const LOCAL_FRAME_SIZE: usize = 32;

        let class = env.find_class(CLASS_OBJECT).unwrap();
        c.bench_function("jni_with_local_frame_returning_local", |b| {
            b.iter(|| {
                env.with_local_frame_returning_local::<_, JObject, _>(LOCAL_FRAME_SIZE, |env| {
                    env.new_object(&class, SIG_OBJECT_CTOR, &[])
                })
            })
        });
        Ok(())
    })
    .unwrap();
}

/// A benchmark measuring Push/PopLocalFrame overhead while retuning a global
/// object reference that then gets converted into a local reference before
/// dropping the global
fn jni_with_local_frame_returning_global_to_local(c: &mut Criterion) {
    VM.attach_current_thread_for_scope(|env| -> jni::errors::Result<()> {
        const LOCAL_FRAME_SIZE: usize = 32;

        let class = env.find_class(CLASS_OBJECT).unwrap();
        c.bench_function("jni_with_local_frame_returning_global_to_local", |b| {
            b.iter(|| {
                let global = env
                    .with_local_frame::<_, GlobalRef<JObject<'static>>, jni::errors::Error>(
                        LOCAL_FRAME_SIZE,
                        |env| {
                            let local = env.new_object(&class, SIG_OBJECT_CTOR, &[])?;
                            let global = env.new_global_ref(local)?;
                            Ok(global)
                        },
                    )
                    .unwrap();
                let _local = env.new_local_ref(global).unwrap();
            })
        });
        Ok(())
    })
    .unwrap();
}

/// A benchmark of the overhead of repeatedly allocating/freeing a string with JNI
///
/// This provides a baseline to compare with the cost of making repeat attachments or checking for
/// existing attachments.
fn jni_new_string_within_single_thread_attachment(c: &mut Criterion) {
    VM.attach_current_thread_for_scope(|env| -> jni::errors::Result<()> {
        c.bench_function("jni_new_string_within_single_thread_attachment", |b| {
            b.iter(|| {
                black_box(env.new_string("Test").unwrap().auto());
            })
        });
        Ok(())
    })
    .unwrap();
}

/// A benchmark of the overhead of repeatedly attaching and detaching a native thread, with a scope.
fn jni_new_string_with_repeat_scoped_thread_attachments(c: &mut Criterion) {
    c.bench_function(
        "jni_new_string_with_repeat_scoped_thread_attachments",
        |b| {
            b.iter(|| {
                VM.attach_current_thread_for_scope(|env| -> jni::errors::Result<()> {
                    black_box(env.new_string("Test").unwrap().auto());
                    Ok(())
                })
                .unwrap();
            })
        },
    );
}

/// A benchmark of the overhead of repeatedly attaching and detaching a native thread.
///
/// The request to attach the thread permanently should mean the cost of attaching the thread is
/// only seen once before we start profiling.
fn jni_new_string_with_repeat_permanent_thread_attachments(c: &mut Criterion) {
    // Create a permanent attachment before we start the benchmark
    VM.attach_current_thread(|_env| -> jni::errors::Result<()> { Ok(()) })
        .unwrap();

    c.bench_function(
        "jni_new_string_with_repeat_permanent_thread_attachments",
        |b| {
            b.iter(|| {
                VM.attach_current_thread(|env| -> jni::errors::Result<()> {
                    black_box(env.new_string("Test").unwrap().auto());
                    Ok(())
                })
                .unwrap();
            })
        },
    );

    VM.detach_current_thread().unwrap();
}

fn native_arc(c: &mut Criterion) {
    VM.attach_current_thread_for_scope(|env| -> jni::errors::Result<()> {
        let class = CLASS_OBJECT;
        let obj = env.new_object(class, SIG_OBJECT_CTOR, &[]).unwrap();
        let global_ref = env.new_global_ref(&obj).unwrap();
        env.delete_local_ref(obj);
        let arc = Arc::new(global_ref);

        c.bench_function("native_arc", |b| {
            b.iter(|| {
                let _ = black_box(Arc::clone(&arc));
            })
        });
        Ok(())
    })
    .unwrap();
}

fn native_rc(c: &mut Criterion) {
    VM.attach_current_thread_for_scope(|env| -> jni::errors::Result<()> {
        let rc = Rc::new(env);

        c.bench_function("native_rc", |b| {
            b.iter(|| {
                let _ = black_box(Rc::clone(&rc));
            })
        });
        Ok(())
    })
    .unwrap();
}

criterion_group!(
    benches,
    native_call_function,
    jni_call_static_abs_method_safe,
    jni_call_static_abs_method_unchecked_str,
    jni_call_static_abs_method_unchecked_jclass,
    jni_call_static_date_time_method_safe,
    jni_call_static_date_time_method_unchecked_jclass,
    jni_call_object_hash_method_safe,
    jni_call_object_hash_method_unchecked,
    jni_new_object_str,
    jni_new_object_by_id_str,
    jni_new_object_jclass,
    jni_new_object_by_id_jclass,
    jni_new_global_ref,
    jni_check_exception,
    jni_get_java_vm,
    jni_get_string,
    jni_get_string_unchecked,
    jni_noop_with_local_frame,
    jni_with_local_frame_returning_local,
    jni_with_local_frame_returning_global_to_local,
    jni_new_string_within_single_thread_attachment,
    jni_new_string_with_repeat_scoped_thread_attachments,
    jni_new_string_with_repeat_permanent_thread_attachments,
    native_arc,
    native_rc
);

criterion_main!(benches);
