//! Windows-specific Fiber Local Storage (FLS) implementation for thread attachment tracking.
//!
//! This module provides a Windows-specific alternative to thread-local storage that uses Fiber
//! Local Storage (FLS) callbacks. The key advantage is that FLS callbacks run without holding the
//! Windows loader lock, avoiding potential deadlocks.
//!
//! For example see: https://github.com/jni-rs/jni-rs/issues/701
//!
//! On Windows, this sequence of events can trigger a deadlock:
//!
//! 1. A native thread, previously attached to a JVM, is stopped
//! 2. Windows kernel calls the TLS destructor - This happens while [holding Windows Loader
//!    Lock](https://doc.rust-lang.org/stable/std/thread/struct.LocalKey.html#synchronization-in-thread-local-destructors)
//! 3. jni-rs `InternalAttachGuard` calls `DetachCurrentThread`. This involves a thread state
//!    transition from Native to VM
//! 4. Concurrently, there is a new Java thread being started by some other thread
//! 5. During the Native -> VM transition, the native thread is trapped in a JVM safepoint. While
//!    holding the loader lock!
//! 6. The Java thread created at step no. 4 is already runnable from JVM's point of view, and the
//!    JVM expects this thread to arrive at the safepoint. But Windows won't execute user code on
//!    this thread until it manages to acquire the loader lock. Which is held by the Rust thread
//!    that is trapped at safepoint. This means the Java thread cannot make any progress and
//!    certainly cannot reach the safepoint. JVM won't release the native thread until all threads
//!    are at safepoint -> Deadlock
//!

use std::{
    ptr,
    sync::{atomic::Ordering, OnceLock},
    thread::{current, Thread},
};

use log::{debug, error};

use crate::{
    errors::*,
    sys,
    vm::java_vm::{sys_detach_current_thread, ATTACHED_THREADS},
};

use windows_sys::Win32::{
    Foundation::FALSE,
    System::Threading::{FlsAlloc, FlsGetValue, FlsSetValue},
};

/// The FLS index used to store the attachment guard data.
/// This is allocated once when the first thread attaches.
static FLS_INDEX: OnceLock<u32> = OnceLock::new();

/// Data stored in FLS for each attached thread.
#[derive(Debug)]
struct FlsAttachData {
    env: *mut sys::JNIEnv,
    thread: Thread,
}

/// FLS callback that runs when a thread terminates.
///
/// SAFETY: This callback is invoked by Windows when:
/// 1. A thread terminates (via ExitThread or thread return)
/// 2. The FLS slot is freed (via FlsFree)
unsafe extern "system" fn fls_callback(data: *const core::ffi::c_void) {
    if data.is_null() {
        return;
    }
    // Safety: We only ever store Box<FlsAttachData> in FLS
    let attach_data = unsafe { Box::from_raw(data as *mut FlsAttachData) };

    // Detach the thread from the JVM
    if let Err(e) = unsafe { sys_detach_current_thread(attach_data.env, &attach_data.thread) } {
        error!(
            "Error detaching thread in FLS callback: {:#?}\nThread {} id={:?}",
            e,
            attach_data.thread.name().unwrap_or_default(),
            attach_data.thread.id(),
        );
    }
}

/// Initialize FLS if not already initialized.
///
/// Returns the FLS index, allocating it if necessary.
fn get_or_init_fls_index() -> Result<u32> {
    // Try to get existing index first
    if let Some(&index) = FLS_INDEX.get() {
        return Ok(index);
    }

    // Need to initialize - allocate a new FLS index with our callback
    // Safety: fls_callback is a valid callback function
    let new_index = unsafe { FlsAlloc(Some(fls_callback)) };

    if new_index == u32::MAX {
        // FLS_OUT_OF_INDEXES
        return Err(Error::JniCall(JniError::Unknown));
    }

    // Try to store it, or use the value another thread stored
    Ok(*FLS_INDEX.get_or_init(|| new_index))
}

/// Attach the current thread using FLS for cleanup tracking.
///
/// # Safety
///
/// The `env` pointer must be a valid JNIEnv pointer for the current thread.
pub(super) unsafe fn fls_attach_current_thread(env: *mut sys::JNIEnv) -> Result<()> {
    let fls_index = get_or_init_fls_index()?;

    // Check if already attached via FLS
    // Safety: fls_index is valid
    let existing = unsafe { FlsGetValue(fls_index) };
    if !existing.is_null() {
        // Already attached
        return Ok(());
    }

    // Create attachment data
    let attach_data = Box::new(FlsAttachData {
        env,
        thread: current(),
    });

    let data_ptr = Box::into_raw(attach_data) as *mut core::ffi::c_void;

    // Store in FLS
    // Safety: fls_index is valid, data_ptr is a valid pointer
    let result = unsafe { FlsSetValue(fls_index, data_ptr) };

    if result == FALSE {
        // Failed to set FLS value, clean up
        unsafe {
            let _ = Box::from_raw(data_ptr as *mut FlsAttachData);
        }
        return Err(Error::JniCall(JniError::Unknown));
    }

    debug!(
        "Attached thread via FLS: {} ({:?}). {} threads attached",
        current().name().unwrap_or_default(),
        current().id(),
        ATTACHED_THREADS.load(Ordering::SeqCst)
    );

    Ok(())
}

/// Explicitly detach the current thread if attached via FLS.
///
/// This clears the FLS slot without invoking the callback, since we're
/// detaching manually.
///
/// Returns Ok(true) if a thread was detached, Ok(false) if not attached.
pub(super) fn fls_detach_current_thread() -> Result<bool> {
    let Some(&fls_index) = FLS_INDEX.get() else {
        return Ok(false); // FLS never initialized, so not attached
    };

    // Get the current FLS value
    // Safety: fls_index is valid
    let data_ptr = unsafe { FlsGetValue(fls_index) };    if data_ptr.is_null() {
        // Not attached via FLS
        return Ok(false);
    }

    // Clear the FLS slot first (before detaching) to prevent the callback from running
    // Safety: fls_index is valid
    let result = unsafe { FlsSetValue(fls_index, ptr::null_mut()) };

    if result == FALSE {
        return Err(Error::JniCall(JniError::Unknown));
    }

    // Now manually detach and clean up
    // Safety: data_ptr came from FlsGetValue and we stored Box<FlsAttachData>
    let attach_data = unsafe { Box::from_raw(data_ptr as *mut FlsAttachData) };

    unsafe { sys_detach_current_thread(attach_data.env, &attach_data.thread)? };

    Ok(true)
}
