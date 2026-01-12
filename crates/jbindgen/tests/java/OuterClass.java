package com.example;

/**
 * A test class with inner classes for verifying inner class binding generation.
 */
public class OuterClass {
    private int value;

    /**
     * Creates a new OuterClass instance.
     */
    public OuterClass() {
        this.value = 0;
    }

    /**
     * Gets the value.
     */
    public int getValue() {
        return value;
    }

    /**
     * An inner class within OuterClass.
     */
    public static class InnerClass {
        private String name;

        /**
         * Creates a new InnerClass instance.
         */
        public InnerClass() {
            this.name = "default";
        }

        /**
         * Creates a new InnerClass with the specified name.
         */
        public InnerClass(String name) {
            this.name = name;
        }

        /**
         * Gets the name.
         */
        public String getName() {
            return name;
        }

        /**
         * Sets the name.
         */
        public void setName(String name) {
            this.name = name;
        }
    }

    /**
     * Another inner class for testing multiple inner classes.
     */
    public static class AnotherInner {
        /**
         * A static utility method.
         */
        public static int calculate(int x, int y) {
            return x + y;
        }
    }
}
