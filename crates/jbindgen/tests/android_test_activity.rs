//! Android Activity binding test

use adb_client::ADBDeviceExt;
use adb_client::server::ADBServer;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Get the Android SDK home directory
fn get_android_home() -> Option<PathBuf> {
    env::var("ANDROID_HOME")
        .or_else(|_| env::var("ANDROID_SDK_ROOT"))
        .ok()
        .map(PathBuf::from)
}

/// Get the test-activity project directory
fn get_test_activity_project() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("android-test-activity")
}

/// Build the test-activity project for Android
fn build_android_test_activity(project_dir: &PathBuf, features: &str) -> bool {
    // Check if cargo-ndk is available
    let cargo_ndk_check = Command::new("cargo").arg("install").arg("--list").output();

    let has_cargo_ndk = cargo_ndk_check
        .map(|output| String::from_utf8_lossy(&output.stdout).contains("cargo-ndk"))
        .unwrap_or(false);

    if !has_cargo_ndk {
        println!("cargo-ndk not found.");
        println!("To build for Android, install: cargo install cargo-ndk");
        return false;
    }

    // Determine target based on environment variables
    // If ANDROID_TEST_TARGET is set, use that
    // Otherwise, if ANDROID_TEST_SERIAL is set, assume physical device (aarch64)
    // Otherwise, use emulator target (x86_64)
    let target = if let Ok(target) = env::var("ANDROID_TEST_TARGET") {
        println!("Using target from ANDROID_TEST_TARGET: {}", target);
        target
    } else if env::var("ANDROID_TEST_SERIAL").is_ok() {
        println!("ANDROID_TEST_SERIAL set, using aarch64-linux-android for physical device");
        "aarch64-linux-android".to_string()
    } else {
        println!("Using x86_64-linux-android for emulator");
        "x86_64-linux-android".to_string()
    };

    println!("Building Rust library for Android target: {}", target);

    let targets = vec![target];

    for rust_target in &targets {
        print!("  Building for {}... ", rust_target);
        let mut cmd = Command::new("cargo");
        cmd.arg("ndk")
            .arg("--platform")
            .arg("35")
            .arg("-o")
            .arg("app/src/main/jniLibs/")
            .arg("build")
            .arg("--target")
            .arg(rust_target)
            .arg("--features")
            .arg(features)
            .current_dir(project_dir);

        println!("Executing: {:?}", cmd);
        let output = cmd.output();

        match output {
            Ok(output) => {
                if !output.status.success() {
                    eprintln!("Build failed for {}:", rust_target);
                    eprintln!("{}", String::from_utf8_lossy(&output.stderr));
                    return false;
                }
            }
            Err(e) => {
                eprintln!("Failed to run cargo ndk for {}: {}", rust_target, e);
                return false;
            }
        }
    }

    println!("All architectures built successfully");
    true
}

/// Check the android-test-activity project compiles (without building for Android)
fn check_android_test_activity_compiles(project_dir: &PathBuf) -> bool {
    println!("Checking Rust code compiles...");
    let mut cmd = Command::new("cargo");
    cmd.arg("check")
        .arg("--target")
        .arg("aarch64-linux-android")
        .current_dir(project_dir);
    println!("Executing: {:?}", cmd);
    let output = cmd.output().expect("Failed to run cargo check");

    if !output.status.success() {
        eprintln!("Cargo check failed:");
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        false
    } else {
        println!("Rust code compiles");
        true
    }
}

/// Find an available Android emulator
fn find_emulator(android_home: &Path) -> Option<String> {
    let emulator_tool = android_home.join("emulator/emulator");

    let output = Command::new(&emulator_tool).arg("-list-avds").output();

    if let Ok(output) = output {
        let avds = String::from_utf8_lossy(&output.stdout);
        avds.lines()
            .next()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    } else {
        None
    }
}

/// Build the Android APK using gradle
fn build_apk_with_gradle(project_dir: &PathBuf) -> Result<PathBuf, String> {
    println!("Building APK with gradle...");

    let gradlew = if cfg!(windows) {
        project_dir.join("gradlew.bat")
    } else {
        project_dir.join("gradlew")
    };

    if !gradlew.exists() {
        return Err(format!("gradlew not found at {}", gradlew.display()));
    }

    let output = Command::new(&gradlew)
        .arg("assembleDebug")
        .current_dir(project_dir)
        .output()
        .map_err(|e| format!("Failed to run gradlew: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Gradle build failed:\n{}", stderr));
    }

    let apk_path = project_dir.join("app/build/outputs/apk/debug/app-debug.apk");
    if !apk_path.exists() {
        return Err(format!("APK not found at {}", apk_path.display()));
    }

    println!("APK built: {}", apk_path.display());
    Ok(apk_path)
}

