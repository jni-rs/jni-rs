package com.github.jni.jbindgen.testactivity;

import android.app.Activity;
import android.graphics.Color;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.util.Log;
import android.util.TypedValue;
import android.view.Gravity;
import android.view.ViewGroup;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;

/**
 * Custom Activity subclass with native methods for JNI binding demonstration.
 * This demonstrates how to bind a custom Activity that will be implemented in
 * Rust.
 */
public class TestActivity extends Activity {
    private static final String TAG = "TestActivity";
    private Handler uiHandler;
    private LinearLayout contentLayout;
    private TextView deviceInfoView;
    private TextView currentTestView;
    private int testCount = 0;

    // Native library name
    static {
        System.loadLibrary("testactivity");
    }

    // Native method called from onCreate
    public native void nativeOnCreate(Bundle savedInstanceState);

    // Native method to get message from Rust
    public native String nativeGetMessage();

    // Native method to process data
    public native int nativeProcessData(int value);

    // Native method to run next test in sequence (returns empty string when done)
    public native String nativeRunNextTest();

    // Regular Java method that can be called from Rust to update device info
    public void updateDeviceInfo(String info) {
        Log.d(TAG, "Device info: " + info);
        if (deviceInfoView != null) {
            deviceInfoView.setText(info);
        }
    }

    // Regular Java method that can be called from Rust
    public void updateUi(String message) {
        Log.d(TAG, "updateUi called with: " + message);
        addTestResult("Initial Message", message, true);
    }

    // Regular Java method
    public String getActivityName() {
        return "TestActivity";
    }

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        Log.d(TAG, "onCreate called");

        // Initialize UI handler for sequential testing
        uiHandler = new Handler(Looper.getMainLooper());

        // Setup UI with proper margins
        setupUi();

        // Call native onCreate implementation
        nativeOnCreate(savedInstanceState);

        // Demonstrate calling native methods
        String message = nativeGetMessage();
        Log.d(TAG, "Native message: " + message);

        int result = nativeProcessData(42);
        Log.d(TAG, "Native result: " + result);

        // Update UI with message from Rust
        updateUi(message);

        // Start sequential SDK feature testing
        startSdkFeatureTests();
    }

    private void setupUi() {
        // Create ScrollView to handle content overflow
        ScrollView scrollView = new ScrollView(this);
        scrollView.setFillViewport(true);

        // Create main layout with padding to avoid insets
        contentLayout = new LinearLayout(this);
        contentLayout.setOrientation(LinearLayout.VERTICAL);
        contentLayout.setBackgroundColor(Color.WHITE);

        // Add padding to avoid notches and system bars (in dp)
        int paddingDp = 16;
        float scale = getResources().getDisplayMetrics().density;
        int paddingPx = (int) (paddingDp * scale + 0.5f);
        contentLayout.setPadding(paddingPx, paddingPx * 3, paddingPx, paddingPx * 2);

        // Title
        TextView titleView = new TextView(this);
        titleView.setText("JNI TestActivity");
        titleView.setTextSize(TypedValue.COMPLEX_UNIT_SP, 24);
        titleView.setTextColor(Color.BLACK);
        titleView.setPadding(0, 0, 0, paddingPx);
        contentLayout.addView(titleView);

        // Device info section
        deviceInfoView = new TextView(this);
        deviceInfoView.setText("Loading device info...");
        deviceInfoView.setTextSize(TypedValue.COMPLEX_UNIT_SP, 12);
        deviceInfoView.setTextColor(Color.DKGRAY);
        deviceInfoView.setPadding(0, 0, 0, paddingPx * 2);
        contentLayout.addView(deviceInfoView);

        // Current test section
        currentTestView = new TextView(this);
        currentTestView.setText("Initializing tests...");
        currentTestView.setTextSize(TypedValue.COMPLEX_UNIT_SP, 14);
        currentTestView.setTextColor(Color.BLUE);
        currentTestView.setPadding(0, 0, 0, paddingPx);
        contentLayout.addView(currentTestView);

        scrollView.addView(contentLayout);
        setContentView(scrollView);
    }

    private void addTestResult(String testName, String result, boolean success) {
        TextView resultView = new TextView(this);

        String statusIcon = success ? "✓" : "✗";
        int statusColor = success ? Color.rgb(0, 128, 0) : Color.rgb(200, 0, 0);

        resultView.setText(String.format("%s %s: %s", statusIcon, testName, result));
        resultView.setTextSize(TypedValue.COMPLEX_UNIT_SP, 13);
        resultView.setTextColor(statusColor);

        float scale = getResources().getDisplayMetrics().density;
        int paddingPx = (int) (8 * scale + 0.5f);
        resultView.setPadding(paddingPx, paddingPx / 2, paddingPx, paddingPx / 2);

        contentLayout.addView(resultView);
    }

    private void startSdkFeatureTests() {
        Log.d(TAG, "Starting SDK feature tests");
        runNextSdkFeatureTest();
    }

    private void runNextSdkFeatureTest() {
        // Post test to UI looper to avoid blocking onCreate
        uiHandler.post(new Runnable() {
            @Override
            public void run() {
                try {
                    testCount++;
                    currentTestView.setText(String.format("Running test %d...", testCount));

                    String result = nativeRunNextTest();

                    // Empty string signals no more tests
                    if (result == null || result.isEmpty()) {
                        currentTestView.setText("All tests complete!");
                        currentTestView.setTextColor(Color.rgb(0, 128, 0));
                        Log.i(TAG, "TEST_ACTIVITY_TEST_COMPLETE");
                        return;
                    }

                    Log.d(TAG, "SDK test result: " + result);

                    // Parse test result to extract test name and result
                    // Format is typically: "[index] testname: result"
                    boolean success = !result.toLowerCase().contains("error") &&
                            !result.toLowerCase().contains("failed");

                    // Extract test name from result
                    String testName = result;
                    if (result.contains(":")) {
                        String[] parts = result.split(":", 2);
                        testName = parts[0].trim();
                        if (parts.length > 1) {
                            result = parts[1].trim();
                        }
                    }

                    addTestResult(testName, result, success);

                    // Schedule next test with delay to allow UI to process
                    uiHandler.postDelayed(new Runnable() {
                        @Override
                        public void run() {
                            runNextSdkFeatureTest();
                        }
                    }, 100); // 100ms delay between tests
                } catch (Exception e) {
                    Log.e(TAG, "Error running SDK test", e);
                    addTestResult("Test " + testCount, "ERROR: " + e.getMessage(), false);
                    currentTestView.setText("Tests failed with error");
                    currentTestView.setTextColor(Color.RED);
                    Log.i(TAG, "TEST_ACTIVITY_TEST_COMPLETE");
                }
            }
        });
    }
}