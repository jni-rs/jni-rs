package com.example;

/**
 * Test class that depends on other types to verify type_map optimization.
 * This class should only include Calculator and SimpleClass in its type_map,
 * not any other unrelated types.
 */
public class WithDependencies {
    private Calculator calculator;
    private SimpleClass simpleClass;

    public WithDependencies() {
    }

    public Calculator getCalculator() {
        return calculator;
    }

    public void setCalculator(Calculator calc) {
        this.calculator = calc;
    }

    public SimpleClass getSimpleClass() {
        return simpleClass;
    }

    public void setSimpleClass(SimpleClass simple) {
        this.simpleClass = simple;
    }

    public Calculator createCalculator() {
        return new Calculator();
    }

    public SimpleClass createSimpleClass(int value) {
        return new SimpleClass(value);
    }

    // This method uses a String, but String is a built-in JNI type,
    // so it should NOT appear in the type_map
    public String getMessage() {
        return "Dependencies test";
    }

    // Primitives should also not appear in type_map
    public int getValue() {
        return 42;
    }
}
