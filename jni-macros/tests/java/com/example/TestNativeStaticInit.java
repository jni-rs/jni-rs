package com.example;

public class TestNativeStaticInit {
    // Static field that will be initialized by the native method
    private static int staticValue;
    private static String staticMessage;

    // Static initializer that calls a native method
    static {
        System.out.println("Static initializer starting...");
        staticValue = nativeInitializeStatic();
        staticMessage = nativeGetStaticMessage();
        System.out.println("Static initializer completed. Value: " + staticValue + ", Message: " + staticMessage);
    }

    // Native methods called during static initialization
    private static native int nativeInitializeStatic();

    private static native String nativeGetStaticMessage();

    // Public methods to access the static fields
    public static int getStaticValue() {
        return staticValue;
    }

    public static String getStaticMessage() {
        return staticMessage;
    }

    // Constructor
    public TestNativeStaticInit() {
        System.out.println("Constructor called");
    }

    // Instance method
    public String getMessage() {
        return "Instance message, static value was: " + staticValue;
    }
}
