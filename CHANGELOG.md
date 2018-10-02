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

- Fixed the issue with early detaching of a thread by nested AttachGuard.

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

[Unreleased]: https://github.com/jni-rs/jni-rs/compare/v0.10.2...HEAD
[0.10.2]: https://github.com/jni-rs/compare/v0.10.1...v0.10.2
[0.10.1]: https://github.com/jni-rs/compare/v0.1...v0.10.1
