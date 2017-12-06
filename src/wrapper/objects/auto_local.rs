use std::mem;

use objects::JObject;

use JNIEnv;

/// Auto-delete wrapper for local refs.
///
/// Anything passed to a foreign method is considered a local ref unless it's
/// explicitly turned into a global. These refs are automatically deleted once
/// the foreign method exits, but it's possible that they may reach the
/// JVM-imposed limit before that happens.
///
/// This wrapper provides automatic local ref deletion when it goes out of
/// scope.
///
/// NOTE: This comes with some potential safety risks. DO NOT use this to wrap
/// something unless you're SURE it won't be used after this wrapper gets
/// dropped. Otherwise, you'll get a nasty JVM crash.
pub struct AutoLocal<'a> {
    obj: JObject<'a>,
    env: &'a JNIEnv<'a>,
}

impl<'a> AutoLocal<'a> {
    /// Creates a new auto-delete wrapper for a local ref.
    ///
    /// Once this wrapper goes out of scope, the `delete_local_ref` will be
    /// called on the object. While wrapped, the object can be accessed via
    /// the `Deref` impl.
    pub fn new(env: &'a JNIEnv<'a>, obj: JObject<'a>) -> Self {
        AutoLocal { obj: obj, env: env }
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
    pub fn as_obj<'b>(&'b self) -> JObject<'b>
    where
        'a: 'b,
    {
        self.obj
    }
}

impl<'a> Drop for AutoLocal<'a> {
    fn drop(&mut self) {
        let res = self.env.delete_local_ref(self.obj);
        match res {
            Ok(()) => {}
            Err(e) => debug!("error dropping global ref: {:#?}", e),
        }
    }
}
