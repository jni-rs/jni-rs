// A JNI call that is expected to return a non-null pointer when successful.
// If a null pointer is returned, it is converted to an Err.
// Returns Err if there is a pending exception after the call.
macro_rules! jni_non_null_call {
    ( $jnienv:expr, $name:tt $(, $args:expr )* ) => ({
        let res = jni_non_void_call!($jnienv, $name $(, $args)*);
        non_null!(res, concat!(stringify!($name), " result")).into()
    })
}

// A non-void JNI call. May return anything — primitives, references, error codes.
// Returns Err if there is a pending exception after the call.
macro_rules! jni_non_void_call {
    ( $jnienv:expr, $name:tt $(, $args:expr )* ) => ({
        trace!("calling checked jni method: {}", stringify!($name));

        let res = unsafe {
            jni_method!($jnienv, $name)($jnienv, $($args),*)
        };

        check_exception!($jnienv);
        res
    })
}

macro_rules! non_null {
    ( $obj:expr, $ctx:expr ) => {
        if $obj.is_null() {
            return Err($crate::errors::ErrorKind::NullPtr($ctx).into());
        } else {
            $obj
        }
    }
}

// A void JNI call.
// Returns Err if there is a pending exception after the call.
macro_rules! jni_void_call {
    ( $jnienv:expr, $name:tt $(, $args:expr )* ) => ({
        trace!("calling checked jni method: {}", stringify!($name));

        unsafe {
            jni_method!($jnienv, $name)($jnienv, $($args),*)
        };

        check_exception!($jnienv);
    })
}

// A JNI call that does not check for exceptions or verify
// error codes (if any).
macro_rules! jni_unchecked {
    ( $jnienv:expr, $name:tt $(, $args:expr )* ) => ({
        trace!("calling unchecked jni method: {}", stringify!($name));

        unsafe {
            jni_method!($jnienv, $name)($jnienv, $($args),*)
        }
    })
}

macro_rules! jni_method {
    ( $jnienv:expr, $name:tt ) => ({
        trace!("looking up jni method {}", stringify!($name));
        let env = $jnienv;
        match deref!(deref!(env, "JNIEnv"), "*JNIEnv").$name {
            Some(method) => {
                trace!("found jni method");
                method
            },
            None => {
                trace!("jnienv method not defined, returning error");
                return Err($crate::errors::Error::from(
                    $crate::errors::ErrorKind::JNIEnvMethodNotFound(
                        stringify!($name))).into())},
        }
    })
}

macro_rules! check_exception {
    ( $jnienv:expr ) => {
        trace!("checking for exception");
        let check = {
            jni_unchecked!($jnienv, ExceptionCheck)
        } == $crate::sys::JNI_TRUE;
        if check {
            trace!("exception found, returning error");
            return Err($crate::errors::Error::from(
                $crate::errors::ErrorKind::JavaException).into());
        }
        trace!("no exception found");
    }
}

macro_rules! catch {
    ( move $b:block ) => {
        (move || $b)()
    };
    ( $b:block ) => {
        (|| $b)()
    };
}

macro_rules! java_vm_unchecked {
    ( $java_vm:expr, $name:tt $(, $args:expr )* ) => ({
        trace!("calling unchecked JavaVM method: {}", stringify!($name));
        java_vm_method!($java_vm, $name)($java_vm, $($args),*)
    })
}

macro_rules! java_vm_method {
    ( $jnienv:expr, $name:tt ) => ({
        trace!("looking up JavaVM method {}", stringify!($name));
        let env = $jnienv;
        match deref!(deref!(env, "JavaVM"), "*JavaVM").$name {
            Some(meth) => {
                trace!("found JavaVM method");
                meth
            },
            None => {
                trace!("JavaVM method not defined, returning error");
                return Err($crate::errors::Error::from(
                    $crate::errors::ErrorKind::JavaVMMethodNotFound(
                        stringify!($name))).into())},
        }
    })
}

macro_rules! deref {
    ( $obj:expr, $ctx:expr ) => {
        if $obj.is_null() {
            return Err($crate::errors::ErrorKind::NullDeref($ctx).into());
        } else {
            #[allow(unused_unsafe)]
            unsafe { *$obj }
        }
    };
}
