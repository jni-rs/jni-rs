//! Various macros for making low-level JNI calls more ergonomic
//!
//! Note: all macros must avoid un-hygienic / hidden control flow like `return`
//! or `?`

/// Directly calls an exception-safe Env FFI function, nothing else
///
/// # Safety
///
/// This may only be used with a JNI function that is considered to be safe
/// to call with a pending exception. From the JNI design spec, this includes:
///
/// - `ExceptionOccurred()`
/// - `ExceptionDescribe()`
/// - `ExceptionClear()`
/// - `ExceptionCheck()`
/// - `ReleaseStringChars()`
/// - `ReleaseStringUTFChars()`
/// - `ReleaseStringCritical()`
/// - `Release<Type>ArrayElements()`
/// - `ReleasePrimitiveArrayCritical()`
/// - `DeleteLocalRef()`
/// - `DeleteGlobalRef()`
/// - `DeleteWeakGlobalRef()`
/// - `MonitorExit()`
/// - `PushLocalFrame()`
/// - `PopLocalFrame()`
/// - `DetachCurrentThread()`
///
/// You must also ensure that the arguments you pass are valid for the
/// particular JNI function you are calling.
///
/// When calling any function added after JNI 1.1 you must know that it's valid
/// for the current JNI version.
macro_rules! ex_safe_jni_call_no_post_check_ex {
    ( $jnienv:expr, $version:tt, $name:ident $(, $args:expr )*) => {{
        // Safety: we know that the Env pointer can't be null, since that's
        // checked in `from_raw()`
        let env: *mut jni_sys::JNIEnv = $jnienv.get_raw();
        let interface: *const jni_sys::JNINativeInterface_ = *env;

        ((*interface).$version.$name)(env $(, $args)*)
    }};
}

/// Pre-checks for a pending exception error and then calls an Env FFI function
///
/// Since most JNI functions may trigger undefined behaviour if they are called
/// with a pending exception, this macro will explicitly check for a pending
/// exception (and return a `Err(crate::errors::Error::JavaException)` if one is
/// found) before calling the JNI function.
///
/// # Safety
///
/// You must also ensure that the arguments you pass are valid for the
/// particular JNI function you are calling.
///
/// When calling any function added after JNI 1.1 you must know that it's valid
/// for the current JNI version.
macro_rules! jni_call_no_post_check_ex {
    ( $jnienv:expr, $version:tt, $name:ident $(, $args:expr )*) => {{
        $crate::__must_use(if $jnienv.exception_check() {
            Err(crate::errors::Error::JavaException)
        } else {
            // Safety: we know that the Env pointer can't be null, since that's
            // checked in `from_raw()`
            let env: *mut jni_sys::JNIEnv = $jnienv.get_raw();
            let interface: *const jni_sys::JNINativeInterface_ = *env;

            Ok(((*interface).$version.$name)(env $(, $args)*))
        })
    }};
}

/// Calls a Env function, then checks for a pending exception
///
/// This only checks for an exception, it doesn't clear the exception and so the
/// exception will be thrown if the native code returns to the JVM.
///
/// Returns `Err` if there is a pending exception after the call.
macro_rules! jni_call_post_check_ex {
    ( $jnienv:expr, $version:tt, $name:ident $(, $args:expr )* ) => ({
        jni_call_no_post_check_ex!($jnienv, $version, $name $(, $args)*).and_then(|ret| {
            if $jnienv.exception_check() {
                Err(crate::errors::Error::JavaException)
            } else {
                Ok(ret)
            }
        })
    })
}

/// Calls a Env function, then checks for a pending exception, then checks for a `null` return value
///
/// Returns `Err` if there is a pending exception after the call.
/// Returns `Err(Error::NullPtr)` if the JNI function returns `null`
macro_rules! jni_call_post_check_ex_and_null_ret {
    ( $jnienv:expr, $version:tt, $name:ident $(, $args:expr )* ) => ({
        jni_call_post_check_ex!($jnienv, $version, $name $(, $args)*).and_then(|ret| {
            if ret.is_null() {
                Err($crate::errors::Error::NullPtr(concat!(stringify!($name), " result")))
            } else {
                Ok(ret)
            }
        })
    })
}

/// Calls a Env function, with no post check for exceptions, then checks for a
/// `null` return value
///
/// This will do a pre-check for pending exceptions to avoid undefined behaviour
/// when calling JNI functions that are not exception safe.
///
/// Returns `Err(Error::NullPtr)` if the JNI function returns `null`
macro_rules! jni_call_only_check_null_ret {
    ( $jnienv:expr, $version:tt, $name:ident $(, $args:expr )* ) => ({
        jni_call_no_post_check_ex!($jnienv, $version, $name $(, $args)*).and_then(|ret| {
            if ret.is_null() {
                Err($crate::errors::Error::NullPtr(concat!(stringify!($name), " result")))
            } else {
                Ok(ret)
            }
        })
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
///
/// # Safety
///
/// You must ensure that the arguments you pass are valid for the particular JNI
/// function you are calling.
macro_rules! java_vm_call_unchecked {
    ( $jvm:expr, $version:tt, $name:ident $(, $args:expr )*) => {{
        // Safety: we know that the pointer can't be null, since that's
        // checked in `from_raw()`
        let jvm: *mut jni_sys::JavaVM = $jvm.get_raw();
        ((*(*jvm)).$version.$name)(jvm $(, $args)*)
    }};
}
