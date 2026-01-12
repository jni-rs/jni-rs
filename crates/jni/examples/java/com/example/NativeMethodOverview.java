// Support class for examples/native_method.rs for demonstrating how to use
// the native_method! macro
package com.example;

public class NativeMethodOverview {
    public int value;
    public String name;

    // Constructors
    public NativeMethodOverview() {
        this.value = 0;
        this.name = "default";
    }

    public NativeMethodOverview(int initialValue) {
        this.value = initialValue;
        this.name = "initialized";
    }

    // Native methods - instance methods
    public native int nativeAdd(int a, int b);

    public native String nativeConcatenate(String a, String b);

    public native int nativeRaw(int value);

    public native int nativeWithFunction(int value);

    public native int nativeNoUnwind(int value);

    public native int nativeCustomErrorPolicy(int value);

    public native String nativeProcessHandle(long handle);

    // Native methods - static methods
    public static native String nativeGreet(String name);

    public static native int[] nativeEchoIntArray(int[] arr);

    // Test method that calls all native methods from Java
    public String testAllNativeMethods() {
        StringBuilder result = new StringBuilder();

        // Test instance native method
        int sum = nativeAdd(10, 20);
        result.append("nativeAdd(10, 20) = ").append(sum).append("; ");

        // Test concatenate
        String concat = nativeConcatenate("Hello, ", "World!");
        result.append("nativeConcatenate = ").append(concat).append("; ");

        // Test static native method
        String greeting = nativeGreet("Rust");
        result.append("nativeGreet(Rust) = ").append(greeting).append("; ");

        // Test raw native method
        int raw = nativeRaw(42);
        result.append("nativeRaw(42) = ").append(raw).append("; ");

        // Test function-based native method
        int func = nativeWithFunction(100);
        result.append("nativeWithFunction(100) = ").append(func).append("; ");

        // Test no-unwind native method
        int noUnwind = nativeNoUnwind(99);
        result.append("nativeNoUnwind(99) = ").append(noUnwind).append("; ");

        // Test custom error policy native method
        int customError = nativeCustomErrorPolicy(50);
        result.append("nativeCustomErrorPolicy(50) = ").append(customError).append("; ");

        // Test array methods
        int[] intArray = { 1, 2, 3 };
        int[] echoedInts = nativeEchoIntArray(intArray);
        result.append("nativeEchoIntArray length = ").append(echoedInts.length);

        return result.toString();
    }
}
