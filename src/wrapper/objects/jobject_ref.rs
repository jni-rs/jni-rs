use std::any::{Any, TypeId};

use dashmap::mapref::one::MappedRef;
use jni_sys::jobject;

use crate::{
    errors::Error,
    objects::{GlobalRef, JClass, JClassLoader, JObject},
    JavaVM,
};

#[cfg(doc)]
use crate::objects::{AutoLocal, GlobalRef, JString};

/// Identifies whether this is a bootstrap class or an application class.
pub enum ClassKind {
    /// Can use `FindClass` to locate the class.
    Bootstrap,

    /// Must use `ClassLoader::loadClass` to locate the class.
    Application,
}

pub type ClassRef<'a> =
    MappedRef<'a, TypeId, Box<dyn Any + Send + Sync>, GlobalRef<JClass<'static>>>;

/// A trait for types that represents a JNI reference (could be local, global or
/// weak global as well as wrapper types like [`AutoLocal`] and [`GlobalRef`])
///
///
/// This makes it possible for APIs like [`JNIEnv::new_global_ref`] to be given
/// a non-static local reference type like [`JString<'local>`] (or an
/// [`AutoLocal`] wrapper) and return a [`GlobalRef`] that is instead
/// parameterized by [`JString<'static>`].
pub trait JObjectRef: Sized {
    /// The fully qualified class name of the Java class represented by this
    /// reference.
    ///
    /// The format depends on the associated `LookupMethod`:
    ///
    /// - If the `LookupMethod` is `FindClass`, the class name is expected to be
    ///   a slash separated name (e.g. `"com/example/MyClass"`).
    ///
    /// - If the `LookupMethod` is `LoadClass`, the class name is expected to be
    ///   a dot-separated name (e.g. `"com.example.MyClass"`).
    const CLASS_NAME: &'static str;

    /// Determines whether the class can be found using `FindClass` or whether it
    /// requires `ClassLoader::loadClass`.
    const CLASS_KIND: ClassKind = ClassKind::Bootstrap;

    /// The generic associated [`Self::Kind`] type corresponds to the underlying
    /// class type (such as [`JObject`] or [`JString`]), parameterized by the
    /// lifetime that indicates whether the type holds a global reference
    /// (`'static`) or a local reference that's tied to a JNI stack frame.
    type Kind<'local>: JObjectRef + Default + Into<JObject<'local>> + AsRef<JObject<'local>>;
    // XXX: the compiler blows up if we try and specify a Send + Sync bound
    // here: "overflow evaluating the requirement..."
    //where
    //    Self::Kind<'static>: Send + Sync;
    //
    // As a workaround, we have a separate associated type

    /// The associated `GlobalKind` type should be equivalent to
    /// `Kind<'static>`, with the additional bound that ensures the type is
    /// `Send + Sync`
    type GlobalKind: JObjectRef
        + Default
        + Into<JObject<'static>>
        + AsRef<JObject<'static>>
        + Send
        + Sync;

    /// Returns the underlying, raw [`crate::sys::jobject`] reference.
    fn as_raw(&self) -> jobject;

    /// Returns `true` if this is a `null` object reference
    fn is_null(&self) -> bool {
        self.as_raw().is_null()
    }

    /// Returns `null` reference based on [`Self::Kind`]
    fn null<'any>() -> Self::Kind<'any> {
        Self::Kind::default()
    }

    /// Borrows a global reference to the class implemented by this reference.
    ///
    /// This is used as part of downcasting checks to do a cached lookup of associated class
    /// references - avoiding the cost of repeated FindClass or loadClass calls.
    ///
    /// The implementation is expected to use [`JavaVM::get_cached_or_insert_with`] to lookup cached
    /// API state, including a `GlobalRef<JClass>`.
    ///
    /// In case no class reference is already cached then use `loader_source.lookup_class()` to
    /// lookup a class reference.
    ///
    fn lookup_class<'vm>(vm: &'vm JavaVM, loader_source: LoaderSource) -> Option<ClassRef<'vm>>;

    /// Returns a new reference type based on [`Self::Kind`] for the given `local_ref` that is
    /// tied to the JNI stack frame for the given lifetime.
    ///
    /// # Safety
    ///
    /// The given lifetime must associated with an AttachGuard or a JNIEnv and represent a
    /// JNI stack frame.
    ///
    /// There must not be no other wrapper for the given `local_ref` reference (unless it is
    /// `null`)
    ///
    /// You are responsible to knowing that `Self::Kind` is a suitable wrapper type for the
    /// given `local_ref` reference. E.g. because the `local_ref` came from an `into_raw`
    /// call from the same type.
    ///
    unsafe fn from_local_raw<'env>(local_ref: jobject) -> Self::Kind<'env>;

    /// Returns a (`'static`) reference type based on [`Self::GlobalKind`] for the given `global_ref`.
    ///
    /// # Safety
    ///
    /// There must not be no other wrapper for the given `global_ref` reference (unless it is
    /// `null`)
    ///
    /// You are responsible to knowing that `Self::GlobalKind` is a suitable wrapper type for the
    /// given `global_ref` reference. E.g. because the `global_ref` came from an `into_raw`
    /// call from the same type.
    ///
    unsafe fn from_global_raw(global_ref: jobject) -> Self::GlobalKind;
}

