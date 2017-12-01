#![allow(unused_doc_comment)]
#![allow(missing_docs)]

error_chain!{
    foreign_links {
    }

    errors {
        WrongJValueType(cast: &'static str, actual: &'static str) {
            description("Invalid JValue type cast")
            display("Invalid JValue type cast: {}. Actual type: {}",
                    cast,
                    actual)
        }
        InvalidCtorReturn {
            description("Invalid constructor return type (must be void)")
            display("Invalid constructor return type (must be void)")
        }
        InvalidArgList {
            description("Invalid number of arguments passed to java method")
            display("Invalid number of arguments passed to java method")
        }
        MethodNotFound(name: String, sig: String) {
            description("Method not found")
            display("Method not found: {} {}", name, sig)
        }
        FieldNotFound(name: String, ty: String) {
            description("Field not found")
            display("Field not found: {} {}", name, ty)
        }
        JavaException {
            description("Java exception was thrown")
            display("Java exception was thrown")
        }
        JNIEnvMethodNotFound(name: &'static str) {
            description("Method pointer null in JNIEnv")
            display("JNIEnv null method pointer for {}", name)
        }
        NullPtr(context: &'static str) {
            description("null pointer")
            display("null pointer in {}", context)
        }
        NullDeref(context: &'static str) {
            description("null pointer deref")
            display("null pointer deref in {}", context)
        }
        TryLock {
            description("mutex already locked")
            display("mutex already locked")
        }
        Detached {
            description("Current thread is not attached to the java VM")
            display("Current thread is not attached to the java VM")
        }
    }
}

impl<T> From<::std::sync::TryLockError<T>> for Error {
    fn from(_: ::std::sync::TryLockError<T>) -> Self {
        ErrorKind::TryLock.into()
    }
}

pub struct Exception {
    pub class: String,
    pub msg: String,
}

pub trait ToException {
    fn to_exception(&self) -> Exception;
}
