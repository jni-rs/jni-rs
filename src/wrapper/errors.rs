#![allow(missing_docs)]

use thiserror::Error;

use crate::sys;
use crate::wrapper::signature::TypeSignature;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Invalid JValue type cast: {0}. Actual type: {1}")]
    WrongJValueType(&'static str, &'static str),
    #[error("Invalid constructor return type (must be void)")]
    InvalidCtorReturn,
    #[error("Invalid number of arguments passed to java method: {0}")]
    InvalidArgList(TypeSignature),
    #[error("Method not found: {name} {sig}")]
    MethodNotFound { name: String, sig: String },
    #[error("Field not found: {name} {sig}")]
    FieldNotFound { name: String, sig: String },
    #[error("Java exception was thrown")]
    JavaException,
    #[error("JNIEnv null method pointer for {0}")]
    JNIEnvMethodNotFound(&'static str),
    #[error("Null pointer in {0}")]
    NullPtr(&'static str),
    #[error("Null pointer deref in {0}")]
    NullDeref(&'static str),
    #[error("Mutex already locked")]
    TryLock,
    #[error("JavaVM null method pointer for {0}")]
    JavaVMMethodNotFound(&'static str),
    #[error("Unknown error")]
    Unknown,
    #[error("Current thread is not attached to the Java VM")]
    ThreadDetached,
    #[error("Problem due to JNI version mismatch")]
    WrongVersion,
    #[error("Not enough memory available")]
    NoMemory,
    #[error("Java VM already created")]
    AlreadyCreated,
    #[error("Invalid arguments provided")]
    InvalidArguments,
    #[error("Field already set: {0}")]
    FieldAlreadySet(String),
    #[error("Throw failed with error code {0}")]
    ThrowFailed(i32),
    #[error("Parse failed for input: {1}")]
    ParseFailed(#[source] combine::error::StringStreamError, String),
    #[error("JNI error: {0}")]
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
        sys::JNI_ERR => Err(Error::Unknown),
        sys::JNI_EDETACHED => Err(Error::ThreadDetached),
        sys::JNI_EVERSION => Err(Error::WrongVersion),
        sys::JNI_ENOMEM => Err(Error::NoMemory),
        sys::JNI_EEXIST => Err(Error::AlreadyCreated),
        sys::JNI_EINVAL => Err(Error::InvalidArguments),
        _ => Err(Error::Other(code)),
    }
}

pub struct Exception {
    pub class: String,
    pub msg: String,
}

pub trait ToException {
    fn to_exception(&self) -> Exception;
}
