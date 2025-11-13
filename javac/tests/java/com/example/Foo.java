package com.example;

/**
 * A simple test class for javac compilation.
 */
public class Foo {
    private String message;

    public Foo() {
        this.message = "Hello from Foo";
    }

    public Foo(String message) {
        this.message = message;
    }

    public String getMessage() {
        return message;
    }

    public void setMessage(String message) {
        this.message = message;
    }

    public String greet(String name) {
        return message + ", " + name + "!";
    }
}
