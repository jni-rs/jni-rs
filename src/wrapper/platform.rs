#[cfg(windows)]
/// A platform specific name of the java executable.
pub const JAVA_EXE_NAME: &str = "java.exe";
#[cfg(not(windows))]
/// A platform specific name of the java executable.
pub const JAVA_EXE_NAME: &str = "java";

#[cfg(windows)]
/// A platform specific name of the jvm dynamic library.
pub const LIBJVM_NAME: &str = "jvm.dll";
#[cfg(all(unix, any(target_os = "macos", target_os = "ios")))]
/// A platform specific name of the jvm dynamic library.
pub const LIBJVM_NAME: &str = "libjvm.dylib";
#[cfg(all(unix, not(any(target_os = "macos", target_os = "ios"))))]
/// A platform specific name of the jvm dynamic library.
pub const LIBJVM_NAME: &str = "libjvm.so";

#[cfg(windows)]
/// A platform specific separator of paths in corresponding environment variables.
pub const PATHS_SEP: &str = ";";
#[cfg(not(windows))]
/// A platform specific separator of paths in corresponding environment variables.
pub const PATHS_SEP: &str = ":";
