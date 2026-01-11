//! The Reference trait can technically be implemented for non-transparent reference type wrappers
//! (only the associated ::Kind and ::GlobalKind types are required to be transparent)
//!
//! The bind_java_type! macro has the additional restriction that any is_instance_of types refer to
//! Reference types that are FFI safe / transparent wrappers around JNI references. (I.e. T ==
//! T::Kind).
//!
//! This constraint allows it to generate AsRef impls that safely transmute from the .as_raw()
//! jobject reference to the is_instance_of type.

use jni::objects::JObject;
use jni::refs::Reference;

// To test that the macro rejects non-transparent reference types, we manually
// define a MyType Reference type and then a NonTransparentMyType that
// represents the same Java type but is not a transparent wrapper.
//
// (We need the transparent MyType type because Reference::Kind must still be a
// transparent wrapper around JObject even if the Reference type itself is not.)

#[derive(Default)]
#[repr(transparent)]
struct MyType<'local>(JObject<'local>);
impl<'local> AsRef<JObject<'local>> for MyType<'local> {
    fn as_ref(&self) -> &JObject<'local> {
        &self.0
    }
}
impl<'local> From<MyType<'local>> for JObject<'local> {
    fn from(value: MyType<'local>) -> Self {
        value.0
    }
}
unsafe impl Reference for MyType<'_> {
    // Note: The Kind and GlobalKind are always _required_ to be transparent wrappers around JObject
    // even if the Reference type itself is not.
    type Kind<'local> = MyType<'local>;
    type GlobalKind = MyType<'static>;

    fn as_raw(&self) -> jni::sys::jobject {
        self.0.as_raw()
    }

    fn class_name() -> std::borrow::Cow<'static, jni::strings::JNIStr> {
        std::borrow::Cow::Borrowed(jni::jni_str!("com.example.MyType"))
    }

    fn lookup_class<'caller>(
        env: &jni::Env<'_>,
        loader_context: &jni::refs::LoaderContext,
    ) -> jni::errors::Result<
        impl std::ops::Deref<Target = jni::refs::Global<jni::objects::JClass<'static>>> + 'caller,
    > {
        static CLASS: std::sync::OnceLock<jni::objects::Global<jni::objects::JClass>> =
            std::sync::OnceLock::new();

        let class = if let Some(class) = CLASS.get() {
            class
        } else {
            env.with_local_frame(4, |env| -> jni::errors::Result<_> {
                let class: jni::objects::JClass =
                    loader_context.load_class_for_type::<Self>(env, false)?;
                let global_class = env.new_global_ref(&class)?;
                let _ = CLASS.set(global_class);
                Ok(CLASS.get().unwrap())
            })?
        };

        Ok(class)
    }
}

#[derive(Default)]
struct NonTransparentMyType<'local>(JObject<'local>, i32);
impl<'local> AsRef<JObject<'local>> for NonTransparentMyType<'local> {
    fn as_ref(&self) -> &JObject<'local> {
        &self.0
    }
}
impl<'local> From<NonTransparentMyType<'local>> for JObject<'local> {
    fn from(value: NonTransparentMyType<'local>) -> Self {
        value.0
    }
}
unsafe impl Reference for NonTransparentMyType<'_> {
    // Note: The Kind and GlobalKind are always _required_ to be transparent wrappers around JObject
    // even if the Reference type itself is not.
    type Kind<'local> = MyType<'local>;
    type GlobalKind = MyType<'static>;

    fn as_raw(&self) -> jni::sys::jobject {
        self.0.as_raw()
    }

    fn class_name() -> std::borrow::Cow<'static, jni::strings::JNIStr> {
        MyType::class_name()
    }

    fn lookup_class<'caller>(
        env: &jni::Env<'_>,
        loader_context: &jni::refs::LoaderContext,
    ) -> jni::errors::Result<
        impl std::ops::Deref<Target = jni::refs::Global<jni::objects::JClass<'static>>> + 'caller,
    > {
        MyType::lookup_class(env, loader_context)
    }
}

jni::bind_java_type! {
    rust_type = JTest,
    java_type = "com.example.Test",

    type_map = {
        NonTransparentMyType => "com.example.MyType",
        //MyType => "com.example.MyType",
    },

    is_instance_of = {
        // This should cause a compile-time error because NonTransparentMyType is not FFI
        non_transparent = NonTransparentMyType,
        //transparent = MyType,
    },
}

fn main() {}
