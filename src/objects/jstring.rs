use std::ops::Deref;

use once_cell::sync::OnceCell;
use thiserror::Error;

use crate::{
    errors::Result,
    objects::{Global, JClass, JMethodID, JObject, LoaderContext},
    strings::{JNIStr, MUTF8Chars},
    sys::{jobject, jstring},
    Env, JavaVM,
};

use super::JObjectRef;

#[cfg(doc)]
use crate::errors::Error;

/// Lifetime'd representation of a `jstring`. Just a `JObject` wrapped in a new
/// class.
#[repr(transparent)]
#[derive(Default)]
pub struct JString<'local>(JObject<'local>);

impl<'local> AsRef<JString<'local>> for JString<'local> {
    fn as_ref(&self) -> &JString<'local> {
        self
    }
}

impl<'local> AsRef<JObject<'local>> for JString<'local> {
    fn as_ref(&self) -> &JObject<'local> {
        self
    }
}

impl<'local> ::std::ops::Deref for JString<'local> {
    type Target = JObject<'local>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'local> From<JString<'local>> for JObject<'local> {
    fn from(other: JString) -> JObject {
        other.0
    }
}

#[allow(rustdoc::invalid_html_tags)]
/// Display the contents of a `JString`
///
/// This implementation relies on JNI (GetStringUTFChars) to retrieve the string contents for
/// display.
///
/// If you try and format a null reference this will output "<NULL>"
///
/// In case you attempt to format a JString before [`JavaVM::singleton`] has been initialized then
/// this will simply output "<JNI Not Initialized>" and log an error.
///
/// In case of any other unexpected JNI error, this will output "<JNI Error>" and log the error
/// details.
impl<'local> std::fmt::Display for JString<'local> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[derive(Error, Debug)]
        #[error(transparent)]
        enum FmtOrJniError {
            Fmt(#[from] std::fmt::Error),
            Jni(#[from] crate::errors::Error),
        }

        if self.is_null() {
            return write!(f, "<NULL>");
        }

        // The only way it's possible to have a `JString` while `JavaVM::singleton` is
        // not initialized would be if the `JString` was used to capture a native method
        // argument and `EnvUnowned::with_env` has not been called yet (and nothing
        // else has initialized this crate already)
        //
        // I.e. it's highly unlikely that this should return an error.
        JavaVM::singleton()
            .map_err(FmtOrJniError::Jni)
            .and_then(|vm| {
                // In the common case we expect this attachment will be a NOOP.
                //
                // In the (unlikely) case that a `Global<JString>` is being formatted from an
                // arbitrary thread that's not attached to the JVM then we create a scoped
                // attachment so we avoid the side effect of attaching the current thread
                // permanently.
                vm.attach_current_thread_for_scope(
                    |env| -> std::result::Result<(), FmtOrJniError> {
                        // Since we have already checked for a null reference it should be highly
                        // unlikely for there to be any JNI errors.
                        //
                        // Note: there won't be any local reference created as a side effect.
                        // Note: there's no risk of side effects from an exception being thrown.
                        // A `GetStringUTFChars` failure may result in a `NullPtr` error that
                        // is handled below as a general JNI error.
                        let mutf8_chars = self.mutf8_chars(env)?;
                        let s = mutf8_chars.to_str();
                        write!(f, "{}", s)?;
                        Ok(())
                    },
                )
            })
            .or_else(|err| {
                match err {
                    FmtOrJniError::Fmt(err) => Err(err),
                    FmtOrJniError::Jni(crate::errors::Error::UninitializedJavaVM) => {
                        log::error!(
                            "error getting JavaVM singleton to format JString: {:#?}",
                            err
                        );
                        write!(f, "<JNI Not Initialized>")
                    }
                    FmtOrJniError::Jni(err) => {
                        // If we failed to get the string contents, just print the error
                        log::error!("error getting JString contents: {:#?}", err);
                        write!(f, "<JNI Error>")
                    }
                }
            })
    }
}

struct JStringAPI {
    class: Global<JClass<'static>>,
    intern_method: JMethodID,
}

impl JStringAPI {
    fn get<'any_local>(
        env: &Env<'_>,
        loader_context: &LoaderContext<'any_local, '_>,
    ) -> Result<&'static Self> {
        static JSTRING_API: OnceCell<JStringAPI> = OnceCell::new();
        JSTRING_API.get_or_try_init(|| {
            let vm = env.get_java_vm();
            vm.with_env_current_frame(|env| {
                let class = loader_context.load_class_for_type::<JString>(true, env)?;
                let class = env.new_global_ref(&class).unwrap();
                let intern_method = env
                    .get_method_id(&class, c"intern", c"()Ljava/lang/String;")
                    .expect("JString.intern method not found");

                Ok(Self {
                    class,
                    intern_method,
                })
            })
        })
    }
}

