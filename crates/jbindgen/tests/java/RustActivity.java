package com.example.rustactivity;

import android.app.Activity;
import android.os.Bundle;
import android.util.Log;
import android.widget.TextView;

/**
 * Custom Activity subclass with native methods for JNI binding demonstration.
 * This demonstrates how to bind a custom Activity that will be implemented in
 * Rust.
 */
public class RustActivity extends Activity {
    private static final String TAG = "RustActivity";

    // Native library name
    static {
        System.loadLibrary("rustactivity");
    }

    // Native method called from onCreate
    private native void nativeOnCreate(Bundle savedInstanceState);

    // Native method to get message from Rust
    public native String nativeGetMessage();

    // Native method to process data
    public native int nativeProcessData(int value);

    // Regular Java method that can be called from Rust
    public void updateUI(String message) {
        Log.d(TAG, "updateUI called with: " + message);
        TextView textView = new TextView(this);
        textView.setText(message);
        setContentView(textView);
    }

    // Regular Java method
    public String getActivityName() {
        return "RustActivity";
    }

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        Log.d(TAG, "onCreate called");

        // Call native onCreate implementation
        nativeOnCreate(savedInstanceState);

        // Demonstrate calling native methods
        String message = nativeGetMessage();
        Log.d(TAG, "Native message: " + message);

        int result = nativeProcessData(42);
        Log.d(TAG, "Native result: " + result);

        // Update UI with message from Rust
        updateUI(message);
    }
}
