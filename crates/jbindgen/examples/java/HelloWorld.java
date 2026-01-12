package com.example;

/**
 * Example class for demonstrating jbindgen.
 */
public class HelloWorld {
    private String message;
    private int count;

    public HelloWorld() {
        this.message = "Hello, World!";
        this.count = 0;
    }

    public HelloWorld(String message) {
        this.message = message;
        this.count = 0;
    }

    public String getMessage() {
        return message;
    }

    public void setMessage(String message) {
        this.message = message;
    }

    public int getCount() {
        return count;
    }

    public void increment() {
        count++;
    }

    public static String greet(String name) {
        return "Hello, " + name + "!";
    }

    public static int add(int a, int b) {
        return a + b;
    }

    public String formatMessage() {
        return message + " (count: " + count + ")";
    }
}
