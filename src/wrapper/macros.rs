macro_rules! jni_call {
    ( $jnienv:expr, $name:tt $(, $args:expr )* ) => ({
        let res = jni_non_null_call!($jnienv, $name $(, $args)*);
        non_null!(res, concat!(stringify!($name), " result")).into()
    })
}

macro_rules! jni_non_null_call {
    ( $jnienv:expr, $name:tt $(, $args:expr )* ) => ({
        trace!("calling checked jni method: {}", stringify!($name));
        #[allow(unused_unsafe)]
        unsafe {
            trace!("entering unsafe");
            let res = jni_method!($jnienv, $name)($jnienv, $($args),*);
            check_exception!($jnienv);
            trace!("exiting unsafe");
            res
        }
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

macro_rules! jni_void_call {
    ( $jnienv:expr, $name:tt $(, $args:expr )* ) => ({
        trace!("calling checked jni method: {}", stringify!($name));
        #[allow(unused_unsafe)]
        unsafe {
            trace!("entering unsafe");
            jni_method!($jnienv, $name)($jnienv, $($args),*);
            check_exception!($jnienv);
            trace!("exiting unsafe");
        }
    })
}

macro_rules! jni_unchecked {
    ( $jnienv:expr, $name:tt $(, $args:expr )* ) => ({
        trace!("calling unchecked jni method: {}", stringify!($name));
        let res = jni_method!($jnienv, $name)($jnienv, $($args),*);
        res
    })
}

macro_rules! jni_method {
    ( $jnienv:expr, $name:tt ) => ({
        trace!("looking up jni method {}", stringify!($name));
        let env = $jnienv;
        match deref!(deref!(env, "JNIEnv"), "*JNIEnv").$name {
            Some(meth) => {
                trace!("found jni method");
                meth
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
        #[allow(unused_unsafe)]
        let check = unsafe {
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
        let res = java_vm_method!($java_vm, $name)($java_vm, $($args),*);
        res
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
            *$obj
        }
    };
}
