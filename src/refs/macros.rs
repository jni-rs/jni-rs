#[cfg(doc)]
use crate::{objects::JObject, refs::Reference};

/// Define a new reference type that wraps a `JObject` and implements `Reference`.
#[macro_export]
macro_rules! define_reference_type {
    ($type:ident, $class_name:expr, $api_init:expr) => {

        paste::paste!{
            impl [<$type API>] {
                fn get<'any_local>(
                    env: &$crate::Env<'_>,
                    loader_context: &$crate::refs::LoaderContext<'any_local, '_>,
                ) -> $crate::errors::Result<&'static Self> {
                    static API: once_cell::sync::OnceCell<[<$type API>]> = once_cell::sync::OnceCell::new();
                    API.get_or_try_init(|| {
                        env.with_local_frame($crate::DEFAULT_LOCAL_FRAME_CAPACITY, |env| {
                            ($api_init)(env, loader_context)
                        })
                    })
                }
            }
        }

        #[doc = concat!(r#"A `"#, $class_name, r#"` wrapper that is tied to a JNI local reference frame.

See the [`JObject`] documentation for more information about reference
wrappers, how to cast them, and local reference frame lifetimes.

[`JObject`]: $crate::objects::JObject
"#)]
        #[repr(transparent)]
        #[derive(Debug, Default)]
        pub struct $type<'local>($crate::objects::JObject<'local>);

        impl<'local> AsRef<$type<'local>> for $type<'local> {
            fn as_ref(&self) -> &$type<'local> {
                self
            }
        }

        impl<'local> AsRef<$crate::objects::JObject<'local>> for $type<'local> {
            fn as_ref(&self) -> &$crate::objects::JObject<'local> {
                self
            }
        }

        impl<'local> ::std::ops::Deref for $type<'local> {
            type Target = $crate::objects::JObject<'local>;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl<'local> From<$type<'local>> for $crate::objects::JObject<'local> {
            fn from(other: $type<'local>) -> $crate::objects::JObject<'local> {
                other.0
            }
        }

        impl<'local> $type<'local> {
            #[doc = concat!(r#"Creates a [`"#, stringify!($type), r#"`] that wraps the given `raw` [jobject]

# Safety

- `raw` must be a valid raw JNI local reference (or `null`).
- `raw` must be an instance of `"#, $class_name, r#"`.
- There must not be any other owning [Reference] wrapper for the same reference.
- The local reference must belong to the current thread and not outlive the
  JNI stack frame associated with the [Env] `'local` lifetime.

[jobject]: $crate::sys::jobject
[Reference]: $crate::refs::Reference
[Env]: $crate::Env
"#)]
            pub unsafe fn from_raw<'local_inner>(
                env: &$crate::Env<'local_inner>,
                raw: $crate::sys::jobject,
            ) -> $type<'local_inner> {
                $type($crate::objects::JObject::from_raw(env, raw))
            }

            #[doc = concat!(r#"Creates a new null reference.

Null references are always valid and do not belong to a local reference frame. Therefore,
the returned [`"#, stringify!($type), r#"`] always has the `'static` lifetime."#)]
            pub const fn null() -> $type<'static> {
                $type($crate::objects::JObject::null())
            }

            /// Unwrap to the raw jni type.
            pub const fn into_raw(self) -> $crate::sys::jobject {
                self.0.into_raw()
            }

            #[doc = concat!(r#"Cast a local reference to a [`"#, stringify!($type), r#"`]

This will do a runtime (`IsInstanceOf`) check that the object is an instance of `"#, $class_name, r#"`.

Also see these other options for casting local or global references to a [`"#, stringify!($type), r#"`]:
- [Env::as_cast]
- [Env::new_cast_local_ref]
- [Env::cast_local]
- [Env::new_cast_global_ref]
- [Env::cast_global]

# Errors

Returns [Error::WrongObjectType] if the `IsInstanceOf` check fails.

[Error::WrongObjectType]: $crate::errors::Error::WrongObjectType
"#)]
            pub fn cast_local<'any_local>(
                obj: impl $crate::refs::Reference + Into<$crate::objects::JObject<'any_local>> + AsRef<$crate::objects::JObject<'any_local>>,
                env: &mut $crate::Env<'_>,
            ) -> $crate::errors::Result<$type<'any_local>> {
                env.cast_local::<$type>(obj)
            }
        }

        paste::paste!{
            // SAFETY: this is a transparent JObject wrapper with no Drop side effects
            unsafe impl $crate::refs::Reference for $type<'_> {
                type Kind<'env> = $type<'env>;
                type GlobalKind = $type<'static>;

                fn as_raw(&self) -> $crate::sys::jobject {
                    self.0.as_raw()
                }

                fn class_name() -> Cow<'static, $crate::strings::JNIStr> {
                    const CLASS_NAME: &$crate::strings::JNIStr = $crate::strings::JNIStr::from_cstr(match std::ffi::CStr::from_bytes_with_nul(concat!($class_name, "\0").as_bytes()) { Ok(cstr) => cstr, Err(_) => panic!("Class name is not a valid C string") });
                    Cow::Borrowed(CLASS_NAME)
                }

                fn lookup_class<'caller>(
                    env: &Env<'_>,
                    loader_context: LoaderContext,
                ) -> $crate::errors::Result<impl Deref<Target = $crate::refs::Global<$crate::objects::JClass<'static>>> + 'caller> {
                    let api = [<$type API>]::get(env, &loader_context)?;
                    Ok(&api.class)
                }

                unsafe fn kind_from_raw<'env>(local_ref: $crate::sys::jobject) -> Self::Kind<'env> {
                    $type($crate::objects::JObject::kind_from_raw(local_ref))
                }

                unsafe fn global_kind_from_raw(global_ref: $crate::sys::jobject) -> Self::GlobalKind {
                    $type($crate::objects::JObject::global_kind_from_raw(global_ref))
                }
            }
        }
    };
}
