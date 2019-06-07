# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

<!-- Use the following sections from the spec: http://keepachangelog.com/en/1.0.0/
  - Added for new features.
  - Changed for changes in existing functionality.
  - Deprecated for soon-to-be removed features.
  - Removed for now removed features.
  - Fixed for any bug fixes.
  - Security in case of vulnerabilities. -->

## [Unreleased]

### Added
- `JavaVM::attach_current_thread_permanently` method, which attaches the current 
  thread and detaches it when the thread finishes. Daemon threads attached 
  with `JavaVM::attach_current_thread_as_daemon` also automatically detach themselves
  when finished. The number of currently attached threads may be acquired using
  `JavaVM::threads_attached` method. The current thread may be detached by calling
  `JavaVM::detach_current_thread`. (#180)

### Changed
- The default JNI API version in `InitArgsBuilder` from V1 to V8. (#178)

## [0.12.3]

### Added
- `From<jboolean>` implementation for `JValue` (#173)
- `Debug` trait for InitArgsBuilder. (#175)
- `InitArgsBuilder#options` returning the collected JVM options. (#177)

## [0.12.2]

### Changed
- Updated documentation of GetXArrayRegion methods (#169)
- Improved ABI compatibility on various platforms (#170)

## [0.12.1]

This release does not bring code changes.

### Changed
- Updated project documentation.

## [0.12.0]

### Changed
- `JString`, `JMap` and `JavaStr` and their respective iterators now require an extra lifetime so
  that they can now work with `&'b JNIEnv<'a>`, where `'a: 'b`.

## [0.11.0]

### Highlights
This release brings various improvements and fixes, outlined below. The most notable changes are:
- `null` is no longer represented as an `Err` with error kind `NullPtr` if it is a value of some 
  nullable Java reference (not an indication of an error). Related issues: #136, #148, #163.
- `unsafe` methods, providing a low-level API similar to JNI, has been marked safe and renamed
  to have `_unchecked` suffix. Such methods can be used to implement caching of class references
  and method IDs to improve performance in loops and frequently called Java callbacks.
  If you have such, check out [the docs][unchecked-docs] and [one of early usages][cache-exonum] 
  of this feature.

[unchecked-docs]: https://docs.rs/jni/0.11.0/jni/struct.JNIEnv.html
[cache-exonum]: https://github.com/exonum/exonum-java-binding/blob/affa85c026c1870b502725b291822c00f199745d/exonum-java-binding/core/rust/src/utils/jni_cache.rs#L40

### Added
- Invocation API support on Windows and AppVeyor CI (#149)

### Changed
- `push_local_frame`, `delete_global_ref` and `release_string_utf_chars`
no longer check for exceptions as they are
[safe](https://docs.oracle.com/javase/10/docs/specs/jni/design.html#exception-handling)
to call if there is a pending exception (#124):
  - `push_local_frame` will now work in case of pending exceptions — as
  the spec requires; and fail in case of allocation errors
  - `delete_global_ref` and `release_string_utf_chars` won't print incorrect
  log messages

- Rename some macros to better express their intent (see #123):
  - Rename `jni_call` to `jni_non_null_call` as it checks the return value
  to be non-null.
  - Rename `jni_non_null_call` (which may return nulls) to `jni_non_void_call`.

- A lot of public methods of `JNIEnv` have been marked as safe, all unsafe code 
  has been isolated inside internal macros. Methods with `_unsafe` suffixes have
  been renamed and now have `_unchecked` suffixes (#140)

- `from_str` method of the `JavaType` has been replaced by the `FromStr`
  implementation

- Implemented Sync for GlobalRef (#102).

- Improvements in macro usage for JNI methods calls (#136):
  - `call_static_method_unchecked` and `get_static_field_unchecked` methods are 
  allowed to return NULL object
  - Added checking for pending exception to the `call_static_method_unchecked` 
  method (eliminated WARNING messages in log)
  
- Further improvements in macro usage for JNI method calls (#150):
  - The new_global_ref() and new_local_ref() functions are allowed to work with NULL objects according to specification.
  - Fixed the family of functions new_direct_byte_buffer(), get_direct_buffer_address() and get_direct_buffer_capacity()
   by adding checking for null and error codes.
  - Increased tests coverage for JNIEnv functions.

- Implemented Clone for JNIEnv (#147).

- The get_superclass(), get_field_unchecked() and get_object_array_element() are allowed to return NULL object according
 to the specification (#163). 

### Fixed
- The issue with early detaching of a thread by nested AttachGuard. (#139)

## [0.10.2]

### Added
- `JavaVM#get_java_vm_pointer` to retrieve a JavaVM pointer (#98)
- This changelog and other project documents (#106)

### Changed
- The project is moved to an organization (#104)
- Updated versions of dependencies (#105)
- Improved project documents (#107)

### Fixed
- Crate type of a shared library with native methods 
  must be `cdylib` (#100)

## [0.10.1]
- No changes has been made to the Changelog until this release.

[Unreleased]: https://github.com/jni-rs/jni-rs/compare/v0.12.3...HEAD
[0.12.3]: https://github.com/jni-rs/jni-rs/compare/v0.12.2...v0.12.3
[0.12.2]: https://github.com/jni-rs/jni-rs/compare/v0.12.1...v0.12.2
[0.12.1]: https://github.com/jni-rs/jni-rs/compare/v0.12.0...v0.12.1
[0.12.0]: https://github.com/jni-rs/jni-rs/compare/v0.11.0...v0.12.0
[0.11.0]: https://github.com/jni-rs/jni-rs/compare/v0.10.2...v0.11.0
[0.10.2]: https://github.com/jni-rs/jni-rs/compare/v0.10.1...v0.10.2
[0.10.1]: https://github.com/jni-rs/jni-rs/compare/v0.1...v0.10.1
