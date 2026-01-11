package com.example;

/**
 * Test class with array types for testing bind_java_type array bindings.
 */
public class TestArrays {
    // Static array fields
    public static int[] staticIntArray = { 1, 2, 3, 4, 5 };
    public static String[] staticStringArray = { "hello", "world" };
    public static int[][] staticInt2DArray = { { 1, 2 }, { 3, 4 } };

    // Instance array fields
    public int[] intArray;
    public long[] longArray;
    public String[] stringArray;
    public int[][] int2DArray;
    public String[][] string2DArray;

    // Constructors
    public TestArrays() {
        this.intArray = new int[] { 10, 20, 30 };
        this.longArray = new long[] { 100L, 200L, 300L };
        this.stringArray = new String[] { "foo", "bar", "baz" };
        this.int2DArray = new int[][] { { 1, 2 }, { 3, 4 }, { 5, 6 } };
        this.string2DArray = new String[][] { { "a", "b" }, { "c", "d" } };
    }

    public TestArrays(int[] intArray, String[] stringArray) {
        this.intArray = intArray;
        this.longArray = new long[] { 0L };
        this.stringArray = stringArray;
        this.int2DArray = new int[][] { { 0 } };
        this.string2DArray = new String[][] { { "" } };
    }

    // Static methods with array parameters
    public static int sumArray(int[] values) {
        int sum = 0;
        for (int value : values) {
            sum += value;
        }
        return sum;
    }

    public static String[] concatenateArrays(String[] arr1, String[] arr2) {
        String[] result = new String[arr1.length + arr2.length];
        System.arraycopy(arr1, 0, result, 0, arr1.length);
        System.arraycopy(arr2, 0, result, arr1.length, arr2.length);
        return result;
    }

    public static int[][] transpose(int[][] matrix) {
        if (matrix.length == 0)
            return new int[0][0];
        int rows = matrix.length;
        int cols = matrix[0].length;
        int[][] result = new int[cols][rows];
        for (int i = 0; i < rows; i++) {
            for (int j = 0; j < cols; j++) {
                result[j][i] = matrix[i][j];
            }
        }
        return result;
    }

    // Static methods that return arrays
    public static int[] createIntArray(int size, int value) {
        int[] result = new int[size];
        for (int i = 0; i < size; i++) {
            result[i] = value;
        }
        return result;
    }

    public static String[] createStringArray(int size, String prefix) {
        String[] result = new String[size];
        for (int i = 0; i < size; i++) {
            result[i] = prefix + i;
        }
        return result;
    }

    // Instance methods with array parameters
    public void updateIntArray(int[] values) {
        this.intArray = values;
    }

    public void updateStringArray(String[] values) {
        this.stringArray = values;
    }

    public void update2DArray(int[][] values) {
        this.int2DArray = values;
    }

    // Instance methods that return arrays
    public int[] getIntArray() {
        return intArray;
    }

    public String[] getStringArray() {
        return stringArray;
    }

    public int[][] get2DArray() {
        return int2DArray;
    }

    // Instance methods that process arrays
    public int[] doubleValues(int[] input) {
        int[] result = new int[input.length];
        for (int i = 0; i < input.length; i++) {
            result[i] = input[i] * 2;
        }
        return result;
    }

    public String[] toUpperCase(String[] input) {
        String[] result = new String[input.length];
        for (int i = 0; i < input.length; i++) {
            result[i] = input[i].toUpperCase();
        }
        return result;
    }

    // Helper methods
    public int getIntArrayLength() {
        return intArray != null ? intArray.length : 0;
    }

    public int getStringArrayLength() {
        return stringArray != null ? stringArray.length : 0;
    }
}
