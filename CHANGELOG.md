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


## [0.20.0] — 2022-10-17

### Added
- `Default` trait implemented for `JObject`, `JString`, `JClass`, and `JByteBuffer` ([#199](https://github.com/jni-rs/jni-rs/issues/199))
- `Debug` trait implemented for `JavaVM`, `GlobalRef`, `GlobalRefGuard`, `JStaticMethodID` and `ReleaseMode` ([#345](https://github.com/jni-rs/jni-rs/pull/345))
- `ReturnType` for specifying object return types without a String allocation. ([#329](https://github.com/jni-rs/jni-rs/issues/329))

### Changed
- The `release_string_utf_chars` function has been marked as unsafe. ([#334](https://github.com/jni-rs/jni-rs/pull/334))
- Mark `JNIEnv::new_direct_byte_buffer` as `unsafe` ([#320](https://github.com/jni-rs/jni-rs/pull/320))
- `JNIEnv::new_direct_byte_buffer` now takes a raw pointer and size instead of a slice ([#351](https://github.com/jni-rs/jni-rs/pull/351) and [#364](https://github.com/jni-rs/jni-rs/pull/364))
- `JNIEnv::direct_buffer_address` returns a raw pointer instead of a slice ([#364](https://github.com/jni-rs/jni-rs/pull/364))
- The lifetime of `AutoArray` is no longer tied to the lifetime of a particular `JNIEnv` reference. ([#302](https://github.com/jni-rs/jni-rs/issues/302))
- Relaxed lifetime restrictions on `JNIEnv::new_local_ref`. Now it can be used to create a local
  reference from a global reference. ([#301](https://github.com/jni-rs/jni-rs/issues/301) / [#319](https://github.com/jni-rs/jni-rs/pull/319))
- `JMethodID` and `JStaticMethodID` implement `Send` + `Sync` and no longer has a lifetime parameter, making method
  IDs cacheable (with a documented 'Safety' note about ensuring they remain valid). ([#346](https://github.com/jni-rs/jni-rs/pull/346))
- `JFieldID` and `JStaticFieldID` implement `Send` + `Sync` and no longer has a lifetime parameter, making field
  IDs cacheable (with a documented 'Safety' note about ensuring they remain valid). ([#346](https://github.com/jni-rs/jni-rs/pull/365))
- The `call_*_method_unchecked` functions now take `jni:sys::jvalue` arguments to avoid allocating
  a `Vec` on each call to map + collect `JValue`s as `sys:jvalue`s ([#329](https://github.com/jni-rs/jni-rs/issues/329))
- The `From` trait implementations converting `jni_sys` types like `jobject` to `JObject` have been replaced
  with `unsafe` `::from_raw` functions and corresponding `::into_raw` methods. Existing `::into_inner` APIs were
  renamed `::into_raw` for symmetry. ([#197](https://github.com/jni-rs/jni-rs/issues/197))
- The APIs `JNIEnv::set_rust_field`, `JNIEnv::get_rust_field` and `JNIEnv::take_rust_field` have been marked as `unsafe` ([#219](https://github.com/jni-rs/jni-rs/issues/219))

## [0.19.0] — 2021-01-24

### Added
- `AutoArray` and generic `get_array_elements()`, along with `get_<type>_array_elements` helpers. (#287)
- `size()` method to `AutoArray` and `AutoPrimitiveArray`. (#278 / #287)
- `discard()` method to `AutoArray` and `AutoPrimitiveArray`. (#275 / #287)

### Changed
- Removed AutoPrimitiveArray::commit(). (#290)
- `AutoByte/PrimitiveArray.commit()` now returns `Result`. (#275)
- Removed methods get/release/commit_byte/primitive_array_{elements|critical}. (#281)
- Renamed methods get_auto_byte/long/primitive_array_{elements|critical} to
	get_byte/long/primitive_array_{elements|critical}. (#281)

## [0.18.0] — 2020-09-23

### Added
- `JNIEnv#define_unnamed_class` function that allows loading a class without
  specifying its name. The name is inferred from the class data. (#246)
- `SetStatic<type>Field`. (#248)
- `TryFrom<JValue>` for types inside JValue variants (#264).
- Implemented Copy for JNIEnv (#255).
- `repr(transparent)` attribute to JavaVM struct (#259)

### Changed
- Switch from `error-chain` to `thiserror`, making all errors `Send`. Also, support all JNI errors
  in the `jni_error_code_to_result` function and add more information to the `InvalidArgList`
  error. ([#242](https://github.com/jni-rs/jni-rs/pull/242))

## [0.17.0] — 2020-06-30

### Added
- Get/ReleaseByteArrayElements, and Get/ReleasePrimitiveArrayCritical. (#237)

## [0.16.0] — 2020-02-28

### Fixed
- Java VM instantiation with some MacOS configurations. (#220, #229, #230).

## [0.15.0] — 2020-02-28

### Added
- Ability to pass object wrappers that are convertible to `JObject` as arguments to the majority
 of JNIEnv methods without explicit conversion. (#213)
- `JNIEnv#is_same_object` implementation. (#213)
- `JNIEnv#register_native_methods`. (#214)
- Conversion from `Into<JObject>` to `JValue::Object`.

### Fixed
- Passing `null` as class loader to `define_class` method now allowed according
  to the JNI specification. (#225)

## [0.14.0] — 2019-10-31

### Changed
- Relaxed some lifetime restrictions in JNIEnv to support the case when
  method, field ids; and global references to classes
  have a different (larger) lifetime than JNIEnv. (#209)

## [0.13.1] — 2019-08-22

### Changed
- Various documentation improvements.

## [0.13.0] — 2019-07-05

0.13 brings major improvements in thread management, allowing to attach the native threads
permanently and safely; `Executor` for extra convenience and safety; and other
improvements and fixes.

:warning: If your code attaches native threads — make sure to check the updated documentation
of [JavaVM](https://docs.rs/jni/0.13.0/jni/struct.JavaVM.html) to learn about the new features!

### Added
- `JavaVM::attach_current_thread_permanently` method, which attaches the current
  thread and detaches it when the thread finishes. Daemon threads attached
  with `JavaVM::attach_current_thread_as_daemon` also automatically detach themselves
  when finished. The number of currently attached threads may be acquired using
  `JavaVM::threads_attached` method. (#179, #180)
- `Executor` — a simple thread attachment manager which helps to safely
  execute a closure in attached thread context and to automatically free
  created local references at closure exit. (#186)

### Changed
- The default JNI API version in `InitArgsBuilder` from V1 to V8. (#178)
- Extended the lifetimes of `AutoLocal` to make it more flexible. (#190)
- Default exception type from checked `java.lang.Exception` to unchecked `java.lang.RuntimeException`.
  It is used implicitly when `JNIEnv#throw` is invoked with exception message:
  `env.throw("Exception message")`; however, for efficiency reasons, it is recommended
  to specify the exception type explicitly *and* use `throw_new`:
  `env.throw_new(exception_type, "Exception message")`. (#194)

### Fixed
- Native threads attached with `JavaVM::attach_current_thread_as_daemon` now automatically detach
  themselves on exit, preventing Java Thread leaks. (#179)
- Local reference leaks in `JList`, `JMap` and `JMapIter`. (#190, #191)

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

[Unreleased]: https://github.com/jni-rs/jni-rs/compare/v0.20.0...HEAD
[0.20.0]: https://github.com/jni-rs/jni-rs/compare/v0.19.0...v0.20.0
[0.19.0]: https://github.com/jni-rs/jni-rs/compare/v0.18.0...v0.19.0
[0.18.0]: https://github.com/jni-rs/jni-rs/compare/v0.17.0...v0.18.0
[0.17.0]: https://github.com/jni-rs/jni-rs/compare/v0.16.0...v0.17.0
[0.16.0]: https://github.com/jni-rs/jni-rs/compare/v0.15.0...v0.16.0
[0.15.0]: https://github.com/jni-rs/jni-rs/compare/v0.14.0...v0.15.0
[0.14.0]: https://github.com/jni-rs/jni-rs/compare/v0.13.1...v0.14.0
[0.13.1]: https://github.com/jni-rs/jni-rs/compare/v0.13.0...v0.13.1
[0.13.0]: https://github.com/jni-rs/jni-rs/compare/v0.12.3...v0.13.0
[0.12.3]: https://github.com/jni-rs/jni-rs/compare/v0.12.2...v0.12.3
[0.12.2]: https://github.com/jni-rs/jni-rs/compare/v0.12.1...v0.12.2
[0.12.1]: https://github.com/jni-rs/jni-rs/compare/v0.12.0...v0.12.1
[0.12.0]: https://github.com/jni-rs/jni-rs/compare/v0.11.0...v0.12.0
[0.11.0]: https://github.com/jni-rs/jni-rs/compare/v0.10.2...v0.11.0
[0.10.2]: https://github.com/jni-rs/jni-rs/compare/v0.10.1...v0.10.2
[0.10.1]: https://github.com/jni-rs/jni-rs/compare/v0.1...v0.10.1
