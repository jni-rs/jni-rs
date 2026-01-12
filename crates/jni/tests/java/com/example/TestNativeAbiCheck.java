package com.example;

public class TestNativeAbiCheck {
    public TestNativeAbiCheck() {
    }

    public native int nativeMethod(int value);

    public static native int nativeStaticMethod(int value);

    public int callMethod(int value) {
        return nativeMethod(value);
    }

    public static int callStaticMethod(int value) {
        return nativeStaticMethod(value);
    }
}
