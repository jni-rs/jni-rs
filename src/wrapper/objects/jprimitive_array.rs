use std::marker::PhantomData;
use std::ops::Deref;

use once_cell::sync::OnceCell;

use crate::{
    errors::Result,
    objects::{GlobalRef, JClass, JObject, JObjectRef, LoaderContext},
    sys::{jarray, jobject},
    JavaVM,
};

use super::TypeArray;

#[cfg(doc)]
use crate::JNIEnv;

/// Lifetime'd representation of a [`jarray`] which wraps a [`JObject`] reference
///
/// This is a wrapper type for a [`JObject`] local reference that's used to
/// differentiate JVM array types.
#[repr(transparent)]
#[derive(Debug)]
pub struct JPrimitiveArray<'local, T: TypeArray> {
    obj: JObject<'local>,
    lifetime: PhantomData<(&'local (), T)>,
}

impl<'local, T: TypeArray> AsRef<JPrimitiveArray<'local, T>> for JPrimitiveArray<'local, T> {
    fn as_ref(&self) -> &JPrimitiveArray<'local, T> {
        self
    }
}

impl<'local, T: TypeArray> AsMut<JPrimitiveArray<'local, T>> for JPrimitiveArray<'local, T> {
    fn as_mut(&mut self) -> &mut JPrimitiveArray<'local, T> {
        self
    }
}

impl<'local, T: TypeArray> AsRef<JObject<'local>> for JPrimitiveArray<'local, T> {
    fn as_ref(&self) -> &JObject<'local> {
        &self.obj
    }
}

impl<'local, T: TypeArray> ::std::ops::Deref for JPrimitiveArray<'local, T> {
    type Target = JObject<'local>;

    fn deref(&self) -> &Self::Target {
        &self.obj
    }
}

impl<'local, T: TypeArray> From<JPrimitiveArray<'local, T>> for JObject<'local> {
    fn from(other: JPrimitiveArray<'local, T>) -> JObject<'local> {
        other.obj
    }
}

impl<T: TypeArray> std::default::Default for JPrimitiveArray<'_, T> {
    fn default() -> Self {
        Self {
            obj: JObject::null(),
            lifetime: PhantomData,
        }
    }
}

impl<T: TypeArray> JPrimitiveArray<'_, T> {
    /// Creates a [`JPrimitiveArray`] that wraps the given `raw` [`jarray`]
    ///
    /// # Safety
    ///
    /// `raw` may be a null pointer. If `raw` is not a null pointer, then:
    ///
    /// * `raw` must be a valid raw JNI local reference.
    /// * There must not be any other `JObject` representing the same local reference.
    /// * The lifetime `'local` must not outlive the local reference frame that the local reference
    ///   was created in.
    pub const unsafe fn from_raw(raw: jarray) -> Self {
        Self {
            obj: JObject::from_raw(raw as jobject),
            lifetime: PhantomData,
        }
    }

    /// Unwrap to the raw jni type.
    pub const fn into_raw(self) -> jarray {
        self.obj.into_raw() as jarray
    }
}

/// Lifetime'd representation of a [`crate::sys::jbooleanArray`] which wraps a [`JObject`] reference
pub type JBooleanArray<'local> = JPrimitiveArray<'local, crate::sys::jboolean>;

/// Lifetime'd representation of a [`crate::sys::jbyteArray`] which wraps a [`JObject`] reference
pub type JByteArray<'local> = JPrimitiveArray<'local, crate::sys::jbyte>;

/// Lifetime'd representation of a [`crate::sys::jcharArray`] which wraps a [`JObject`] reference
pub type JCharArray<'local> = JPrimitiveArray<'local, crate::sys::jchar>;

/// Lifetime'd representation of a [`crate::sys::jshortArray`] which wraps a [`JObject`] reference
pub type JShortArray<'local> = JPrimitiveArray<'local, crate::sys::jshort>;

/// Lifetime'd representation of a [`crate::sys::jintArray`] which wraps a [`JObject`] reference
pub type JIntArray<'local> = JPrimitiveArray<'local, crate::sys::jint>;

