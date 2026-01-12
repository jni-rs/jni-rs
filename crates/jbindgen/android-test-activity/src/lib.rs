//! TestActivity JNI implementation
use std::sync::atomic::{AtomicUsize, Ordering};
use tracing::{error, info};
use tracing_subscriber::prelude::*;

pub mod sdk;

// Include the generated bindings from build.rs
include!(concat!(env!("OUT_DIR"), "/test_activity_bindings.rs"));

use com::github::jni::jbindgen::testactivity::{
    TestActivity, TestActivityAPI, TestActivityNativeInterface,
};

const DEFAULT_ENV_FILTER: &str = "debug";

// Canonical test array - the single source of truth for all SDK tests
struct SdkTest {
    name: &'static str,
    runner: for<'local> fn(
        &mut jni::Env<'local>,
        TestActivity<'local>,
    ) -> Result<String, jni::errors::Error>,
}

const SDK_TESTS: &[SdkTest] = &[
    #[cfg(feature = "sdk_util_time_utils")]
    SdkTest {
        name: "sdk_util_time_utils",
        runner: sdk::util_time_utils::test_time_utils,
    },
    #[cfg(feature = "sdk_os_build")]
    SdkTest {
        name: "sdk_os_build",
        runner: sdk::os_build::test_os_build,
    },
    #[cfg(feature = "sdk_os_binder")]
    SdkTest {
        name: "sdk_os_binder",
        runner: sdk::os_binder::test_os_binder,
    },
    #[cfg(feature = "sdk_bluetooth")]
    SdkTest {
        name: "sdk_bluetooth",
        runner: sdk::bluetooth::test_bluetooth,
    },
    #[cfg(feature = "sdk_content_intent")]
    SdkTest {
        name: "sdk_content_intent",
        runner: sdk::content_intent::test_content_intent,
    },
    #[cfg(feature = "sdk_net_uri")]
    SdkTest {
        name: "sdk_net_uri",
        runner: sdk::net_uri::test_net_uri,
    },
];

// Track current test index
static CURRENT_TEST_INDEX: AtomicUsize = AtomicUsize::new(0);

// Implement the TestActivityNativeInterface trait
impl TestActivityNativeInterface for TestActivityAPI {
    type Error = jni::errors::Error;

    fn native_on_create<'local>(
        env: &mut jni::Env<'local>,
        this: TestActivity<'local>,
        _saved_instance_state: jni::objects::JObject<'local>,
    ) -> Result<(), Self::Error> {
        static INIT: std::sync::Once = std::sync::Once::new();
        INIT.call_once(|| {
            unsafe { std::env::set_var("RUST_BACKTRACE", "full") };
            // Simple logging without paranoid_android for now
            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::DEBUG)
                .init();
        });

        // Example: Call the updateUI method from Rust
        let message = jni::objects::JString::from_str(env, "Hello from Rust native onCreate!")?;
        this.update_ui(env, message)?;

        Ok(())
    }

    fn native_get_message<'local>(
        env: &mut jni::Env<'local>,
        _this: TestActivity<'local>,
    ) -> Result<jni::objects::JString<'local>, Self::Error> {
        jni::objects::JString::from_str(env, "Message from Rust!")
    }

    fn native_process_data<'local>(
        _env: &mut jni::Env<'local>,
        _this: TestActivity<'local>,
        value: jni::sys::jint,
    ) -> Result<jni::sys::jint, Self::Error> {
        // Simple processing: multiply by 2
        Ok(value * 2)
    }

    fn native_run_next_test<'local>(
        env: &mut jni::Env<'local>,
        this: TestActivity<'local>,
    ) -> Result<jni::objects::JString<'local>, Self::Error> {
        let index = CURRENT_TEST_INDEX.fetch_add(1, Ordering::SeqCst);

        if index >= SDK_TESTS.len() {
            // No more tests - return empty string to signal completion
            return jni::objects::JString::from_str(env, "");
        }

        let test = &SDK_TESTS[index];
        info!("Running SDK test [{}]: {}", index, test.name);

        match (test.runner)(env, this) {
            Ok(result) => {
                let formatted = format!("[{}] {}: {}", index, test.name, result);
                jni::objects::JString::from_str(env, &formatted)
            }
            Err(e) => {
                error!("Test {} failed: {:?}", test.name, e);
                let error_msg = format!("[{}] {}: ERROR - {:?}", index, test.name, e);
                jni::objects::JString::from_str(env, &error_msg)
            }
        }
    }
}
