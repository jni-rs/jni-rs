package com.example;

/**
 * Another test class that depends on Foo.
 */
public class Bar {
    private Foo foo;
    private int count;

    public Bar() {
        this.foo = new Foo("Hello from Bar");
        this.count = 0;
    }

    public Bar(Foo foo, int count) {
        this.foo = foo;
        this.count = count;
    }

    public Foo getFoo() {
        return foo;
    }

    public int getCount() {
        return count;
    }

    public void increment() {
        count++;
    }

    public String getGreeting(String name) {
        return foo.greet(name) + " (count: " + count + ")";
    }
}
