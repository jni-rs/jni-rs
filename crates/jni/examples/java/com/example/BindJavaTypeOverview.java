// Support class for examples/bind_java_type.rs

package com.example;

public class BindJavaTypeOverview {
    public int value;
    public String name;
    public CustomType custom;
    public OtherType other;

    public static int staticValue = 42;
    public static int CONSTANT_FIVE = 5;

    public static class OtherType {
        public OtherType() {
        }
    }

    public static class CustomType {
        public CustomType() {
        }
    }

    // Constructors
    public BindJavaTypeOverview() {
        this.value = 0;
        this.name = "default";
    }

    public BindJavaTypeOverview(int initialValue) {
        this.value = initialValue;
        this.name = "initialized";
    }

    // Methods with various signatures
    public void doNothing() {
    }

    public int addNumbers(int a, int b) {
        return a + b;
    }

    public String concatenate(String a, String b) {
        return a + b;
    }

    public String processHandle(long handle) {
        return nativeProcessHandle(handle);
    }

    public static int[] echoIntArray(int[] arr) {
        return arr;
    }

    public static int[][] echoInt2DArray(int[][] arr) {
        return arr;
    }

    public static String[] echoStringArray(String[] arr) {
        return arr;
    }

    public static String[][] echoString2DArray(String[][] arr) {
        return arr;
    }

    public static OtherType echoOtherType(OtherType other) {
        return other;
    }

    public static CustomType echoCustomType(CustomType custom) {
        return custom;
    }

    // method that will be wrapped
    public String internalGetInfo() {
        return "Value: " + value + ", Name: " + name;
    }

    // Fields
    public int getValue() {
        return value;
    }

    public void setValue(int value) {
        this.value = value;
    }

    public String getName() {
        return name;
    }

    public void setName(String name) {
        this.name = name;
    }

    public String getUserFriendlyName() {
        return name.toUpperCase();
    }

    public void setUserFriendlyName(String name) {
        this.name = name.toLowerCase();
    }

    // Native methods
    public native int nativeAdd(int a, int b);

    public static native String nativeGreet(String name);

    public native int nativeRaw(int value);

    public native int nativeWithFunction(int value);

    public native int nativeNotExported(int value);

    public native int nativeNoUnwind(int value);

    public native int nativeCustomErrorPolicy(int value);

    public native String nativeProcessHandle(long handle);

    // Test methods that call native methods from Java
    public String testAllNativeMethods() {
        StringBuilder result = new StringBuilder();

        // Test instance native method
        int sum = nativeAdd(10, 20);
        result.append("nativeAdd(10, 20) = ").append(sum).append("; ");

        // Test static native method
        String greeting = nativeGreet("World");
        result.append("nativeGreet(World) = ").append(greeting).append("; ");

        // Test raw native method
        int raw = nativeRaw(42);
        result.append("nativeRaw(42) = ").append(raw).append("; ");

        // Test function-based native method
        int func = nativeWithFunction(100);
        result.append("nativeWithFunction(100) = ").append(func).append("; ");

        // Test not-exported native method
        int notExported = nativeNotExported(7);
        result.append("nativeNotExported(7) = ").append(notExported).append("; ");

        // Test no-unwind native method
        int noUnwind = nativeNoUnwind(99);
        result.append("nativeNoUnwind(99) = ").append(noUnwind).append("; ");

        // Test custom error policy native method
        int customError = nativeCustomErrorPolicy(50);
        result.append("nativeCustomErrorPolicy(50) = ").append(customError);

        return result.toString();
    }
}
