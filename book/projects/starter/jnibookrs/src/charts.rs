// in charts.rs
use plotlib::page::Page;
use plotlib::repr::Plot;
use plotlib::style::{PointMarker, PointStyle};
use plotlib::view::ContinuousView;

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
