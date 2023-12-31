/// Directly calls a JNIEnv FFI function, nothing else
///
/// # Safety
///
/// When calling any function added after JNI 1.1 you must know that it's valid
/// for the current JNI version.
macro_rules! jni_call_unchecked {
    ( $jnienv:expr, $version:tt, $name:tt $(, $args:expr )*) => {{
        // Safety: we know that the JNIEnv pointer can't be null, since that's
        // checked in `from_raw()`
        let env: *mut jni_sys::JNIEnv = $jnienv.get_raw();
        let interface: *const jni_sys::JNINativeInterface_ = *env;
        ((*interface).$version.$name)(env $(, $args)*)
    }};
}

/// Calls a JNIEnv function, then checks for a pending exception
///
/// This only checks for an exception, it doesn't map an exception into
/// an Error and it doesn't clear the exception and so the exception will
/// be thrown if the native code returns to the JVM.
///
/// Returns `Err` if there is a pending exception after the call.
macro_rules! jni_call_check_ex {
    ( $jnienv:expr, $version:tt, $name:tt $(, $args:expr )* ) => ({
        let ret = jni_call_unchecked!($jnienv, $version, $name $(, $args)*);
        if $jnienv.exception_check() {
            Err(crate::errors::Error::JavaException(crate::errors::JavaException::force_capture($jnienv)))
        } else {
            Ok(ret)
        }
    })
}

/// Calls a JNIEnv function, then checks for a pending exception, then checks for a `null` return value
///
/// Returns `Err` if there is a pending exception after the call.
/// Returns `Err(Error::NullPtr)` if the JNI function returns `null`
macro_rules! jni_call_check_ex_and_null_ret {
    ( $jnienv:expr, $version:tt, $name:tt $(, $args:expr )* ) => ({
        jni_call_check_ex!($jnienv, $version, $name $(, $args)*).and_then(|ret| {
            if ret.is_null() {
                Err($crate::errors::Error::NullPtr(concat!(stringify!($name), " result")))
            } else {
                Ok(ret)
            }
        })
    })
}

/// Calls a JNIEnv function, with no check for exceptions, then checks for a `null` return value
///
/// Returns `Err(Error::NullPtr)` if the JNI function returns `null`
macro_rules! jni_call_only_check_null_ret {
    ( $jnienv:expr, $version:tt, $name:tt $(, $args:expr )* ) => ({
        let ret = jni_call_unchecked!($jnienv, $version, $name $(, $args)*);
        if ret.is_null() {
            Err($crate::errors::Error::NullPtr(concat!(stringify!($name), " result")))
        } else {
            Ok(ret)
        }
    })
}

/// Maps a pointer to either Ok(ptr) or Err(Error::NullPtr)
///
/// This makes it reasonably ergonomic to use `?` to early-exit with an `Err` in
/// case of `null` pointer arguments.
///
/// Unlike earlier macros this avoids using `return`, since that can result in
/// surprising control flow if the caller doesn't realize that a macro might
/// explicitly return from the current function.
macro_rules! null_check {
    ( $obj:expr, $ctx:expr ) => {
        if $obj.is_null() {
            Err($crate::errors::Error::NullPtr($ctx))
        } else {
            Ok($obj)
        }
    };
}

/// Directly calls a JavaVM function, nothing else
macro_rules! java_vm_call_unchecked {
    ( $jvm:expr, $version:tt, $name:tt $(, $args:expr )*) => {{
        // Safety: we know that the pointer can't be null, since that's
        // checked in `from_raw()`
        let jvm: *mut jni_sys::JavaVM = $jvm.get_java_vm_pointer();
        ((*(*jvm)).$version.$name)(jvm $(, $args)*)
    }};
}
