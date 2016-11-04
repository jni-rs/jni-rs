use jclass::JClass;
use jmethodid::JMethodID;
use self::Desc::Descriptor;
use self::Desc::Value;

// "Either"-like enum for java types that can have descriptors.
// This is to facilitate easier optimization of jni calls that
// could either take an actual object or look it up in the jvm
// from a descriptor.
#[derive(Debug)]
pub enum Desc<D, V> {
    Descriptor(D),
    Value(V),
}

pub trait IntoDesc<D, V> {
    fn into_desc(self) -> Desc<D, V>;
}