/// Start an Android emulator in the background
fn start_emulator(android_home: &Path, emulator_name: &str) -> Result<(), String> {
    println!("Starting emulator '{}'...", emulator_name);

    let emulator_tool = android_home.join("emulator/emulator");

    Command::new(&emulator_tool)
        .arg("-avd")
        .arg(emulator_name)
        .arg("-no-snapshot-load")
        .arg("-no-window")
        .spawn()
        .map_err(|e| format!("Failed to start emulator: {}", e))?;

    println!("Emulator starting in background");
    Ok(())
}

/// Get the serial number of the most recently started emulator
fn get_emulator_serial() -> Option<String> {
    let mut server = ADBServer::default();
    if let Ok(devices) = server.devices() {
        // Find the first device that starts with "emulator-"
        devices
            .iter()
            .find(|d| d.identifier.starts_with("emulator-"))
            .map(|d| d.identifier.clone())
    } else {
        None
    }
}

/// Get the serial number of the first non-emulator device (physical device)
fn get_physical_device_serial() -> Option<String> {
    let mut server = ADBServer::default();
    if let Ok(devices) = server.devices() {
        // Find the first device that does NOT start with "emulator-"
        devices
            .iter()
            .find(|d| !d.identifier.starts_with("emulator-"))
            .map(|d| d.identifier.clone())
    } else {
        None
    }
}

