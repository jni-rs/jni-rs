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

    public int callMethodNonExported(int value) {
        return methodNonExported(value);
    }

    public int callMethodNonExported2(int value) {
        return methodNonExported2(value);
    }

    public static int callStaticMethodNonExported(int value) {
        return staticMethodNonExported(value);
    }

    public static int callStaticMethodNonExported2(int value) {
        return staticMethodNonExported2(value);
    }

    public int callMethod1(int value) {
        return method1(value);
    }

    public int callMethod2(int value) {
        return method2(value);
    }

    public int callMethod3(int value) {
        return method3(value);
    }

    public int callMethod4(int value) {
        return method4(value);
    }

    public static int callStaticMethod1(int value) {
        return staticMethod1(value);
    }

    public static int callStaticMethod2(int value) {
        return staticMethod2(value);
    }

    public static int callStaticMethod3(int value) {
        return staticMethod3(value);
    }
}
