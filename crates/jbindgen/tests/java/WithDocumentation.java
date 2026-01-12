package com.example;

/**
 * A test class for verifying Javadoc comment extraction.
 * This class demonstrates how documentation comments are preserved
 * in the generated Rust bindings.
 */
public class WithDocumentation {

    /**
     * A static constant field with documentation.
     */
    public static final int MAX_VALUE = 100;

    /**
     * An instance field representing the current value.
     */
    public int currentValue;

    /**
     * Creates a new instance with default value of zero.
     */
    public WithDocumentation() {
        this.currentValue = 0;
    }

    /**
     * Creates a new instance with the specified initial value.
     *
     * @param initialValue the initial value to set
     */
    public WithDocumentation(int initialValue) {
        this.currentValue = initialValue;
    }

    /**
     * Gets the current value.
     *
     * @return the current value
     */
    public int getValue() {
        return currentValue;
    }

    /**
     * Sets a new value.
     *
     * @param newValue the new value to set
     */
    public void setValue(int newValue) {
        this.currentValue = newValue;
    }

    /**
     * Adds two numbers together.
     *
     * @param a the first number
     * @param b the second number
     * @return the sum of a and b
     */
    public static int add(int a, int b) {
        return a + b;
    }
}
