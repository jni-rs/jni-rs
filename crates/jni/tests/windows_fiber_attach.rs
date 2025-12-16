//! Windows-specific test for fiber-aware JNI attachment.
//!
//! This test verifies that the FLS-based attachment mechanism works correctly
//! on Windows, including proper tracking when:
//! - There are multiple redundant attach_current_thread() calls across multiple
//!   fibers on the same OS thread.
//! - detach_current_thread() is used in fibers with no FLS attachment.
//! - detach_current_thread() is used in fibers with an FLS attachment.
//!
//! No attempt is made to test misuse of fibers that would violate safety
//! invariants (e.g., switching or deleting fibers while AttachGuards are
//! active).

#![cfg(all(use_fls_attach_guard, feature = "invocation"))]

mod util;

use jni::{JavaVM, errors::Result};
use std::ptr;
use util::jvm;
use windows_sys::Win32::System::Threading::ConvertThreadToFiber;
use windows_sys::Win32::System::Threading::{CreateFiber, DeleteFiber, SwitchToFiber};

use rusty_fork::rusty_fork_test;

use crate::util::sys_detach_current_thread;

rusty_fork_test! {
#[test]
fn test_attached_threads_counter_with_actual_fibers() {
    let jvm = jvm();

    // Detach the initial thread attachment
    assert_eq!(jvm.threads_attached(), 1);
    jvm.detach_current_thread().unwrap();
    let initial_count = jvm.threads_attached();
    assert_eq!(initial_count, 0);

    eprintln!("Converting main thread to fiber and creating fibers");
    unsafe {
        // Convert the current thread to a fiber (main fiber)
        let main_fiber = ConvertThreadToFiber(ptr::null_mut());
        assert!(!main_fiber.is_null(), "Failed to convert thread to fiber");

        // Track fiber execution
        static mut FIBER1_EXECUTED: bool = false;
        static mut FIBER1_PART2_EXECUTED: bool = false;
        static mut FIBER2_EXECUTED: bool = false;
        static mut FIBER3_EXECUTED: bool = false;
        static mut FIBER4_EXECUTED: bool = false;
        static mut MAIN_FIBER: *mut std::ffi::c_void = ptr::null_mut();

        MAIN_FIBER = main_fiber;

        // Fiber callbacks - each attaches to JVM using TLS attachment (which persists)
        unsafe extern "system" fn fiber1_proc(param: *mut std::ffi::c_void) {
            let jvm = unsafe { &*(param as *const std::sync::Arc<JavaVM>) };
            eprintln!("Running fiber 1, part 1");

            // FLS attachment - stays attached after callback returns
            jvm.attach_current_thread(|env| -> Result<()> {
                let _string = env.new_string("Fiber 1")?;
                unsafe { FIBER1_EXECUTED = true; }
                Ok(())
            })
            .expect("Fiber 1 failed to attach");

            // The thread should be permanently attached now with an FLS allocation
            assert!(jvm.is_thread_attached().unwrap());
            assert_eq!(jvm.threads_attached(), 1);

            // Manually detach the thread to test re-attachment in fiber 2
            unsafe { sys_detach_current_thread(); }

            unsafe { SwitchToFiber(MAIN_FIBER); }

            //panic!("DEBUG: Back in fiber 1 part 1 after switching to main fiber");
            eprintln!("Running fiber 1, part 2");
            // At this point, all the other fibers have been deleted and we should be left with
            // a count of 1 from the FLS attachment we got earlier
            assert_eq!(jvm.threads_attached(), 1);
            assert!(!jvm.is_thread_attached().unwrap());

            // Re-attach the thread/fiber again (without incrementing the attached count) - stays attached after callback returns
            jvm.attach_current_thread(|env| -> Result<()> {
                let _string = env.new_string("Fiber 1, part 2")?;
                unsafe { FIBER1_PART2_EXECUTED = true; }
                Ok(())
            })
            .expect("Fiber 1 failed to attach");

            assert!(jvm.is_thread_attached().unwrap());
            assert_eq!(jvm.threads_attached(), 1);

            unsafe { SwitchToFiber(MAIN_FIBER); }
        }

        unsafe extern "system" fn fiber2_proc(param: *mut std::ffi::c_void) {
            let jvm = unsafe { &*(param as *const std::sync::Arc<JavaVM>) };
            eprintln!("Running fiber 2");

            // Ensure the thread is detached before running this fiber
            assert!(!jvm.is_thread_attached().unwrap());

            // New FLS attachment (due to manual detach) - stays attached after callback returns
            jvm.attach_current_thread(|env| -> Result<()> {
                let _string = env.new_string("Fiber 2")?;
                unsafe { FIBER2_EXECUTED = true; }
                Ok(())
            })
            .expect("Fiber 2 failed to attach");

            // Since we manually detach the thread before running this fiber,
            // this fiber will re-attach the thread now fiber1 and fiber2 will
            // have FLS allocations that will also try to detach the thread when
            // they are deleted.
            assert!(jvm.is_thread_attached().unwrap());
            assert_eq!(jvm.threads_attached(), 2);

            // This should succeed because this fiber does have its own FLS allocation
            // This should set the FLS slot to null and detach the thread such that
            // when the fiber is deleted later, the FLS callback is a no-op.
            jvm.detach_current_thread().expect("Fiber 2 failed to call detach_current_thread");

            // Note: threads_attached() at this point is 1 because fiber1 still
            // has its FLS allocation although the thread itself is technically
            // detached now. This is a corner case for the Windows FLS
            // attachment model where this number is more like "number of FLS
            // attachments". The number should still remain balanced and return
            // to zero when all fibers are deleted.
            assert!(!jvm.is_thread_attached().unwrap());
            assert_eq!(jvm.threads_attached(), 1);

            unsafe { SwitchToFiber(MAIN_FIBER); }
        }

        unsafe extern "system" fn fiber3_proc(param: *mut std::ffi::c_void) {
            let jvm = unsafe { &*(param as *const std::sync::Arc<JavaVM>) };
            eprintln!("Running fiber 3");

            // Ensure the thread is detached before running this fiber
            assert!(!jvm.is_thread_attached().unwrap());

            // New FLS attachment (due to detach_current_thread()) - stays attached after callback returns
            jvm.attach_current_thread(|env| -> Result<()> {
                let _string = env.new_string("Fiber 3")?;
                unsafe { FIBER3_EXECUTED = true; }
                Ok(())
            })
            .expect("Fiber 3 failed to attach");

            // Since we detach the thread before running this fiber, this fiber
            // will re-attach the thread, so now fiber1, fiber2 and fiber3 will have
            // FLS allocations but the fiber2 data is null after calling
            //detach_current_thread()
            assert!(jvm.is_thread_attached().unwrap());
            assert_eq!(jvm.threads_attached(), 2);

            unsafe { SwitchToFiber(MAIN_FIBER); }
        }

        unsafe extern "system" fn fiber4_proc(param: *mut std::ffi::c_void) {
            let jvm = unsafe { &*(param as *const std::sync::Arc<JavaVM>) };
            eprintln!("Running fiber 4");

            // Use pre-existing attachment - stays attached after callback returns
            jvm.attach_current_thread(|env| -> Result<()> {
                let _string = env.new_string("Fiber 4")?;
                unsafe { FIBER4_EXECUTED = true; }
                Ok(())
            })
            .expect("Fiber 4 failed to attach");

            assert!(jvm.is_thread_attached().unwrap());
            // This fiber should see that something else has already attached the thread
            // and so no new attachment is created
            assert_eq!(jvm.threads_attached(), 2);

            // This should be a no-op, since the fiber is using the pre-existing attachment
            jvm.detach_current_thread().expect("Fiber 4 failed to call detach_current_thread");

            assert!(jvm.is_thread_attached().unwrap());
            assert_eq!(jvm.threads_attached(), 2);

            unsafe { SwitchToFiber(MAIN_FIBER); }
        }
        // Create three fibers on the same OS thread
        let jvm_ptr = jvm as *const _ as *mut std::ffi::c_void;

        eprintln!("Creating fibers");

        let fiber1 = CreateFiber(0, Some(fiber1_proc), jvm_ptr);
        assert!(!fiber1.is_null(), "Failed to create fiber 1");

        let fiber2 = CreateFiber(0, Some(fiber2_proc), jvm_ptr);
        assert!(!fiber2.is_null(), "Failed to create fiber 2");

        let fiber3 = CreateFiber(0, Some(fiber3_proc), jvm_ptr);
        assert!(!fiber3.is_null(), "Failed to create fiber 3");

        let fiber4 = CreateFiber(0, Some(fiber4_proc), jvm_ptr);
        assert!(!fiber4.is_null(), "Failed to create fiber 4");

        // Execute all three fibers on the same OS thread
        SwitchToFiber(fiber1);
        assert!(FIBER1_EXECUTED, "Fiber 1 did not execute");

        SwitchToFiber(fiber2);
        assert!(FIBER2_EXECUTED, "Fiber 2 did not execute");

        SwitchToFiber(fiber3);
        assert!(FIBER3_EXECUTED, "Fiber 3 did not execute");

        SwitchToFiber(fiber4);
        assert!(FIBER4_EXECUTED, "Fiber 4 did not execute");

        assert!(jvm.is_thread_attached().unwrap());
        assert_eq!(jvm.threads_attached(), 2);

        eprintln!("Deleting fiber 2");
        // Deleting fiber2 should be a no-op since it calls detach_current_thread() which
        // cleared its FLS slot
        DeleteFiber(fiber2);
        assert!(jvm.is_thread_attached().unwrap());
        assert_eq!(jvm.threads_attached(), 2);

        eprintln!("Deleting fiber 3");
        // Deleting fiber 3 should detach the thread and decrement the attached threads counter
        DeleteFiber(fiber3);
        assert!(!jvm.is_thread_attached().unwrap());
        assert_eq!(jvm.threads_attached(), 1);

        eprintln!("Deleting fiber 4");
        // Deleting fiber4 should (redundantly) detach the thread and decrement the counter
        DeleteFiber(fiber4);
        assert!(!jvm.is_thread_attached().unwrap());
        assert_eq!(jvm.threads_attached(), 1);

        eprintln!("Running second part of fiber 1");
        // Running the second part of fiber1 to verify it can re-attach without
        // incrementing the counter (since it already has an FLS allocation)
        SwitchToFiber(fiber1);
        assert!(FIBER1_PART2_EXECUTED, "Fiber 1 part 2 did not execute");

        assert!(jvm.is_thread_attached().unwrap());
        assert_eq!(jvm.threads_attached(), 1);

        //panic!("DEBUG: Before deleting fiber 1");
        eprintln!("Deleting fiber 1");
        DeleteFiber(fiber1);
        assert!(!jvm.is_thread_attached().unwrap());
        assert_eq!(jvm.threads_attached(), 0);
    }
}
}
