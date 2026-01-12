package com.example;

/**
 * Test class with both public and private native methods for testing visibility
 * handling.
 */
public class NativeMethodVisibility {
    // Public static native method
    public static native int publicStaticNative(int a, int b);

    // Public instance native method
    public native String publicInstanceNative();

    // Private static native method
    private static native int privateStaticNative(int x);

    // Private instance native method
    private native void privateInstanceNative(String data);

    // Regular public method for comparison
    public int regularMethod(int x) {
        return privateStaticNative(x) * 2;
    }

    // Static initializer
    static {
        // System.loadLibrary("nativemethodvisibility");
    }
}
