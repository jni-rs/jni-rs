// ANCHOR: complete
package jni_rs_book;

import org.junit.Test;

public class ChartRendererTest {

    @Test
    public void testRenderChart() {
        String s = ChartRenderer.renderChart(80, 50, "Spaceship Number",
                0, 5,
                new double[]{1.0, 2.0, 3.0, 4.0, 5.0},
                "Launch Time in Hours",
                0, 5,
                new double[]{3.0, 2.0, 5.0, 4.0, 1.0});
        System.out.println(s);
    }
}
// ANCHOR: complete
