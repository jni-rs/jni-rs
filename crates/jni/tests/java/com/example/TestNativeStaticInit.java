package com.example;

public class TestNativeStaticInit {
    // Static field that will be incremented to 15 when the initializer runs
    private static int staticValue = 5;

    // Static initializer that calls a native method
    static {
        staticValue += 10;
        System.out.println("Static initializer starting...");
        staticValue = nativeInitializeStatic(staticValue);
        System.out.println("Static initializer completed. Value: " + staticValue);
    }

    // Native methods called during static initialization
    private static native int nativeInitializeStatic(int counter);

    public static int getStaticValue() {
        return staticValue;
    }
}
