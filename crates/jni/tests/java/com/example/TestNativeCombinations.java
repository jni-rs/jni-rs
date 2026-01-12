package com.example;

public class TestNativeCombinations {
    public TestNativeCombinations() {
    }

    public native int methodNonExported(int value);

    public native int methodNonExported2(int value);

    public static native int staticMethodNonExported(int value);

    public static native int staticMethodNonExported2(int value);

    // Numbered methods that can be exported without clashing symbol names...
    public native int method1(int value);

    public native int method2(int value);

    public native int method3(int value);

    public native int method4(int value);

    public static native int staticMethod1(int value);

    public static native int staticMethod2(int value);

    public static native int staticMethod3(int value);
}
