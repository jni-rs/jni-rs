package com.example;

public class TestNativeRaw {
    // Constructor
    public TestNativeRaw() {
    }

    // Native methods that will be implemented as raw function pointers
    public native int rawAdd(int a, int b);

    public native boolean rawIsPositive(int value);

    public native String rawProcessString(String input);

    // Static native methods with raw implementations
    public static native int rawMultiply(int a, int b);

    public static native boolean rawIsEven(int value);

    // Regular native method (trait-based)
    public native int regularMethod(int value);

    // Wrapper methods for testing
    public int callRawAdd(int a, int b) {
        return rawAdd(a, b);
    }

    public boolean callRawIsPositive(int value) {
        return rawIsPositive(value);
    }

    public String callRawProcessString(String input) {
        return rawProcessString(input);
    }

    public static int callRawMultiply(int a, int b) {
        return rawMultiply(a, b);
    }

    public static boolean callRawIsEven(int value) {
        return rawIsEven(value);
    }

    public int callRegularMethod(int value) {
        return regularMethod(value);
    }
}
