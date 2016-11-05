pub mod sys;

#[cfg(not(feature = "sys-only"))]
#[macro_use]
extern crate log;

#[cfg(not(feature = "sys-only"))]
#[macro_use]
extern crate error_chain;

#[cfg(not(feature = "sys-only"))]
extern crate combine;

#[cfg(not(feature = "sys-only"))]
extern crate cesu8;

#[cfg(not(feature = "sys-only"))]
mod wrapper {
    #[macro_use]
    mod macros;

    // errors. do you really need an explanation?
    pub mod errors;

    pub mod descriptors;

    // parser for method signatures
    pub mod signature;

    pub mod objects;

    pub mod strings;

    // Actual communication with the JVM
    mod jnienv;
    pub use self::jnienv::*;
}

#[cfg(not(feature = "sys-only"))]
pub use wrapper::*;