impl JString<'_> {
    /// Creates a [`JString`] that wraps the given `raw` [`jstring`]
    ///
    /// # Safety
    ///
    /// `raw` may be a null pointer. If `raw` is not a null pointer, then:
    ///
    /// * `raw` must be a valid raw JNI local reference.
    /// * There must not be any other `JObject` representing the same local reference.
    /// * The lifetime `'local` must not outlive the local reference frame that the local reference
    ///   was created in.
    pub const unsafe fn from_raw(raw: jstring) -> Self {
        Self(JObject::from_raw(raw as jobject))
    }

    /// Unwrap to the raw jni type.
    pub const fn into_raw(self) -> jstring {
        self.0.into_raw() as jstring
    }

    /// Returns a canonical, interned version of this string.
    pub fn intern<'local>(&self, env: &mut Env<'local>) -> Result<JString<'local>> {
        let api = JStringAPI::get(env, &LoaderContext::None)?;

        // Safety: We know that `intern` is a valid method on `java/lang/String` that has no
        // arguments and it returns a valid `String` instance.
        let interned = unsafe {
            let interned = env
                .call_method_unchecked(
                    self,
                    api.intern_method,
                    crate::signature::ReturnType::Object,
                    &[],
                )?
                .l()?;
            JString::from_raw(interned.into_raw() as jstring)
        };
        Ok(interned)
    }

    /// Gets the contents of this string, in [modified UTF-8] encoding (via `GetStringUTFChars`).
    ///
    /// The returned [MUTF8Chars] guard can be used to access the modified UTF-8 bytes, or to
    /// convert to a Rust string (UTF-8).
    ///
    /// For example:
    ///
    /// ```rust,no_run
    /// # use jni::{errors::Result, Env, objects::*, strings::*};
    /// #
    /// # fn f(env: &mut Env) -> Result<()> {
    /// let my_jstring = env.new_string(c"Hello, world!")?;
    /// let mutf8_chars = my_jstring.mutf8_chars(env)?;
    /// let jni_str: &JNIStr = &mutf8_chars;
    /// let rust_str = jni_str.to_str();
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// When the [MUTF8Chars] guard is dropped, the reference to the contents gets released.
    ///
    /// The [MUTF8Chars] guard dereferences to a [JNIStr].
    ///
    /// Also note that [MUTF8Chars] (and also [`JString`] itself) implements `Display` and
    /// `ToString` so it's also possible to use `.to_string()` to get a Rust String from a [JString]
    ///
    /// [modified UTF-8]: https://en.wikipedia.org/wiki/UTF-8#Modified_UTF-8
    ///
    /// # Errors
    ///
    /// Returns an [Error::NullPtr] if this [`JString`] is null.
    pub fn mutf8_chars(&self, env: &Env<'_>) -> Result<MUTF8Chars<'_, &JString<'_>>> {
        MUTF8Chars::from_get_string_utf_chars(env, self)
    }

    /// Gets the contents of this string as a Rust `String`.
    ///
    /// For example:
    ///
    /// ```rust,no_run
    /// # use jni::{errors::Result, Env, objects::*};
    /// #
    /// # fn f(env: &mut Env) -> Result<()> {
    /// let jstring = env.new_string(c"Hello, world!")?;
    /// let rust_string = jstring.try_to_string(&env)?;
    /// assert_eq!(rust_string, "Hello, world!");
    /// # ; Ok(())
    /// # }
    /// ```
    ///
    /// This is equivalent to calling [`Self::mutf8_chars`] and then converting that to a `String`, like:
    ///
    /// ```rust,no_run
    /// # use jni::{errors::Result, Env, objects::*};
    /// #
    /// # fn f(env: &mut Env) -> Result<()> {
    /// let jstring = env.new_string(c"Hello, world!")?;
    /// let mutf8_chars = jstring.mutf8_chars(&env)?;
    /// let rust_string = mutf8_chars.to_string();
    /// # ; Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an [Error::NullPtr] if this [`JString`] is null.
    pub fn try_to_string(&self, env: &Env<'_>) -> Result<String> {
        let mutf8_chars = self.mutf8_chars(env)?;
        Ok(mutf8_chars.to_string())
    }
}

// SAFETY: JString is a transparent JObject wrapper with no Drop side effects
unsafe impl JObjectRef for JString<'_> {
    const CLASS_NAME: &'static JNIStr = JNIStr::from_cstr(c"java.lang.String");

    type Kind<'env> = JString<'env>;
    type GlobalKind = JString<'static>;

    fn as_raw(&self) -> jobject {
        self.0.as_raw()
    }

    fn lookup_class<'env>(
        env: &'env Env<'_>,
        loader_context: LoaderContext,
    ) -> crate::errors::Result<impl Deref<Target = Global<JClass<'static>>> + 'env> {
        let api = JStringAPI::get(env, &loader_context)?;
        Ok(&api.class)
    }

    unsafe fn from_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        JString::from_raw(local_ref)
    }

    unsafe fn from_global_raw(global_ref: jobject) -> Self::GlobalKind {
        JString::from_raw(global_ref)
    }
}
