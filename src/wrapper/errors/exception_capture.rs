use std::borrow::Cow;
use std::cell::Cell;
use std::fmt::{self, Display};
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

use crate::errors::Error;
use crate::objects::{GlobalRef, JClass, JString, JThrowable};
use crate::strings::JavaStr;
use crate::JNIEnv;

#[cfg(doc)]
use crate::JavaVM;

/// A captured Java exception. Shows the exception message when [`Display`]ed.
/// Contents of [`Error::JavaException`].
///
/// When a [`JNIEnv`] method such as [`JNIEnv::call_method`] results in a Java
/// exception being thrown, it returns `Err(Error::JavaException)`, which
/// contains an instance of this type. This type, in turn, contains information
/// about the exception, including the exception message, which can be
/// [`Display`]ed as an error message.
///
/// The creation of a `JavaException` from the currently pending Java exception
/// is referred to as <dfn>capturing</dfn> the exception, and is done by the
/// [`JavaException::capture`] method.
///
///
/// # Warning: avoid moving to unattached threads
///
/// For best performance, don't send a `JavaException`, or anything containing
/// it, to a thread that is not attached to the JVM.
///
/// When a `JavaException` instance is `Display`ed as an error message or
/// dropped, the thread where that happens must be attached to the Java
/// Virtual Machine. The thread is temporarily attached, if necessary, using
/// [`JavaVM::attach_current_thread`], which is very slow. A warning is logged
/// using the [`log`] crate whenever this happens.
#[derive(Debug)]
pub struct JavaException(JavaExceptionInner);

#[derive(Debug)]
enum JavaExceptionInner {
    /// An exception was captured correctly.
    Captured(GlobalRef),

    /// An exception was thrown, but capturing was suppressed at the time.
    Suppressed,

    /// An exception was thrown, but capturing it resulted in another error.
    ErrorCapturing(Box<Error>),

    /// An exception was supposedly thrown, but when we went to capture it, no
    /// exception was pending.
    Missing,
}

thread_local!(static SUPPRESSING_EXCEPTION_CAPTURE: Cell<usize> = Cell::new(0));

impl JavaException {
    /// Captures the [currently pending Java
    /// exception][JNIEnv::exception_occurred].
    ///
    /// Returns `None` if no Java exception is currently pending.
    pub fn capture(env: &JNIEnv) -> Option<Self> {
        // If captures are being suppressed, just check if an exception is
        // pending, which unlike a full capture is fast and infallible.
        if Self::is_suppressing_captures() {
            return env
                .exception_check()
                .then_some(Self(JavaExceptionInner::Suppressed));
        }

        // We need to suppress capturing while capturing, in order to prevent
        // infinite recursion.
        Self::suppress_capturing(|| {
            // Safety: all local references created inside this function are
            // deleted before it returns.
            let mut env = unsafe { env.unsafe_clone() };

            env.with_local_frame(2, |env| {
                // Capture the currently pending exception.
                let exception: JThrowable = match env.exception_occurred() {
                    Some(e) => e,

                    // If no exception is currently pending, then there's
                    // nothing to do here.
                    None => return Ok(None),
                };

                // Temporarily clear the currently pending exception, so that
                // we can make a global reference to it. Per JNI spec, it is
                // not safe to create a global reference while an exception is
                // pending.
                env.exception_clear();

                // Try to make that global reference. That might fail, but
                // don't bail just yet.
                let exception_global: super::Result<GlobalRef> = env.new_global_ref(&exception);

                // Try to make the exception pending again.
                if let Err(error) = env.throw(exception) {
                    // That shouldn't ever fail, but just in case…
                    log::error!("Error re-throwing captured exception: {error}");
                }

                // Now, if there was an error making that global reference,
                // bail.
                let exception_global: GlobalRef = exception_global?;

                // Wrap it up.
                Ok(Some(Self(JavaExceptionInner::Captured(exception_global))))
            })
            // It is exceedingly unlikely that any error will occur in
            // capturing an exception, but not impossible. Report any errors
            // that occur.
            .unwrap_or_else(|error| Some(Self(JavaExceptionInner::ErrorCapturing(Box::new(error)))))
        })
    }

