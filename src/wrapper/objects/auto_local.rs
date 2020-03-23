use std::mem;

use log::debug;

use crate::{objects::JObject, JNIEnv};

/// Auto-delete wrapper for local refs.
///
/// Anything passed to a foreign method _and_ returned from JNI methods is considered a local ref
/// unless it is specified otherwise.
/// These refs are automatically deleted once the foreign method exits, but it's possible that
/// they may reach the JVM-imposed limit before that happens.
///
/// This wrapper provides automatic local ref deletion when it goes out of
/// scope.
///
/// NOTE: This comes with some potential safety risks. DO NOT use this to wrap
/// something unless you're SURE it won't be used after this wrapper gets
/// dropped. Otherwise, you'll get a nasty JVM crash.
///
/// See also the [JNI specification][spec-references] for details on referencing Java objects
/// and some [extra information][android-jni-references].
///
/// [spec-references]: https://docs.oracle.com/en/java/javase/12/docs/specs/jni/design.html#referencing-java-objects
/// [android-jni-references]: https://developer.android.com/training/articles/perf-jni#local-and-global-references
pub struct AutoLocal<'a: 'b, 'b> {
    obj: JObject<'a>,
    env: &'b JNIEnv<'a>,
}

impl<'a, 'b> AutoLocal<'a, 'b> {
    /// Creates a new auto-delete wrapper for a local ref.
    ///
    /// Once this wrapper goes out of scope, the `delete_local_ref` will be
    /// called on the object. While wrapped, the object can be accessed via
    /// the `Deref` impl.
    pub fn new(env: &'b JNIEnv<'a>, obj: JObject<'a>) -> Self {
        AutoLocal { obj, env }
    }

    /// Forget the wrapper, returning the original object.
    ///
    /// This prevents `delete_local_ref` from being called when the `AutoLocal`
    /// gets
    /// dropped. You must either remember to delete the local ref manually, or
    /// be
    /// ok with it getting deleted once the foreign method returns.
    pub fn forget(self) -> JObject<'a> {
        let obj = self.obj;
        mem::forget(self);
        obj
    }

    /// Get a reference to the wrapped object
    ///
    /// Unlike `forget`, this ensures the wrapper from being dropped while the
    /// returned `JObject` is still live.
    pub fn as_obj<'c>(&self) -> JObject<'c>
    where
        'a: 'c,
    {
        self.obj
    }
}

impl<'a, 'b> Drop for AutoLocal<'a, 'b> {
    fn drop(&mut self) {
        let res = self.env.delete_local_ref(self.obj);
        match res {
            Ok(()) => {}
            Err(e) => debug!("error dropping global ref: {:#?}", e),
        }
    }
}

impl<'a> From<&'a AutoLocal<'a, '_>> for JObject<'a> {
    fn from(other: &'a AutoLocal) -> JObject<'a> {
        other.as_obj()
    }
}
