package com.example;

/**
 * Test class with native methods for jbindgen.
 */
public class WithNative {
    // Static native method
    public static native int nativeAdd(int a, int b);

    // Instance native method
    public native String nativeGetMessage();

    // Instance native method with parameters
    public native void nativeProcess(String input, double value);

    // Static native method returning String
    public static native String nativeGetPlatform();

    // Regular Java method for comparison
    public int regularMethod(int x) {
        return x * 2;
    }

    // Load native library
    static {
        // System.loadLibrary("withnative");
    }
}
