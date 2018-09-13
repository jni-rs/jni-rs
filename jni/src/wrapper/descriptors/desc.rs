use JNIEnv;
use errors::*;

/// Trait for things that can be looked up through the JNI via a descriptor.
/// This will be something like the fully-qualified class name
/// `java/lang/String` or a tuple containing a class descriptor, method name,
/// and method signature. For convenience, this is also implemented for the
/// concrete types themselves in addition to their descriptors.
pub trait Desc<'a, T> {
    /// Look up the concrete type from the JVM.
    fn lookup(self, &JNIEnv<'a>) -> Result<T>;
}

impl<'a, T> Desc<'a, T> for T {
    fn lookup(self, _: &JNIEnv<'a>) -> Result<T> {
        Ok(self)
    }
}
