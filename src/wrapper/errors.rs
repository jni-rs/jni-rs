#![allow(missing_docs)]

use std::char::{CharTryFromError, DecodeUtf16Error};

use thiserror::Error;

use crate::sys;
use crate::wrapper::signature::TypeSignature;

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(doc)]
use crate::objects::{char_from_java_int, char_to_java, char_to_java_int, JValue, JValueOwned};

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("JavaVM singleton uninitialized")]
    UninitializedJavaVM,
    #[error("Invalid JValue type cast: {0}. Actual type: {1}")]
    WrongJValueType(&'static str, &'static str),
    #[error("Invalid object type")]
    WrongObjectType,
    #[error("Invalid constructor return type (must be void)")]
    InvalidCtorReturn,
    #[error("Invalid number or type of arguments passed to java method: {0}")]
    InvalidArgList(TypeSignature),
    #[error("Object behind weak reference freed")]
    ObjectFreed,
    #[error("Class not found: {name:?}")]
    ClassNotFound { name: String },
    #[error("Method not found: {name} {sig}")]
    MethodNotFound { name: String, sig: String },
    #[error("Field not found: {name} {sig}")]
    FieldNotFound { name: String, sig: String },
    #[error("Java exception was thrown")]
    JavaException,
    #[error("Env null method pointer for {0}")]
    EnvMethodNotFound(&'static str),
    #[error("Null pointer in {0}")]
    NullPtr(&'static str),
    #[error("Mutex already locked")]
    TryLock,
    #[error("Field already set: {0}")]
    FieldAlreadySet(String),
    #[error("Throw failed with error code {0}")]
    ThrowFailed(i32),
    #[error("Parse failed for input: {0}")]
    ParseFailed(String),
    #[error("JNI call failed")]
    JniCall(#[source] JniError),

    /// [`JValue::c_char`] or [`JValueOwned::c_char`] was used, and although the value does indeed contain a Java `char`, it is part of a UTF-16 [surrogate pair] and cannot be converted to a Rust `char` by itself.
    ///
    /// [surrogate pair]: https://en.wikipedia.org/wiki/Surrogate_pair
    #[error("A Java `char` has the value 0x{char:x}; it is part of a UTF-16 surrogate pair and cannot be converted to a Rust `char` by itself", char = source.unpaired_surrogate())]
    InvalidUtf16 {
        /// The cause of this error. Use [`DecodeUtf16Error::unpaired_surrogate`] to get the Java `char` in question.
        #[source]
        source: DecodeUtf16Error,
    },

    /// [`JValue::i_char`] or [`JValueOwned::i_char`] was used, and although the value does indeed contain a Java `int`, it is not a valid UTF-32 unit.
    #[error("A Java `int` has the value 0x{char:x}, which is not a valid UTF-32 unit; cannot convert it to a Rust `char`")]
    InvalidUtf32 {
        /// The Java `int` that doesn't contain a valid UTF-32 unit.
        char: sys::jint,

        /// The cause of this error.
        #[source]
        source: CharTryFromError,
    },

    #[error("This Java virtual machine is too old; at least Java 1.4 is required")]
    UnsupportedVersion,

    #[error("The thread can't be detached while AttachGuards exist")]
    ThreadAttachmentGuarded,

    #[error("Panic caught in JNI code: {0}")]
    PanicCaught(String),
}

#[derive(Debug, Error)]
pub enum JniError {
    #[error("Unknown error")]
    Unknown,
    #[error("Current thread is not attached to the Java VM")]
    ThreadDetached,
    #[error("JNI version error")]
    WrongVersion,
    #[error("Not enough memory")]
    NoMemory,
    #[error("VM already created")]
    AlreadyCreated,
    #[error("Invalid arguments")]
    InvalidArguments,
    #[error("Error code {0}")]
    Other(sys::jint),
}

impl<T> From<::std::sync::TryLockError<T>> for Error {
    fn from(_: ::std::sync::TryLockError<T>) -> Self {
        Error::TryLock
    }
}

pub fn jni_error_code_to_result(code: sys::jint) -> Result<()> {
    match code {
        sys::JNI_OK => Ok(()),
        sys::JNI_ERR => Err(JniError::Unknown),
        sys::JNI_EDETACHED => Err(JniError::ThreadDetached),
        sys::JNI_EVERSION => Err(JniError::WrongVersion),
        sys::JNI_ENOMEM => Err(JniError::NoMemory),
        sys::JNI_EEXIST => Err(JniError::AlreadyCreated),
        sys::JNI_EINVAL => Err(JniError::InvalidArguments),
        _ => Err(JniError::Other(code)),
    }
    .map_err(Error::JniCall)
}

pub struct Exception {
    pub class: String,
    pub msg: String,
}

pub trait ToException {
    fn to_exception(&self) -> Exception;
}

/// An error that occurred while starting the JVM using the JNI Invocation API.
///
/// This only exists if the "invocation" feature is enabled.
#[cfg(feature = "invocation")]
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum StartJvmError {
    /// An attempt was made to find a JVM using [java-locator], but it failed.
    ///
    /// If this happens, give an explicit location to [`JavaVM::with_libjvm`] or set the
    /// `JAVA_HOME` environment variable.
    ///
    /// [java-locator]: https://docs.rs/java-locator/
    /// [`JavaVM::with_libjvm`]: crate::JavaVM::with_libjvm
    #[error("Couldn't automatically discover the Java VM's location (try setting the JAVA_HOME environment variable): {0}")]
    NotFound(
        #[from]
        #[source]
        java_locator::errors::JavaLocatorError,
    ),

    /// An error occurred in trying to load the JVM shared library.
    ///
    /// On Windows, if this happens it may be necessary to add your `$JAVA_HOME/bin` directory
    /// to the DLL search path by adding it to the `PATH` environment variable.
    #[error("Couldn't load the Java VM shared library ({0}): {1}")]
    LoadError(String, #[source] libloading::Error),

    /// The JNI function `JNI_CreateJavaVM` returned an error.
    #[error("{0}")]
    Create(
        #[from]
        #[source]
        Error,
    ),
}

#[cfg(feature = "invocation")]
pub type StartJvmResult<T> = std::result::Result<T, StartJvmError>;

/// Raised by `char_to_java` and the implementation of `TryFrom<char>` for [`JValueGen`] when a Rust [`char`] is not representable as a Java `char`.
///
/// See [`char_to_java`] for more information.
#[derive(Debug, Error)]
#[error("The code point U+{char_as_u32:X} {char:?} cannot be converted to a Java `char`, because it is not representable as a single UTF-16 unit.", char_as_u32 = u32::from(*char))]
pub struct CharToJavaError {
    /// The character that could not be converted.
    pub char: char,
}
