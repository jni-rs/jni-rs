package com.example;

public class TestNativeMethods {
    // Instance fields to test state mutation
    private int counter = 0;
    private String message = "initial";

    // Constructor
    public TestNativeMethods() {
    }

    // Instance native methods
    public native int nativeAdd(int a, int b);

    public native String nativeProcessString(String input);

    public native void nativeLog(String message);

    public native int nativeSumArray(int[] arr);

    public native void nativeSetCounter(int value);

    public native int nativeGetCounter();

    public native void nativeSetMessage(String msg);

    public native String nativeGetMessage();

    // Static native methods
    public static native int nativeGetVersion();

    public static native boolean nativeInitialize(String config, int flags);

    public static native String nativeConcatStatic(String a, String b);

    public static native long nativeMultiply(long a, long b);

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

    public String callNativeProcessString(String input) {
        return nativeProcessString(input);
    }

    public void callNativeLog(String msg) {
        nativeLog(msg);
    }

    public int callNativeSumArray(int[] arr) {
        return nativeSumArray(arr);
    }

    public void callNativeSetCounter(int value) {
        nativeSetCounter(value);
    }

    public int callNativeGetCounter() {
        return nativeGetCounter();
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

    public static boolean callNativeInitialize(String config, int flags) {
        return nativeInitialize(config, flags);
    }

    public static String callNativeConcatStatic(String a, String b) {
        return nativeConcatStatic(a, b);
    }

    public static long callNativeMultiply(long a, long b) {
        return nativeMultiply(a, b);
    }
}
