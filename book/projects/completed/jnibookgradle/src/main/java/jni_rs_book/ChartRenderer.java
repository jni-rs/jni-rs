// ANCHOR: complete
package jni_rs_book;

public class ChartRenderer {

    private ChartRenderer() {}

    public static String renderChart(long chartWidth, long chartHeight,
                                     String xTitle, double xStart, double xEnd, double[] xs,
                                     String yTitle, double yStart, double yEnd, double[] ys) {
        if (xs.length != ys.length) {
            throw new IllegalArgumentException("xs and ys array lengths must match");
        }
        return NativeAPI.draw_points_plaintext(chartWidth, chartHeight, xs.length,
                xTitle, xStart, xEnd, xs,
                yTitle, yStart, yEnd, ys);
    }
}
// ANCHOR_END: complete
