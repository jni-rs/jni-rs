// ANCHOR: complete
// ANCHOR: happy_path
package jni_rs_book;

import org.junit.Test;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertThrows;

public class DivisionTest {

    @Test
    public void testDivision() {
        assertEquals(NativeAPI.divide(10, 5), 2);
    }
// ANCHOR_END: happy_path

// ANCHOR: divide_by_zero
    @Test
    public void testDivisionByZero() {
        assertThrows(RuntimeException.class, () -> NativeAPI.divide(10, 0));
    }
// ANCHOR_END: divide_by_zero
}
// ANCHOR_END: complete