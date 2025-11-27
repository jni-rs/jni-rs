package com.example;

public class TestNativeMethods {
    // Instance fields to test state mutation
    private int counter = 0;
    private String message = "initial";

    // Note: in terms of argument + return types, there's very little difference
    // between instance and static native methods, we don't need to test every
    // combination of both so we just split our tests evenly. We want to cover:
    // - void args/return
    // - primitive args/return
    // - String (Reference type) args/return
    // - Array args/return
    // - Multi-dimensional Array args/return

    // Instance native methods
    public native int nativeAdd(int a, int b);

    public native void nativeLog(String message);

    public native int[] nativeArrayAdd(int[] arr, int value);

    public native void nativeSetCounter(int value);

    public native void nativeSetMessage(String msg);

    public native String nativeGetMessage();

    // Static native methods
    public static native int nativeGetVersion();

    public static native boolean[][] native2DArrayInvert(boolean[][] arr);

    public static native String[] nativeStringArrayEcho(String[] arr);

    public static native String[][] native2DStringArrayEcho(String[][] arr);

    // Non-native methods that can be used to test native method calls
    public int getCounter() {
        return counter;
    }

    public void setCounter(int value) {
        counter = value;
    }

    public String getMessage() {
        return message;
    }

    public void setMessage(String msg) {
        message = msg;
    }

    // Wrapper methods that call native methods (for testing)
    public int callNativeAdd(int a, int b) {
        return nativeAdd(a, b);
    }

    public void callNativeLog(String msg) {
        nativeLog(msg);
    }

    public int[] callNativeArrayAdd(int[] arr, int value) {
        return nativeArrayAdd(arr, value);
    }

    public void callNativeSetCounter(int value) {
        nativeSetCounter(value);
    }

    public void callNativeSetMessage(String msg) {
        nativeSetMessage(msg);
    }

    public String callNativeGetMessage() {
        return nativeGetMessage();
    }

    // Wrapper methods for static native methods
    public static int callNativeGetVersion() {
        return nativeGetVersion();
    }

    public static boolean[][] callNative2DArrayInvert(boolean[][] arr) {
        return native2DArrayInvert(arr);
    }

    public static String[] callNativeStringArrayEcho(String[] arr) {
        return nativeStringArrayEcho(arr);
    }

    public static String[][] callNative2DStringArrayEcho(String[][] arr) {
        return native2DStringArrayEcho(arr);
    }
}