/// Lifetime'd representation of a [`crate::sys::jlongArray`] which wraps a [`JObject`] reference
pub type JLongArray<'local> = JPrimitiveArray<'local, crate::sys::jlong>;

/// Lifetime'd representation of a [`crate::sys::jfloatArray`] which wraps a [`JObject`] reference
pub type JFloatArray<'local> = JPrimitiveArray<'local, crate::sys::jfloat>;

/// Lifetime'd representation of a [`crate::sys::jdoubleArray`] which wraps a [`JObject`] reference
pub type JDoubleArray<'local> = JPrimitiveArray<'local, crate::sys::jdouble>;

/// Trait to access the raw `jarray` pointer for types that wrap an array reference
///
/// # Safety
///
/// Implementing this trait will allow a type to be passed to [`JNIEnv::get_array_length()`]
/// or other JNI APIs that only work with a valid reference to an array (or `null`)
///
pub unsafe trait AsJArrayRaw<'local>: AsRef<JObject<'local>> {
    /// Returns the raw JNI pointer as a `jarray`
    fn as_jarray_raw(&self) -> jarray {
        self.as_ref().as_raw() as jarray
    }
}

unsafe impl<'local, T: TypeArray> AsJArrayRaw<'local> for JPrimitiveArray<'local, T> {}

use paste::paste;

macro_rules! impl_ref_for_jprimitive_array {
    ($type:ident, $class_name:expr) => {
        paste! {
            #[allow(non_camel_case_types)]
            struct [<JPrimitiveArrayAPI _ $type>] {
                class: GlobalRef<JClass<'static>>,
            }

            impl [<JPrimitiveArrayAPI _ $type>] {
                fn get<'any_local>(
                    vm: &JavaVM,
                    loader_context: &LoaderContext<'any_local, '_>,
                ) -> Result<&'static Self> {
                    static JPRIMITIVE_ARRAY_API: OnceCell<[<JPrimitiveArrayAPI _ $type>]> = OnceCell::new();
                    JPRIMITIVE_ARRAY_API.get_or_try_init(|| {
                        vm.with_env_current_frame(|env| {
                            let class =
                                loader_context.load_class_for_type::<JPrimitiveArray::<crate::sys::$type>>(false, env)?;
                            let class = env.new_global_ref(&class).unwrap();
                            Ok(Self {
                                class,
                            })
                        })
                    })
                }
            }

            // SAFETY: JPrimitiveArray is a transparent JObject wrapper with no Drop side effects
            unsafe impl JObjectRef for JPrimitiveArray<'_, crate::sys::$type> {
                const CLASS_NAME: &'static str = $class_name;

                type Kind<'env> = JPrimitiveArray<'env, crate::sys::$type>;
                type GlobalKind = JPrimitiveArray<'static, crate::sys::$type>;

                fn as_raw(&self) -> jobject {
                    self.obj.as_raw()
                }

                fn lookup_class<'vm>(
                    vm: &'vm JavaVM,
                    loader_context: LoaderContext,
                ) -> crate::errors::Result<impl Deref<Target = GlobalRef<JClass<'static>>> + 'vm> {
                    let api = [<JPrimitiveArrayAPI _ $type>]::get(vm, &loader_context)?;
                    Ok(&api.class)
                }

                unsafe fn from_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
                    JPrimitiveArray::from_raw(local_ref)
                }

                unsafe fn from_global_raw(global_ref: jobject) -> Self::GlobalKind {
                    JPrimitiveArray::from_raw(global_ref)
                }
            }
        }
    };
}

impl_ref_for_jprimitive_array!(jboolean, "[Z");
impl_ref_for_jprimitive_array!(jbyte, "[B");
impl_ref_for_jprimitive_array!(jchar, "[C");
impl_ref_for_jprimitive_array!(jshort, "[S");
impl_ref_for_jprimitive_array!(jint, "[I");
impl_ref_for_jprimitive_array!(jlong, "[J");
impl_ref_for_jprimitive_array!(jfloat, "[F");
impl_ref_for_jprimitive_array!(jdouble, "[D");