    /// Captures the currently pending exception as in
    /// [`JavaException::capture`], but returns a dummy `JavaException` instead
    /// of `None` if no exception is pending.
    pub(crate) fn force_capture(env: &JNIEnv) -> Self {
        // Note: If the dummy `JavaException` is `Display`ed, the error message
        // says it's a bug in jni-rs, and points to a line in this source code
        // file. If you want to make this method public, be sure to change that
        // error message. It's in the `impl Display for JavaException`, below.
        Self::capture(env).unwrap_or(Self(JavaExceptionInner::Missing))
    }

    /// Executes the provided closure, while preventing the capture of Java
    /// exceptions until it returns.
    ///
    /// This is used to prevent infinite recursion in
    /// [`JavaException::capture`], in the event that capturing an exception
    /// itself causes an exception.
    ///
    /// This affects the current thread only. If another thread is started by
    /// the provided closure, that thread will still capture exceptions as
    /// normal (unless `suppress_capturing` is called again on that thread).
    fn suppress_capturing<T>(f: impl FnOnce() -> T) -> T {
        SUPPRESSING_EXCEPTION_CAPTURE.with(|suppress_count| {
            // Overflow safety: this cannot overflow because:
            //
            // * It is a `usize`, and the stack will overflow long before
            //   `usize::MAX` nested `suppress_capturing` calls are stacked up.
            // * It is decremented before every `suppress_capturing` call
            //   returns, even upon panic.
            suppress_count.set(suppress_count.get() + 1);

            // Unwind safety: the panic is immediately resumed after
            // decrementing the `suppress_count`, so no invalid state is made
            // visible.
            let result = catch_unwind(AssertUnwindSafe(f));

            // Overflow safety: this cannot overflow because the counter's
            // value right now is greater than or equal to 1, and we know that
            // because:
            //
            // * The counter was just incremented, so it must not currently be
            //   at the initial value of zero.
            // * The counter did not overflow when it was incremented (see
            //   above), so it must not have wrapped around back to zero,
            //   either.
            suppress_count.set(suppress_count.get() - 1);

            match result {
                Ok(result) => result,
                Err(panic) => resume_unwind(panic),
            }
        })
    }

    /// Returns true if [`JavaException::suppress_capturing`] is currently in
    /// effect.
    fn is_suppressing_captures() -> bool {
        SUPPRESSING_EXCEPTION_CAPTURE.with(|suppress_count| suppress_count.get() != 0)
    }
}

impl Display for JavaException {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Doing JNI calls inside `Display::fmt` is Fun™. Two different kinds
        // of errors can happen here: `std::fmt::Error` and
        // `jni::errors::Error`. Here's an `enum` to unify them, with
        // `impl From` for each.
        enum DisplayError {
            Fmt(fmt::Error),
            Java(Error),
        }

        impl From<fmt::Error> for DisplayError {
            fn from(error: fmt::Error) -> Self {
                Self::Fmt(error)
            }
        }

        impl From<Error> for DisplayError {
            fn from(error: Error) -> Self {
                Self::Java(error)
            }
        }

        type DisplayResult = std::result::Result<(), DisplayError>;

        // This function gets the characters of a `JString` and writes them to
        // the given `std::fmt::Formatter`, returning the appropriate error
        // upon failure.
        fn fmt_jstring(
            env: &mut JNIEnv,
            string: &JString,
            f: &mut fmt::Formatter,
        ) -> DisplayResult {
            let string: JavaStr = env.get_string(string)?;
            let string: Cow<str> = (&string).into();
            f.write_str(&string)?;
            Ok(())
        }

