package com.example;

/**
 * Test class with various method signatures for testing bind_java_type method
 * bindings.
 */
public class TestMethods {
    private String message;
    private int counter;

    // Constructors
    public TestMethods() {
        this.message = "default";
        this.counter = 0;
    }

    public TestMethods(String message) {
        this.message = message;
        this.counter = 0;
    }

    public TestMethods(String message, int counter) {
        this.message = message;
        this.counter = counter;
    }

    // Static methods
    public static String getStaticMessage() {
        return "static message";
    }

    public static int add(int a, int b) {
        return a + b;
    }

    public static long multiply(long a, long b) {
        return a * b;
    }

    public static String concat(String a, String b) {
        return a + b;
    }

    // Instance methods - getters
    public String getMessage() {
        return message;
    }

    public int getCounter() {
        return counter;
    }

    // Instance methods - setters
    public void setMessage(String message) {
        this.message = message;
    }

    public void setCounter(int counter) {
        this.counter = counter;
    }

    // Instance methods - operations
    public void increment() {
        counter++;
    }

    public void incrementBy(int amount) {
        counter += amount;
    }

    public String formatMessage(String prefix, String suffix) {
        return prefix + message + suffix;
    }

    // Method with boolean return
    public boolean isPositive() {
        return counter > 0;
    }

    // Method with multiple primitive types
    public double calculate(int a, long b, float c, double d) {
        return a + b + c + d;
    }

    // Method that returns an object
    public String toStringCustom() {
        return "TestMethods{message='" + message + "', counter=" + counter + "}";
    }

    // Void method with no parameters
    public void reset() {
        message = "default";
        counter = 0;
    }
}
