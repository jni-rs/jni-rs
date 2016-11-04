// "Either"-like enum for java types that can have descriptors.
// This is to facilitate easier optimization of jni calls that
// could either take an actual object or look it up in the jvm
// from a descriptor.
#[derive(Debug)]
pub enum Desc<D, V> {
    Descriptor(D),
    Value(V),
}
