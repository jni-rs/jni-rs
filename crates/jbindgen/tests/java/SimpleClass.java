package com.example;

/**
 * Simple test class for jbindgen.
 * Contains basic constructors and methods.
 */
public class SimpleClass {
    private int value;

    public SimpleClass() {
        this.value = 0;
    }

    public SimpleClass(int initialValue) {
        this.value = initialValue;
    }

    public int getValue() {
        return value;
    }

    public void setValue(int value) {
        this.value = value;
    }

    public static int add(int a, int b) {
        return a + b;
    }

    public static String getMessage() {
        return "Hello from SimpleClass";
    }

    public static String concat(String a, String b) {
        return a + b;
    }
}