        // Okay then. Let's get that exception object. If there isn't one for
        // whatever reason, just write a generic error message and bail.
        let exception_ref: &GlobalRef = match &self.0 {
            JavaExceptionInner::Captured(e) => e,

            JavaExceptionInner::ErrorCapturing(capture_error) => return write!(f, "<unidentified Java exception - error capturing: {capture_error}>"),

            JavaExceptionInner::Suppressed => return f.write_str("<unidentified Java exception>"),

            // This happens if a `JNIEnv` method determines that a Java
            // exception has been thrown, but by the time
            // `JNIEnv::exception_occurred` was called (see above), no
            // exception was pending. Should never happen. If it does, then
            // either the JVM is clearing exceptions on its own (i.e. JVM bug),
            // or this library is clearing the exception prematurely.
            JavaExceptionInner::Missing => return f.write_str(concat!("<phantom Java exception - this is a bug in the JVM or jni-rs - see jni-rs source code at ", file!(), ":", line!(), ">")),
        };

        // Suppress capturing while we do this. Otherwise, if this fails with
        // another Java exception, attempting to `Display` the resulting error
        // would cause infinite recursion.
        let fmt_result: DisplayResult = JavaException::suppress_capturing(|| {
            let mut env = exception_ref.vm().attach_current_thread()?;

            if !env.was_already_attached() {
                log::warn!("Displaying a `JavaException` on a thread that is not currently attached to the Java Virtual Machine. Fix your code if this message appears frequently (see the `jni::errors::JavaException` docs).");
            }

            env.with_local_frame(3, |env| {
                // If an exception (which may or may not be the same as the one
                // we're trying to display) is currently pending, set it aside.
                let pending_exception = env.exception_occurred();
                env.exception_clear();

                // We're going to try two different ways to display the
                // exception message: first using
                // `java.lang.Object::toString()`, or if that fails, by just
                // the class name (from `java.lang.Class::getName()`). We're
                // doing this because user-defined exception classes can throw
                // from their `toString` implementation, and we should have a
                // fall-back strategy in case that happens.
                let fmt_result: DisplayResult = (|| {
                    let exception_to_string: JString = env.call_method(
                        exception_ref,
                        "toString",
                        "()Ljava/lang/String;",
                        &[],
                    )?.l()?.into();

                    fmt_jstring(env, &exception_to_string, f)
                })().or_else(|error| {
                    // Only proceed with this closure if `toString` threw an
                    // exception.
                    if !matches!(error, DisplayError::Java(Error::JavaException(_))) {
                        return Err(error);
                    }

                    // Ignore the second exception.
                    env.exception_clear();

                    // Get the exception's class.
                    let exception_class: JClass = env.get_object_class(exception_ref)?;

                    // Get the exception's class' name. This should never
                    // throw.
                    let exception_class_name: JString = env.call_method(
                        exception_class,
                        "getName",
                        "()Ljava/lang/String;",
                        &[],
                    )?.l()?.into();

                    // Write the class name.
                    fmt_jstring(env, &exception_class_name, f)?;

                    // Then write a generic error message explaining that we
                    // couldn't get the exception message.
                    f.write_str(": <error getting exception message: its `toString` method threw another exception>")?;

                    // Ok, done.
                    Ok(())
                });

                // If the preceding threw an exception, clear it.
                env.exception_clear();

                // If an exception was pending before we did all this, put it
                // back where we found it.
                if let Some(pending_exception) = pending_exception {
                    if let Err(error) = env.throw(pending_exception) {
                        log::error!("Error re-throwing pending exception: {error}");
                    }
                }

                // Done.
                fmt_result
            })
        });

        // How'd we do?
        match fmt_result {
            Ok(()) => Ok(()),
            Err(DisplayError::Fmt(error)) => Err(error),
            Err(DisplayError::Java(Error::JavaException(JavaException(
                JavaExceptionInner::Captured(_),
            )))) => {
                f.write_str("<error: getting the exception message resulted in another exception>")
            }
            Err(DisplayError::Java(error)) => {
                write!(f, "<error getting exception message: {error}>")
            }
        }
    }
}

impl std::error::Error for JavaException {}
