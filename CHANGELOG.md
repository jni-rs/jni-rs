<!-- markdownlint-disable MD022 MD024 MD032  -->

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- Use the following sections from the spec: https://keepachangelog.com/en/1.0.0/
  - Added for new features.
  - Changed for changes in existing functionality.
  - Deprecated for soon-to-be removed features.
  - Removed for now removed features.
  - Fixed for any bug fixes.
  - Security in case of vulnerabilities. -->

## [Unreleased]

### Added

- Added dependency on `once_cell` version `1.19.0` for lazy initialization of various global statics.

#### JavaVM / Thread Attachment APIs

- `JavaVM::singleton()` lets you acquire the `JavaVM` for the process when you know that the `JavaVM` singleton has been initialized ([#595](https://github.com/jni-rs/jni-rs/pull/595))
- `JavaVM::is_thread_attached` can query whether the current thread is attached to the Java VM ([#570](https://github.com/jni-rs/jni-rs/pull/570))
- `AttachGuard::from_unowned` added as a low-level (unsafe) way to represent a thread attachment with a raw `jni_sys::Env` pointer ([#570](https://github.com/jni-rs/jni-rs/pull/570))
- `AttachConfig` exposes fine-grained control over thread attachment including `Thread` name, `ThreadGroup` and whether scoped or permanent. ([#606](https://github.com/jni-rs/jni-rs/pull/606))
- `JavaVM::attach_current_thread_guard` is a low-level (unsafe) building block for attaching threads that exposes the `AttachGuard` and `AttachConfig` control. ([#606](https://github.com/jni-rs/jni-rs/pull/606))
- `JavaVM::attach_current_thread_with_config` is a safe building block for attaching threads that hides the `AttachGuard` but exposes `AttachConfig` control. ([#606](https://github.com/jni-rs/jni-rs/pull/606))
- `JavaVM::with_local_frame` added as method to borrow a `Env` that is already attached to the current thread, after pushing a new JNI stack frame ([#570](https://github.com/jni-rs/jni-rs/pull/570), [#673](https://github.com/jni-rs/jni-rs/pull/673))
- `JavaVM::with_top_local_frame_frame` added to borrow a `Env` for the top JNI stack frame (i.e. without pushing a new JNI stack frame) ([#570](https://github.com/jni-rs/jni-rs/pull/570), [#673](https://github.com/jni-rs/jni-rs/pull/673))

#### Reference Type APIs

- A `Reference` trait for all reference types like `JObject`, `JClass`, `JString`, enabling `Global` and `Weak` to be generic over `Reference` and enabling safe casting and global caching of `JClass` references. ([#596](https://github.com/jni-rs/jni-rs/pull/596))
- `Reference::lookup_class` exposes a cached `Global<JClass>` for all `Reference` implementations ([#612](https://github.com/jni-rs/jni-rs/pull/612))
- `LoaderContext` + `LoaderContext::load_class` for loading classes, depending on available context ([#612](https://github.com/jni-rs/jni-rs/pull/612))
- `Env::new_cast_global_ref` acts like `new_global_ref` with a type cast ([#612](https://github.com/jni-rs/jni-rs/pull/612))
- `Env::cast_global` takes an owned `Global<From>` and returns an owned `Global<To>` ([#612](https://github.com/jni-rs/jni-rs/pull/612))
- `Env::new_cast_local_ref` acts like `new_local_ref` with a type cast ([#612](https://github.com/jni-rs/jni-rs/pull/612))
- `Env::cast_local` takes an owned local reference and returns a new type-cast wrapper (owned) ([#612](https://github.com/jni-rs/jni-rs/pull/612))
- `Env::as_cast` or `Cast::new` borrows any `From: Reference` (global or local) reference and returns  a `Cast<To>` that will Deref into `&To` ([#612](https://github.com/jni-rs/jni-rs/pull/612))
- `Env::as_cast_unchecked` returns a `Cast<To>` like `as_cast()` but without a runtime `IsInstanceOf` check ([#669](https://github.com/jni-rs/jni-rs/pull/669))
- `Env::as_cast_raw` or `Cast::from_raw` borrows a raw `jobject` reference and returns a `Cast<To>` that will Deref into `&To`
- `Cast::new_unchecked` and `Cast::from_raw_unchecked` let you borrow a reference with an (`unsafe`) type cast, with no runtime check
- `::cast_local()` methods as a convenience for all reference types, such as `let s = JString::cast_local(obj)`
- `const` `null()` methods for all reference types.

#### JNI Environment APIs

- `Env::call_nonvirtual_method` and `Env::call_nonvirtual_method_unchecked` to call non-virtual method. ([#454](https://github.com/jni-rs/jni-rs/issues/454))
- `Env::to_reflected_method` and `Env::to_reflected_static_method` for retrieving the Java reflection API instance for a method or constructor. ([#579](https://github.com/jni-rs/jni-rs/pull/579))
- `Env::throw_new_void` provides an easy way to throw an exception that's constructed with no message argument
- `Env::new_object_type_array<E>` lets you you instantiate a `JObjectArray` with a given element type like `new_object_type_array::<JString>`
- `Env::load_class` supports class lookups via the current `Thread` context class loader, with `FindClass` fallback. ([#674](https://github.com/jni-rs/jni-rs/pull/674))
- `MethodSignature` and `FieldSignature` types have been added for compile-time parsed JNI method and field signatures

#### Native Method APIs

- `EnvUnowned` is an FFI-safe type that can be used to capture a `jni_sys::Env` pointer given to native methods and give it a named lifetime (this can then be temporarily upgraded to a `&mut Env` reference via `EnvUnowned::with_env`) ([#570](https://github.com/jni-rs/jni-rs/pull/570))
- `Outcome` is like a `Result` with the addition of a third `Panic()` variant, used for careful handling of errors in native methods.
- `EnvOutcome` represents an `EnvUnowned::with_env` outcome whose errors can be handle, with access to JNI, via an `ErrorPolicy`.
- `ErrorPolicy` is a trait with `on_error` and `on_panic` methods that can log native method errors or throw them as exceptions.
- `ThrowRuntimeExAndDefault` is an `ErrorPolicy` that throws any error as a `RuntimeException` (and returns a default value).
- `LogErrorAndDefault` is an `ErrorPolicy` that logs errors and returns a default value.
- `LogContextErrorAndDefault` is an `ErrorPolicy` that logs errors, with a given context string, and returns a default value.

#### String APIs

- New functions for converting Rust `char` to and from Java `char` and `int` ([#427](https://github.com/jni-rs/jni-rs/issues/427) / [#434](https://github.com/jni-rs/jni-rs/pull/434))
- `JavaStr/MUTF8Chars`, `JNIStr`, and `JNIString` have several new methods and traits, most notably a `to_str` method that converts to a regular Rust string. ([#510](https://github.com/jni-rs/jni-rs/issues/510) / [#512](https://github.com/jni-rs/jni-rs/pull/512))
- `Global::null()` and `Weak::null()` construct null references (equivalent to `Default::default()`). ([#596](https://github.com/jni-rs/jni-rs/pull/596))

- `JNIStr` now implements `Debug`, `PartialEq`, `Eq`, `PartialOrd`, `Ord` and `Hash` ([#615](https://github.com/jni-rs/jni-rs/pull/615))
- `JNIString` now implements `Debug`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash` and `Clone` ([#615](https://github.com/jni-rs/jni-rs/pull/615))
- `PartialEq<&JNIStr> for JNIString` allows `JNIStr` and `JNIString` to be compared. ([#615](https://github.com/jni-rs/jni-rs/pull/615))
- `From<&JNIStr>` and `From<MUTF8Chars>` implementations for `JNIString`. ([#615](https://github.com/jni-rs/jni-rs/pull/615))
- `JNIStr::from_cstr` safely does a zero-copy cast of a `CStr` to a `JNIStr` after a `const` modified-utf8 encoding validation (with a panic on failure)
- `AsRef<JNIStr>` is implemented for `CStr` (based on `JNIStr::from_cstr`) allows use of literals like `c"java/lang/Foo"` to be passed to JNI APIs without needing to be copied. ([#615](https://github.com/jni-rs/jni-rs/pull/615))
- `JNIStr::to_bytes` gives access to a `&[u8]` slice over the bytes of a JNI string (like `CStr::to_bytes`) ([#615](https://github.com/jni-rs/jni-rs/pull/615))

#### java.lang APIs

- `JClassLoader` as a `Reference` wrapper for `java.lang.ClassLoader` references ([#612](https://github.com/jni-rs/jni-rs/pull/612))
- `JCollection`, `JSet` and `JIterator` reference wrappers for `java.util.Collection`, `java.util.Set` and `java.util.Iterator` interfaces.
- `JList::remove_item` for removing a given value, by-reference, from the list (instead of by index).
- `JList::clear` allows a list to be cleared.
- `JList::is_empty` checks if a list is empty.
- `JList::as_collection` casts a list into a `JCollection`
- `JObjectArray::new` lets you construct a `JObjectArray<E>` with strong element type parameterization, instead of `Env::new_object_array`
- `JObjectArray::get/set_element` let you get and set array elements as methods on the array.
- `JPrimitiveArray::new` lets you construct a `JPrimitiveArray<E>`, consistent with `JObjectArray::new`
- `JStackTraceElement` gives access to stack frame info within a stack trace, like filename, line number etc
- `JString` now has `::new()`, `::from_str` and `::from_jni_str` constructor methods ([#960](https://github.com/jni-rs/jni-rs/pull/690))
- `JThread` as a `Reference` wrapper for `java.lang.Thread` references ([#612](https://github.com/jni-rs/jni-rs/pull/612))
- `JThrowable::get_message` is a binding for `getMessage()` and gives easy access to an exception message
- `JThrowable::get_stack_trace` is a binding for `getStackTrace()`, returning a `JObjectArray<JStackTraceElement>`

#### Macros

- The `#[jni_mangle()]` attribute proc macro can export an `extern "system"` native method with a mangled name like "Java_com_example_myMethod" so it can be automatically resolved within a shared library by the JVM ([#693](https://github.com/jni-rs/jni-rs/pull/693))
- The `jni_str!` and `jni_cstr!` macros can encode a MUTF-8 `&'static JNIStr` or `&' static CStr` at compile time with full unicode support.
- The `jni_sig!`, `jni_sig_str!`, `jni_sig_cstr!` and `jni_sig_jstr!` macros can parse and compile signatures like `(arg0: jint, arg1: JString) -> JString` into `MethodSignature` and `FieldSignature` descriptors or JNI string literals like "(ILjava/lang/String;)Ljava/lang/String;"

### Changed

- `jni-sys` dependency bumped to `0.4` ([#478](https://github.com/jni-rs/jni-rs/issues/478))
- `JNIEnv` is no longer a `#[transparent]` FFI-safe pointer wrapper and has been split into `EnvUnowned` (for FFI/native method args) and `Env` (non-FFI) ([#634](https://github.com/jni-rs/jni-rs/pull/634))
- A `JNIEnv` type alias shows a verbose deprecation warning that explains how to migrate from `JNIEnv` to `EnvUnowned` and `Env` ([#634](https://github.com/jni-rs/jni-rs/pull/634))
- `Env::get_version` has been renamed to `Env::version` ([#478](https://github.com/jni-rs/jni-rs/issues/478))
- JNI version requirements are more explicit in the API and the crate now requires at least JNI `>= 1.4`. It needs `>= 1.2` so it can check for exceptions and needs `>= 1.4` to avoid runtime checks for direct byte buffers ([#478](https://github.com/jni-rs/jni-rs/issues/478))
- At a low-level (unsafe), all thread attachments (not just scoped attachments) are now represented by an owned or unowned `AttachGuard`
- `AttachGuard` usage is now considered `unsafe` since the type must be pinned to the stack (but that can't be guaranteed by the Rust type system alone).
- To allow safe thread attachments (that ensure their `AttachGuard` is pinned to the stack), attachment APIs take a `FnOnce` whose `&mut Env` arg borrows from a hidden `AttachGuard`
  - `JavaVM::attach_current_thread` requests a permanent thread attachment (reducing cost of future `attach_current_thread()` calls)
  - `JavaVM::attach_current_thread_for_scope` requests a thread attachment that's detached after the given closure returns.
- `Env` is no longer ever exposed in the API by-value can only be accessed by borrowing from a thread attachment `AttachGuard`.
- `Env` implements runtime borrow checking to ensure new local references may only be associated with the top JNI stack frame
- `JavaVM::get_env` is replaced by `JavaVM::get_env_attachment` which returns an `AttachGuard` if the current thread is attached.
- The following functions are now infallible ([#478](https://github.com/jni-rs/jni-rs/issues/478)):
  - `Env::version`
  - `Env::get_java_vm`
  - `Env::exception_check`
  - `Env::exception_clear`
  - `Env::exception_describe`
  - `Env::exception_occurred` ([#517](https://github.com/jni-rs/jni-rs/issues/517))
  - `Env::is_same_object`
  - `Env::delete_local_ref`
  - `WeakRef::is_same_object`
  - `WeakRef::is_weak_ref_to_same_object`
  - `WeakRef::is_garbage_collected`
- `Env::fatal_error` is now guaranteed not to panic or allocate, but requires the error message to be encoded ahead of time. ([#480](https://github.com/jni-rs/jni-rs/pull/480))
- `Env::get_native_interface` has been removed since it's redundant and `Env::get_raw` is more consistent with other APIs.
- `Env::register_native_methods` is now marked `unsafe` since it requires all the given function pointers to be valid and match corresponding Java method signatures ([568](https://github.com/jni-rs/jni-rs/pull/568))
- `JavaVM::get_java_vm_pointer` has been renamed `JavaVM::get_raw` for consistency.
- `JavaVM::new` and `JavaVM::with_libjvm` now prevent libjvm from being unloaded. This isn't necessary for HotSpot, but other JVMs could crash if we don't do this. ([#554](https://github.com/jni-rs/jni-rs/pull/554))
- `JValueGen` has been removed. `JValue` and `JValueOwned` are now separate, unrelated, non-generic types. ([#429](https://github.com/jni-rs/jni-rs/pull/429))
- Make `from_raw()`, `into_raw()` and `null()` methods `const fn`. ([#453](https://github.com/jni-rs/jni-rs/pull/453))
- Make `from_raw()` require an `Env` reference so the returned wrapper is guaranteed to have a local reference frame lifetime ([#670](https://github.com/jni-rs/jni-rs/pull/670))
- `get_object_class` borrows the `Env` mutably because it creates a new local reference. ([#456](https://github.com/jni-rs/jni-rs/pull/456))
- `get/set_*_field_unchecked` have been marked as unsafe since they can lead to undefined behaviour if the given types don't match the field type ([#457](https://github.com/jni-rs/jni-rs/pull/457) + [#629](https://github.com/jni-rs/jni-rs/pull/629))
- `set_static_field` takes a field name and signature as strings so the ID is looked up internally to ensure it's valid. ([#629](https://github.com/jni-rs/jni-rs/pull/629))
- `Env::get/set/take_rust_field` no longer require a mutable `Env` reference since they don't return any new local references to the caller ([#455](https://github.com/jni-rs/jni-rs/issues/455))
- `Env::get_rust_field` returns a `MutexGuard<'local>` instead of taking the `&'env self` lifetime (so you don't lose any `&mut Env` reference you have) ([#675](https://github.com/jni-rs/jni-rs/pull/675))
- `Env::is_assignable_from` and `is_instance_of` no longer requires a mutable `Env` reference, since they doesn't return any new local references to the caller
- `JavaStr` has been renamed `MUTF8Chars` (with a deprecated `JavaStr` alias) and is intended to be got via `JString::mutf8_chars()`
- `JavaStr/MUTF8Chars::from_env` has been removed because it was unsound (it could cause undefined behavior and was not marked `unsafe`). Use `JString::mutf8_chars` instead. ([#510](https://github.com/jni-rs/jni-rs/issues/510) / [#512](https://github.com/jni-rs/jni-rs/pull/512))
- `JavaStr/MUTF8Chars::get_raw` has been renamed to `as_ptr`. ([#510](https://github.com/jni-rs/jni-rs/issues/510) / [#512](https://github.com/jni-rs/jni-rs/pull/512))
- `JavaStr/MUTF8Chars`, `JNIStr`, and `JNIString` no longer coerce to `CStr`, because using `CStr::to_str` will often have incorrect results. You can still get a `CStr`, but must use the new `as_cstr` method to do so. ([#510](https://github.com/jni-rs/jni-rs/issues/510) / [#512](https://github.com/jni-rs/jni-rs/pull/512))
- All APIs that were accepting modified-utf8 string args via `Into<JNIString>`, now take `AsRef<JNIStr>` to avoid string copies every call. Considering that these strings are often literals for signatures or class names, most code can rely on `AsRef<JNIStr>` for `CStr` and pass `CStr` literals like `env.find_class(c"java/lang/Foo")`. ([#617](https://github.com/jni-rs/jni-rs/pull/617))
- `JavaStr/MUTF8Chars` and `JString` both implement `Display` and therefore `ToString`, making it even easier to get a Rust `String`.
- `Env::get_string` performance was optimized by caching an expensive class lookup, and using a faster instanceof check. ([#531](https://github.com/jni-rs/jni-rs/pull/531))
- `Env::get_string` performance was later further optimized to avoid the need for runtime type checking ([#612](https://github.com/jni-rs/jni-rs/pull/612))
- `Env::get_string` has been deprecated in favor of `JString::mutf8_chars` and `JString::to_string()` or `JString::try_to_string(env)`

- `GlobalRef` and `WeakRef` have been renamed to `Global` and `Weak` and are now generic, parameterized, transparent wrappers over `'static` reference types like `Global<JClass<'static>>` (no longer an `Arc` holding a reference and VM pointer) ([#596](https://github.com/jni-rs/jni-rs/pull/596))
  - `Global` and `Weak` no longer implement `Clone`, since JNI is required to create new reference (you'll need to explicitly use `env.new_global_ref`)
  - `Global` and `Weak` both implement `Default`, which will represent `::null()` references (equivalent to `JObject::null()`)
- `Global::into_raw` replaces `Global::try_into_raw` and is infallible ([#596](https://github.com/jni-rs/jni-rs/pull/596))
- `Env::new_weak_ref` returns a `Result<Weak>` and `Error::ObjectFreed` if the reference is null or has already been freed (instead of `Result<Option<Weak>>`) ([#596](https://github.com/jni-rs/jni-rs/pull/596))
- `Env::new_global_ref` and `::new_local_ref` may return `Error::ObjectFreed` in case a weak reference was given and the object has been freed. ([#596](https://github.com/jni-rs/jni-rs/pull/596))
- `Env::define_class` takes a `name: Option<>` instead of having a separate `define_unnamed_class` API.
- `Env::define_class_bytearray` was renamed to `Env::define_class_jbyte` and is identical to `define_class` except for taking a `&[jbyte]` slice instead of `&[u8]`, which is a convenience if you have a `JByteArray` or `AutoElements<JByteArray>`.
- `Env::define_class[_jbyte]` now takes a `loader: AsRef<JClassLoader>` instead of `loader: &JObject`.
- `AutoElements` was simplified to only be parameterized by one lifetime for the array reference, and accepts any `AsRef<JPrimitiveArray<T>>` as a reference. ([#508](https://github.com/jni-rs/jni-rs/pull/508))
- `JavaType` was simplified to not capture object names or array details (like `ReturnType`) since these details don't affect `JValue` type checks and had a hidden cost that was redundant.
- `Env::with_local_frame` can be used with a shared `&Env` reference since it doesn't return a new local reference. ([#673](https://github.com/jni-rs/jni-rs/pull/673))
- `Env::with_local_frame_returning_local` can now return any kind of local `Reference`, not just `JObject`
- `JList` is a simpler, transparent reference wrapper implementing `Reference`, like `JObject`, `JClass`, `JString` etc
- `JList::add` returns the boolean returned by the Java API
- `JList::remove` no longer returns an `Option` since there's nothing special about getting a `null` from the Java `remove` API.
- `JList::pop` is deprecated since this doesn't map to standard Java `List` method.
- `JList::iter` returns a `JIterator` instead of a `JListIter`
- `Env::get_list` has been deprecated, in favor of `JList::cast_local`, or other generic `Env` `cast_local/cast_global` APIs.
- `Env::get_array_elements` is deprecated in favor of `JPrimitiveArray::get_elements`
- `Env::get_array_elements_critical` is deprecated in favor of `JPrimitiveArray::get_elements_critical`
- `Env::get_*_array_region` and `Env::set_*_array_region` are deprecated in favor of `JPrimitiveArray::get/set_region`
- `Env::get_array_length` is deprecated in favor of `JPrimitiveArray::len` and `JObjectArray::len`
- `Env::get/set_object_array_element` are deprecated in favor of `JObjectArray::get/set_element`
- `Env::new_*_array` methods for primitive array types (like `JByteArray`) take a `&mut Env` and a `usize` len, and the docs recommend using `J<Type>Array::new()` instead.
- `Env::new_object_unchecked` now takes a `Desc<JMethodID>` for consistency/flexibility instead of directly taking a `JMethodID`
- `JObjectArray` supports generic element types like `JObjectArray<JString>`
- `AutoLocal` has been renamed to `Auto` with a deprecated type alias for `AutoLocal` to sign post the rename.
- The documentation for `Env::find_class` now recommends considering `LoaderContext::load_class` instead.
- `Desc<JClass>::lookup()` is now based on `LoaderContext::load_class` (instead of `Env::find_class`), which checks for a thread context class loader by default.
- `AutoElements[Critical]::discard()` now takes ownership of the elements and drops them to release the pointer after setting the mode to `NoCopyBack` ([#645](https://github.com/jni-rs/jni-rs/pull/645))
- Mark `MonitorGuard` with `#[must_use]` to warn when the guard is dropped accidentally ([#676](https://github.com/jni-rs/jni-rs/pull/676))
- `NativeMethod` (used with `Env::register_native_methods`) is a now a transparent `jni::sys::JNINativeWrapper` wrapper with an `unsafe` `::from_raw_parts` constructor.

### Fixed
- `Env::get_string` no longer leaks local references. ([#528](https://github.com/jni-rs/jni-rs/pull/528), [#557](https://github.com/jni-rs/jni-rs/pull/557))

### Removed
- `JavaVM::attach_current_thread_as_daemon` (and general support for 'daemon' threads) has been removed, since their semantics are inherently poorly defined and unsafe (the distinction relates to the poorly defined limbo state after calling `JavaDestroyVM`, where it becomes undefined to touch the JVM) ([#593](https://github.com/jni-rs/jni-rs/pull/593))
- The 'Executor' API has been removed (`AttachGuard::with_env` can be used instead) ([#570](https://github.com/jni-rs/jni-rs/pull/570))
- `Env::from_raw`, `Env::from_raw_unchecked` and `Env::unsafe_clone` have been removed, since the API no longer exposes the `Env` type by-value, it must always be borrowed from an `AttachGuard`. ([#570](https://github.com/jni-rs/jni-rs/pull/570))
- `Error::NullDeref` and `Error::JavaVMMethodNotFound` have been removed since they were unused.
- `JavaType::Method` was removed since a method signature isn't a type, and all usage was being matched as unreachable or an error.
- `Env::define_unnamed_class` was removed in favor of having the `define_class[_jbyte]` APIs take a `name: Option` instead.

## [0.21.1] — 2023-03-08

### Fixes
- Compilation is fixed for architectures with a C ABI that has unsigned `char` types. ([#419](https://github.com/jni-rs/jni-rs/pull/419))
- `JNIEnv::get_string` no longer leaks local references. ([#528](https://github.com/jni-rs/jni-rs/pull/528))

## [0.21.0] — 2023-02-13

This release makes extensive breaking changes in order to improve safety. Most projects that use this library will need to be changed. Please see [the migration guide](docs/0.21-MIGRATION.md).

### Added
- `JavaStr::into_raw()` which drops the `JavaStr` and releases ownership of the raw string pointer ([#374](https://github.com/jni-rs/jni-rs/pull/374))
- `JavaStr::from_raw()` which takes ownership of a raw string pointer to create a `JavaStr` ([#374](https://github.com/jni-rs/jni-rs/pull/374))
- `JNIEnv::get_string_unchecked` is a cheaper, `unsafe` alternative to `get_string` that doesn't check the given object is a `java.lang.String` instance. ([#328](https://github.com/jni-rs/jni-rs/issues/328))
- `WeakRef` and `JNIEnv#new_weak_ref`. ([#304](https://github.com/jni-rs/jni-rs/pull/304))
- `define_class_bytearray` method that takes an `AutoElements<jbyte>` rather than a `&[u8]` ([#244](https://github.com/jni-rs/jni-rs/pull/244))
- `JObject` now has an `as_raw` method that borrows the `JObject` instead of taking ownership like `into_raw`. Needed because `JObject` no longer has the `Copy` trait. ([#392](https://github.com/jni-rs/jni-rs/issues/392))
- `JavaVM::destroy()` (unsafe) as a way to try and unload a `JavaVM` on supported platforms ([#391](https://github.com/jni-rs/jni-rs/issues/391))
- `JavaVM::detach_current_thread()` (unsafe) as a way to explicitly detach a thread (normally this is automatic on thread exit). Needed to detach daemon threads manually if using `JavaVM::destroy()` ([#391](https://github.com/jni-rs/jni-rs/issues/391))
- `JPrimitiveArray<T: TypeArray>` and type-specific aliases like `JByteArray`, `JIntArray` etc now provide safe, reference wrappers for the `sys` types `jarray` and `jbyteArray` etc with a lifetime like `JObject` ([#400](https://github.com/jni-rs/jni-rs/pull/400))
- `JObjectArray` provides a reference wrapper for a `jobjectArray` with a lifetime like `JObject`. ([#400](https://github.com/jni-rs/jni-rs/pull/400))
- `AutoElements` and `AutoElementsCritical` (previously `AutoArray`/`AutoPrimitiveArray`) implement `Deref<Target=[T]>` and `DerefMut` so array elements can be accessed via slices without needing additional `unsafe` code. ([#400](https://github.com/jni-rs/jni-rs/pull/400))
- `AsJArrayRaw` trait which enables `JNIEnv::get_array_length()` to work with `JPrimitiveArray` or `JObjectArray` types ([#400](https://github.com/jni-rs/jni-rs/pull/400))
- `InitArgsBuilder` now has `try_option` and `option_encoded` methods. ([#414](https://github.com/jni-rs/jni-rs/pull/414))

### Changed
- `JNIEnv::get_string` checks that the given object is a `java.lang.String` instance to avoid undefined behaviour from the JNI implementation potentially aborting the program. ([#328](https://github.com/jni-rs/jni-rs/issues/328))
- `JNIEnv::call_*method_unchecked` was marked `unsafe`, as passing improper argument types, or a bad number of arguments, can cause a JVM crash. ([#385](https://github.com/jni-rs/jni-rs/issues/385))
- The `JNIEnv::new_object_unchecked` function now takes arguments as `&[jni::sys::jvalue]` to avoid allocating, putting it inline with changes to `JniEnv::call_*_unchecked` from 0.20.0 ([#382](https://github.com/jni-rs/jni-rs/pull/382))
- The `get_superclass` function now returns an Option instead of a null pointer if the class has no superclass ([#151](https://github.com/jni-rs/jni-rs/issues/151))
- The `invocation` feature now locates the JVM implementation dynamically at runtime (via the `java-locator` crate by default) instead of linking with the JVM at build time ([#293](https://github.com/jni-rs/jni-rs/pull/293))
- Most `JNIEnv` methods now require `&mut self`. This improves safety by preventing `JObject`s from getting an invalid lifetime. Most native method implementations (that is, `#[no_mangle] extern "system" fn`s) must now make the `JNIEnv` parameter `mut`. See the example on the crate documentation. ([#392](https://github.com/jni-rs/jni-rs/issues/392))
- `JByteBuffer`, `JClass`, `JNIEnv`, `JObject`, `JString`, and `JThrowable` no longer have the `Clone` or `Copy` traits. This improves safety by preventing object references from being used after the JVM deletes them. Most functions that take one of these types as a parameter (except `extern fn`s that are directly called by the JVM) should now borrow it instead, e.g. `&JObject` instead of `JObject`. ([#392](https://github.com/jni-rs/jni-rs/issues/392))
- `AutoLocal` is now generic in the type of object reference (`JString`, etc). ([#392](https://github.com/jni-rs/jni-rs/issues/392))
- The closure passed to `JNIEnv::with_local_frame` must now take a `&mut JNIEnv` parameter, which has a different lifetime. This improves safety by preventing local references from escaping the closure, which would cause a use-after-free bug. `Executor::with_attached` and `Executor::with_attached_capacity` have been similarly changed. ([#392](https://github.com/jni-rs/jni-rs/issues/392))
- The closure passed to `JNIEnv::with_local_frame` can now return a generic `Result<T, E>` so long as the error implements `From<jni::errors::Error>` ([#399](https://github.com/jni-rs/jni-rs/issues/399))
- `JNIEnv::with_local_frame` now returns the same type that the given closure returns ([#399](https://github.com/jni-rs/jni-rs/issues/399))
- `JNIEnv::with_local_frame` no longer supports returning a local reference directly to the calling scope (see `with_local_frame_returning_local`) ([#399](https://github.com/jni-rs/jni-rs/issues/399))
- `Executor::with_attached` and `Executor::with_attached_capacity` have been changed in the same way as `JNIEnv::with_local_frame` (they are thin wrappers) ([#399](https://github.com/jni-rs/jni-rs/issues/399))
- `Desc`, `JNIEnv::pop_local_frame`, and `TypeArray` are now `unsafe`. ([#392](https://github.com/jni-rs/jni-rs/issues/392))
- The `Desc` trait now has an associated type `Output`. Many implementations now return `AutoLocal`, so if you call `Desc::lookup` yourself and then call `as_raw` on the returned object, make sure the `AutoLocal` isn't dropped too soon (see the `Desc::lookup` documentation for examples). ([#392](https://github.com/jni-rs/jni-rs/issues/392))
- The `Desc<JClass>` trait is no longer implemented for `JObject` or `&JObject`. The previous implementation that called `.get_object_class()` was surprising and a simpler cast would make it easy to mistakenly pass instances where a class is required. ([#118](https://github.com/jni-rs/jni-rs/issues/118))
- Named lifetimes in the documentation have more descriptive names (like `'local` instead of `'a`). The new naming convention is explained in the `JNIEnv` documentation. ([#392](https://github.com/jni-rs/jni-rs/issues/392))
- Object reference types (`JObject`, `JClass`, `AutoLocal`, `GlobalRef`, etc) now implement `AsRef<JObject>` and `Deref<Target = JObject>`. Typed wrappers like `JClass` also implement `Into<JObject>`, but `GlobalRef` does not. ([#392](https://github.com/jni-rs/jni-rs/issues/392))
- Most `JList` and `JMap` methods now require a `&mut JNIEnv` parameter. `JListIter` and `JMapIter` no longer implement `Iterator`, and instead have a `next` method that requires a `&mut JNIEnv` parameter (use `while let` loops instead of `for`). ([#392](https://github.com/jni-rs/jni-rs/issues/392))
- `JValue` has been changed in several ways: ([#392](https://github.com/jni-rs/jni-rs/issues/392))
    - It is now a generic type named `JValueGen`. `JValue` is now a type alias for `JValueGen<&JObject>`, that is, it borrows an object reference. `JValueOwned` is a type alias for `JValueGen<JObject>`, that is, it owns an object reference.
    - `JValueOwned` does not have the `Copy` trait.
    - The `to_jni` method is now named `as_jni`, and it borrows the `JValueGen` instead of taking ownership.
    - `JObject` can no longer be converted directly to `JValue`, which was commonly done when calling Java methods or constructors. Instead of `obj.into()`, use `(&obj).into()`.
- All `JNIEnv` array APIs now work in terms of `JPrimitiveArray` and `JObjectArray` (reference wrappers with a lifetime) instead of `sys` types like `jarray` and `jbyteArray` ([#400](https://github.com/jni-rs/jni-rs/pull/400))
- `AutoArray` and `AutoPrimitiveArray` have been renamed `AutoElements` and `AutoElementsCritical` to show their connection and differentiate from new `JPrimitiveArray` API ([#400](https://github.com/jni-rs/jni-rs/pull/400))
- `get_primitive_array_critical` is now `unsafe` and has been renamed to `get_array_elements_critical` (consistent with the rename of `AutoPrimitiveArray`) with more detailed safety documentation ([#400](https://github.com/jni-rs/jni-rs/pull/400))
- `get_array_elements` is now also `unsafe` (for many of the same reasons as `get_array_elements_critical`) and has detailed safety documentation ([#400](https://github.com/jni-rs/jni-rs/pull/400))
- `AutoArray/AutoArrayCritical::size()` has been replaced with `.len()` which can't fail and returns a `usize` ([#400](https://github.com/jni-rs/jni-rs/pull/400))
- The `TypeArray` trait is now a private / sealed trait, that is considered to be an implementation detail for the `AutoArray` API.
- `JvmError` has several more variants and is now `non_exhaustive`. ([#414](https://github.com/jni-rs/jni-rs/pull/414))
- `InitArgsBuilder::option` raises an error on Windows if the string is too long. The limit is currently 1048576 bytes. ([#414](https://github.com/jni-rs/jni-rs/pull/414))

### Fixed
- Trying to use an object reference after it has been deleted now causes a compile error instead of undefined behavior. As a result, it is now safe to use `AutoLocal`, `JNIEnv::delete_local_ref`, and `JNIEnv::with_local_frame`. (Most of the limitations added in #392, listed above, were needed to make this work.) ([#381](https://github.com/jni-rs/jni-rs/issues/381), [#392](https://github.com/jni-rs/jni-rs/issues/392))
- Class lookups via the `Desc` trait now return `AutoLocal`s, which prevents them from leaking. ([#109](https://github.com/jni-rs/jni-rs/issues/109), [#392](https://github.com/jni-rs/jni-rs/issues/392))
- `InitArgsBuilder::option` properly encodes non-ASCII characters on Windows. ([#414](https://github.com/jni-rs/jni-rs/pull/414))

### Removed
- `get_string_utf_chars` and `release_string_utf_chars` from `JNIEnv` (See `JavaStr::into_raw()` and `JavaStr::from_raw()` instead) ([#372](https://github.com/jni-rs/jni-rs/pull/372))
- All `JNIEnv::get_<type>_array_elements()` methods have been removed as redundant since they would all be equivalent to `get_array_elements()` with the introduction of `JPrimitiveArray` ([#400](https://github.com/jni-rs/jni-rs/pull/400))

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

[Unreleased]: https://github.com/jni-rs/jni-rs/compare/v0.21.1...HEAD
[0.21.1]: https://github.com/jni-rs/jni-rs/compare/v0.21.0...v0.21.1
[0.21.0]: https://github.com/jni-rs/jni-rs/compare/v0.20.0...v0.21.0
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