/// Wait for the emulator to boot and be ready, returning its serial number
fn wait_for_emulator_boot(timeout_secs: u64) -> Result<String, String> {
    println!(
        "Waiting for emulator to boot (timeout: {}s)...",
        timeout_secs
    );

    unsafe { std::env::remove_var("TERM") };
    let mut server = ADBServer::default();
    let start = std::time::Instant::now();
    let mut emulator_serial: Option<String> = None;

    loop {
        if start.elapsed().as_secs() > timeout_secs {
            return Err("Emulator boot timeout".to_string());
        }

        // Check if any emulator device is connected
        if let Ok(devices) = server.devices() {
            // Find emulator device
            let emulator = devices
                .iter()
                .find(|d| d.identifier.starts_with("emulator-"));

            if let Some(emu) = emulator {
                let serial = &emu.identifier;
                emulator_serial = Some(serial.clone());

                // Try to get the specific device and check boot completion
                if let Ok(mut device) = server.get_device_by_name(serial) {
                    println!(
                        "Probing emulator {:?} for boot completion...",
                        device.identifier
                    );
                    let mut output = Vec::new();
                    let cmd = "getprop sys.boot_completed";
                    match device.shell_command(&cmd, Some(&mut output), None) {
                        Ok(_) => {
                            let result = String::from_utf8_lossy(&output);
                            if result.trim() == "1" {
                                println!("\nEmulator is ready");
                                return emulator_serial
                                    .ok_or_else(|| "Emulator serial not found".to_string());
                            } else {
                                println!(
                                    "Emulator not booted yet (sys.boot_completed={:?})",
                                    result.trim().as_bytes()
                                );
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to run shell command: {}", e);
                        }
                    }
                }
            }
        }

        thread::sleep(Duration::from_secs(2));
        print!(".");
        use std::io::Write;
        std::io::stdout().flush().unwrap();
    }
}

/// Install APK on the device using adb_client
fn install_apk(apk_path: &PathBuf, device_serial: &str) -> Result<(), String> {
    println!(
        "Installing APK on device {}: {}",
        device_serial,
        apk_path.display()
    );

    let mut server = ADBServer::default();
    let mut device = server
        .get_device_by_name(device_serial)
        .map_err(|e| format!("Failed to get device {}: {}", device_serial, e))?;

    device
        .install(apk_path)
        .map_err(|e| format!("Failed to install APK: {}", e))?;

    println!("APK installed successfully");
    Ok(())
}

/// Launch the TestActivity on the device
fn launch_test_activity(device_serial: &str) -> Result<(), String> {
    println!("Launching TestActivity on device {}...", device_serial);

    let mut server = ADBServer::default();
    let mut device = server
        .get_device_by_name(device_serial)
        .map_err(|e| format!("Failed to get device {}: {}", device_serial, e))?;

    let mut output = Vec::new();
    let cmd = "am start -n com.github.jni.jbindgen.testactivity/.TestActivity";
    device
        .shell_command(&cmd, Some(&mut output), None)
        .map_err(|e| format!("Failed to launch activity: {}", e))?;

    let result = String::from_utf8_lossy(&output);
    if result.contains("Error") {
        return Err(format!("Activity launch failed: {}", result));
    }

    println!("Activity launched");
    println!("Output: {}", result);
    Ok(())
}

/// Check if an emulator is already running
fn is_emulator_running() -> bool {
    let mut server = ADBServer::default();
    if let Ok(devices) = server.devices() {
        devices
            .iter()
            .any(|d| d.identifier.starts_with("emulator-"))
    } else {
        false
    }
}

/// Get current Unix epoch time from the device
fn get_device_epoch_time(device_serial: &str) -> Result<u64, String> {
    let mut server = ADBServer::default();
    let mut device = server
        .get_device_by_name(device_serial)
        .map_err(|e| format!("Failed to get device {}: {}", device_serial, e))?;

    let mut output = Vec::new();
    device
        .shell_command(&"date +%s", Some(&mut output), None)
        .map_err(|e| format!("Failed to get device time: {}", e))?;

    let time_str = String::from_utf8_lossy(&output);
    time_str
        .trim()
        .parse::<u64>()
        .map_err(|e| format!("Failed to parse device time '{}': {}", time_str, e))
}

/// Stream logcat from device and monitor for test logs
fn stream_logcat(device_serial: &str, start_epoch: u64) -> Result<Vec<String>, String> {
    let mut server = ADBServer::default();
    let mut device = server
        .get_device_by_name(device_serial)
        .map_err(|e| format!("Failed to get device {}: {}", device_serial, e))?;

    // Run logcat with time filter
    let logcat_cmd = format!("logcat -T {}.0 TestActivity:* *:E", start_epoch);
    println!("Running: {}", logcat_cmd);

    // Custom writer that captures output line by line
    struct LogCapture {
        logs: Vec<String>,
        buffer: Vec<u8>,
        test_complete: bool,
        timeout: std::time::Instant,
        max_duration: Duration,
    }

    impl std::io::Write for LogCapture {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            // Check for timeout
            if self.timeout.elapsed() > self.max_duration {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "Log streaming timeout",
                ));
            }

            // Check if test is complete
            if self.test_complete {
                // Return an error to stop the shell command from continuing
                return Err(std::io::Error::other("Test complete"));
            }

            self.buffer.extend_from_slice(buf);

            // Process complete lines
            while let Some(newline_pos) = self.buffer.iter().position(|&b| b == b'\n') {
                let line_bytes = self.buffer.drain(..=newline_pos).collect::<Vec<_>>();
                if let Ok(line) = String::from_utf8(line_bytes) {
                    print!("{}", line);

                    // Always add the line to logs first
                    self.logs.push(line.clone());

                    if line.contains("TEST_ACTIVITY_TEST_COMPLETE") {
                        println!("\nTest completion marker found!");
                        self.test_complete = true;
                        // Return error to immediately stop shell_command
                        return Err(std::io::Error::other("Test complete"));
                    }
                }
            }

            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let mut log_capture = LogCapture {
        logs: Vec::new(),
        buffer: Vec::new(),
        test_complete: false,
        timeout: std::time::Instant::now(),
        max_duration: Duration::from_secs(30),
    };

    // Run logcat - this will block until timeout or test complete
    let _result = device.shell_command(&logcat_cmd, Some(&mut log_capture), None);

    if !log_capture.test_complete {
        println!("\n⚠ Test completion marker not found within timeout");
    }

    Ok(log_capture.logs)
}

/// Terminate the emulator
fn terminate_emulator(device_serial: &str) {
    println!("\n--- Terminating emulator {} ---", device_serial);

    let mut server = ADBServer::default();
    if let Ok(mut device) = server.get_device_by_name(device_serial) {
        let mut output = Vec::new();
        if device
            .shell_command(&"reboot -p", Some(&mut output), None)
            .is_ok()
        {
            println!("Emulator shutdown command sent");
        } else {
            println!("Failed to send shutdown command, emulator may still be running");
        }
    }

    // Give it a moment to shutdown
    thread::sleep(Duration::from_secs(2));
}

