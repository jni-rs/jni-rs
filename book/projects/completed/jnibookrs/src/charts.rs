// ANCHOR: render_chart
// Some starter code, for wrapping purposes later
use plotlib::page::Page;
use plotlib::repr::Plot;
use plotlib::style::{PointMarker, PointStyle};
use plotlib::view::ContinuousView;

/// Given various data, render a Chart as a String.
/// Upon failure, return Error.
fn render_chart(
    width: u32,
    height: u32,
    x_label: &str,
    y_label: &str,
    x_start: f64,
    x_end: f64,
    y_start: f64,
    y_end: f64,
    data: Vec<(f64, f64)>,
) -> Result<String, anyhow::Error> {
    let plot = Plot::new(data).point_style(PointStyle::new().marker(PointMarker::Circle));

    let v = ContinuousView::new()
        .add(plot)
        .x_range(x_start, x_end)
        .y_range(y_start, y_end)
        .x_label(x_label)
        .y_label(y_label);

    Ok(Page::single(&v)
        .dimensions(width, height)
        .to_text()
        .map_err(|err| err.compat())?)
}
// ANCHOR_END: render_chart

// ANCHOR: complete
use jni::sys::{jint, jdouble, jdoubleArray, jstring};
use jni::objects::{JClass, JString};
use crate::error::try_java;
use jni::JNIEnv;

#[no_mangle]
pub extern "system" fn Java_jni_1rs_1book_NativeAPI_draw_1points_1plaintext(
    env: JNIEnv,
    _class: JClass,
    chart_width: jint,
    chart_height: jint,
    arr_length: jint,
    java_x_title: JString,
    x_start: jdouble,
    x_end: jdouble,
    java_xs: jdoubleArray,
    java_y_title: JString,
    y_start: jdouble,
    y_end: jdouble,
    java_ys: jdoubleArray,
) -> jstring {
    try_java(env, std::ptr::null_mut(), || {
        let mut xs = vec![0.0; arr_length as usize];
        // Copy the xs
        env.get_double_array_region(java_xs, 0, &mut xs)?;

        let mut ys = vec![0.0; arr_length as usize];
        // Copy the ys
        env.get_double_array_region(java_ys, 0, &mut ys)?;

        // Get the x and y titles...
        let x_title: String = env.get_string(java_x_title)?.into();
        let y_title: String = env.get_string(java_y_title)?.into();

        // Copy the doubles into the right type for the underlying render API
        let points: Vec<(f64, f64)> = xs.into_iter().zip(ys.into_iter()).collect();
        let output: String = render_chart(
            chart_width as u32,
            chart_height as u32,
            &x_title,
            &y_title,
            x_start,
            x_end,
            y_start,
            y_end,
            points,
        )?;
        // Finally, create and return the Java String -
        // using the non-lifetimed representation
        Ok(env.new_string(&output)?.into_inner())
    })
}
// ANCHOR_END: complete