/// Represents the source of a class loader to be used when looking up a class.
pub enum LoaderSource<'any_local, 'a> {
    /// `FindClass` can be used instead of using `ClassLoader::loadClass`
    ///
    /// Ideally this should only be used to lookup bootstrap classes, but maybe used as a fallback
    /// for application classes when there's no better place to find a suitable loader.
    ///
    /// `FindClass` will typically not find application classes when used from native threads that
    /// have been attached to the JVM where there is no native method call on the stack that
    /// can be used to identify a class loader.
    FindClass,
    /// A direct reference to the class loader that should be used
    Loader(&'a JClassLoader<'any_local>),
    /// In case we don't have a direct reference, to a `ClassLoader`, this reference can be used to
    /// try and find a suitable loader via `candidate.get_class()` followed by
    /// `class.get_class_loader()`.
    ///
    /// This is used when downcasting, where we can speculate that the `candidate` object being
    /// downcast _should_ be associated with the correct `ClassLoader`.
    TryFrom(&'a JObject<'any_local>),
}

impl<'a, 'any_local> LoaderSource<'a, 'any_local> {
    /// Loads the class with the given `name` using this loader source.
    ///
    /// Returns the loaded class, or a [`Error::NullPtr`] error if the class could not be found.
    ///
    /// Note: The implementation will only use `FindClass` for `Bootstrap` loader source.
    pub fn load_class<'env_local>(
        &self,
        name: &str,
        env: &mut crate::env::JNIEnv<'env_local>,
    ) -> crate::errors::Result<JClass<'env_local>> {
        fn load_class_with_catch<'any_loader, 'any_local>(
            loader: &JClassLoader<'any_loader>,
            name: &str,
            env: &mut crate::env::JNIEnv<'any_local>,
        ) -> crate::errors::Result<JClass<'any_local>> {
            match loader.load_class(name, env) {
                Ok(cls) => Ok(cls),
                Err(Error::JavaException) => {
                    // We assume it's a ClassNotFoundException and clear it
                    env.exception_clear();
                    Err(Error::NullPtr(
                        "ClassLoader::loadClass ClassNotFoundException",
                    ))
                }
                Err(e) => Err(e),
            }
        }

        match self {
            LoaderSource::FindClass => env.find_class(name),
            LoaderSource::TryFrom(candidate) => env
                .with_local_frame_returning_local::<_, JClass, _>(5, |env| {
                    let candidate_class = env.get_object_class(candidate)?;
                    // Doesn't throw exception for missing loader
                    let loader = candidate_class.get_class_loader(env)?;
                    load_class_with_catch(&loader, name, env)
                }),
            LoaderSource::Loader(loader) => load_class_with_catch(loader, name, env),
        }
    }
}

impl<T> JObjectRef for &T
where
    T: JObjectRef,
{
    const CLASS_NAME: &'static str = T::CLASS_NAME;
    const CLASS_KIND: ClassKind = T::CLASS_KIND;

    type Kind<'local> = T::Kind<'local>;
    type GlobalKind = T::GlobalKind;

    fn as_raw(&self) -> jobject {
        (*self).as_raw()
    }

    fn lookup_class<'vm>(vm: &'vm JavaVM, loader_source: LoaderSource) -> Option<ClassRef<'vm>> {
        T::lookup_class(vm, loader_source)
    }

    unsafe fn from_local_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        T::from_local_raw(local_ref)
    }

    unsafe fn from_global_raw(global_ref: jobject) -> Self::GlobalKind {
        T::from_global_raw(global_ref)
    }
}