/// Tests the AndroidTestActivity binding generation using the existing android-test-activity project
#[test]
#[ignore]
fn test_android_test_activity_build() {
    let android_home = match get_android_home() {
        Some(home) => home,
        None => {
            println!("Skipping test: ANDROID_HOME not set");
            return;
        }
    };

    println!("\n=== Android Test Activity Binding Test ===");
    println!("ANDROID_HOME: {}", android_home.display());

    let project_dir = get_test_activity_project();
    println!("Project: {}", project_dir.display());

    assert!(
        project_dir.exists(),
        "android-test-activity project not found"
    );

    if !check_android_test_activity_compiles(&project_dir) {
        panic!("Rust code does not compile");
    }
}

/// Test that demonstrates the complete Android workflow with the existing project
fn run_test_on_emulator(features: &str) {
    let android_home = match get_android_home() {
        Some(home) => home,
        None => {
            println!("Skipping test: ANDROID_HOME not set");
            return;
        }
    };

    println!("\n=== Android Test Activity Emulator Test (features = {features}) ===");
    println!("ANDROID_HOME: {}", android_home.display());

    let project_dir = get_test_activity_project();
    println!("Project: {}", project_dir.display());

    assert!(
        project_dir.exists(),
        "android-test-activity project not found"
    );

    println!("\n--- Building for Android ---");
    if !build_android_test_activity(&project_dir, features) {
        panic!(
            "Failed to build Android test activity. Ensure cargo-ndk is installed and the build succeeds."
        );
    }

    println!("\n--- Checking for Android emulator ---");
    // Skip emulator check if using ANDROID_TEST_SERIAL or targeting physical device
    let emulator_name = if env::var("ANDROID_TEST_SERIAL").is_ok()
        || env::var("ANDROID_TEST_TARGET").as_deref() == Ok("aarch64-linux-android")
    {
        println!("Skipping emulator check (using ANDROID_TEST_SERIAL or physical device target)");
        String::new() // Not needed when using serial directly or targeting physical device
    } else {
        match find_emulator(&android_home) {
            Some(name) => {
                println!("Found emulator: {}", name);
                name
            }
            None => {
                panic!(
                    "No Android emulator found. Create one with: avdmanager create avd -n test_avd -k 'system-images;android-30;default;x86_64'"
                );
            }
        }
    };

    // Build APK with gradle
    println!("\n--- Building APK with gradle ---");
    let apk_path = match build_apk_with_gradle(&project_dir) {
        Ok(path) => path,
        Err(e) => {
            panic!("Gradle build failed: {}", e);
        }
    };

    // Check if ANDROID_TEST_SERIAL is set
    let (device_serial, should_terminate_emulator) = if let Ok(serial) =
        env::var("ANDROID_TEST_SERIAL")
    {
        println!("\n--- Using device from ANDROID_TEST_SERIAL ---");
        println!("Device serial: {}", serial);
        (serial, false) // Don't terminate device we didn't start
    } else if env::var("ANDROID_TEST_TARGET").as_deref() == Ok("aarch64-linux-android") {
        // If target is aarch64 but no serial is set, try to find a physical device
        println!("\n--- Looking for physical device (aarch64 target) ---");
        match get_physical_device_serial() {
            Some(serial) => {
                println!("Found physical device: {}", serial);
                (serial, false) // Don't terminate device we didn't start
            }
            None => {
                panic!(
                    "No physical device found. Set ANDROID_TEST_SERIAL to specify a device, or connect a physical device"
                );
            }
        }
    } else {
        // Start emulator if not already running
        println!("\n--- Starting emulator ---");
        let emulator_already_running = is_emulator_running();
        let serial = if emulator_already_running {
            println!("Emulator already running");
            // Get the serial of the already running emulator
            match get_emulator_serial() {
                Some(serial) => {
                    println!("Using existing emulator: {}", serial);
                    serial
                }
                None => {
                    panic!("Could not find emulator serial");
                }
            }
        } else {
            if let Err(e) = start_emulator(&android_home, &emulator_name) {
                panic!(
                    "Failed to start emulator: {}. Try starting manually: emulator -avd {}",
                    e, emulator_name
                );
            }

            // Wait for emulator to boot (5 minute timeout) and get its serial
            match wait_for_emulator_boot(300) {
                Ok(serial) => {
                    println!("Emulator serial: {}", serial);
                    serial
                }
                Err(e) => {
                    panic!("{}", e);
                }
            }
        };
        (serial, !emulator_already_running)
    };

    // Install APK on emulator
    println!("\n--- Installing APK on emulator ---");
    if let Err(e) = install_apk(&apk_path, &device_serial) {
        println!("Failed to install APK: {}", e);
        if should_terminate_emulator {
            terminate_emulator(&device_serial);
        }
        return;
    }

    // Get device time before launching activity (for logcat filtering)
    println!("\n--- Getting device time for logcat filtering ---");
    let start_epoch = match get_device_epoch_time(&device_serial) {
        Ok(time) => {
            println!("Device epoch time: {}", time);
            time
        }
        Err(e) => {
            println!("Failed to get device time: {}", e);
            if should_terminate_emulator {
                terminate_emulator(&device_serial);
            }
            return;
        }
    };

    // Start log streaming in a separate thread
    println!("\n--- Starting log stream ---");
    let (log_tx, log_rx) = mpsc::channel();
    let serial_for_logging = device_serial.clone();
    let log_thread = thread::spawn(
        move || match stream_logcat(&serial_for_logging, start_epoch) {
            Ok(logs) => {
                let _ = log_tx.send(Ok(logs));
            }
            Err(e) => {
                let _ = log_tx.send(Err(e));
            }
        },
    );

    // Give log streaming a moment to start
    thread::sleep(Duration::from_millis(500));

    // Launch TestActivity
    println!("\n--- Launching TestActivity ---");
    if let Err(e) = launch_test_activity(&device_serial) {
        println!("Failed to launch activity: {}", e);
        if should_terminate_emulator {
            terminate_emulator(&device_serial);
        }
        return;
    }

    // Wait for logs to complete
    println!("\n--- Monitoring logs ---");
    let logs = match log_rx.recv_timeout(Duration::from_secs(35)) {
        Ok(Ok(logs)) => logs,
        Ok(Err(e)) => {
            println!("Log streaming error: {}", e);
            if should_terminate_emulator {
                terminate_emulator(&device_serial);
            }
            return;
        }
        Err(_) => {
            println!("Log streaming timeout");
            if should_terminate_emulator {
                terminate_emulator(&device_serial);
            }
            return;
        }
    };

    // Wait for log thread to finish
    let _ = log_thread.join();

    // Print summary of captured logs
    println!("\n--- Log Summary ---");
    println!("Captured {} log lines", logs.len());

    let rust_activity_logs: Vec<_> = logs
        .iter()
        .filter(|log| log.contains("TestActivity"))
        .collect();

    println!("TestActivity logs: {}", rust_activity_logs.len());

    // Verify we got the expected logs
    let has_oncreate = rust_activity_logs
        .iter()
        .any(|log| log.contains("onCreate called"));
    let has_native_message = rust_activity_logs
        .iter()
        .any(|log| log.contains("Native message"));
    let has_native_result = rust_activity_logs
        .iter()
        .any(|log| log.contains("Native result"));
    let has_update_ui = rust_activity_logs
        .iter()
        .any(|log| log.contains("updateUi called"));
    let has_completion = rust_activity_logs
        .iter()
        .any(|log| log.contains("TEST_ACTIVITY_TEST_COMPLETE"));

    // Terminate emulator if we started it (not if using ANDROID_TEST_SERIAL)
    if should_terminate_emulator {
        terminate_emulator(&device_serial);
    }

    assert!(has_oncreate, "Missing onCreate log");
    assert!(has_native_message, "Missing native message log");
    assert!(has_native_result, "Missing native result log");
    assert!(has_update_ui, "Missing updateUi log");
    assert!(has_completion, "Missing test completion marker");
}

#[test]
#[ignore]
fn test_on_emulator_basic() {
    run_test_on_emulator("");
}

#[test]
#[ignore]
fn test_on_emulator_sdk() {
    run_test_on_emulator(
        "sdk_util_time_utils,sdk_os_build,sdk_os_binder,sdk_bluetooth,sdk_content_intent,sdk_net_uri",
    );
}
