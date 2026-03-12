//! Statistics page — real-time bandwidth charts, session stats, top domains.
//! Uses cairo for chart rendering with Catmull-Rom spline interpolation.

use adw::prelude::*;
use cairo::Context as CairoContext;
use gtk::glib;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use crate::engine_bridge::EngineBridge;
use crate::model::{AppModel, SpeedSample};
use crate::widgets::status_bar::format_speed;

pub fn build_statistics_page(model: &AppModel, _bridge: &EngineBridge) -> gtk::Box {
    let page = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_start(12)
        .margin_end(12)
        .margin_top(12)
        .margin_bottom(12)
        .build();

    // Stat cards row
    let cards_row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .homogeneous(true)
        .build();

    let dl_total_label = Rc::new(build_stat_card(&cards_row, "Total Downloaded", "0 B"));
    let ul_total_label = Rc::new(build_stat_card(&cards_row, "Total Uploaded", "0 B"));
    let avg_speed_label = Rc::new(build_stat_card(&cards_row, "Average Speed", "0 B/s"));
    page.append(&cards_row);

    // Period toggle
    let period_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(4)
        .halign(gtk::Align::Center)
        .css_classes(["linked"])
        .build();

    let period: Rc<RefCell<u32>> = Rc::new(RefCell::new(300)); // 5 minutes in seconds

    for (label, secs) in &[("5 min", 300u32), ("30 min", 1800), ("Session", 0)] {
        let btn = gtk::ToggleButton::builder()
            .label(*label)
            .build();
        if *secs == 300 { btn.set_active(true); }
        let period = period.clone();
        let s = *secs;
        btn.connect_toggled(move |b| {
            if b.is_active() { *period.borrow_mut() = s; }
        });
        period_box.append(&btn);
    }
    page.append(&period_box);

    // Speed chart (cairo drawing area)
    let chart_area = gtk::DrawingArea::builder()
        .height_request(250)
        .hexpand(true)
        .css_classes(["card"])
        .build();

    let samples: Rc<RefCell<VecDeque<SpeedSample>>> = Rc::new(RefCell::new(VecDeque::new()));

    // Drawing function
    {
        let samples = samples.clone();
        let period = period.clone();
        chart_area.set_draw_func(move |_, cr, width, height| {
            draw_speed_chart(cr, width as f64, height as f64, &samples.borrow(), *period.borrow());
        });
    }

    page.append(&chart_area);

    // Session info row
    let session_row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .homogeneous(true)
        .build();

    let uptime_label = Rc::new(build_info_row(&session_row, "Uptime", "0:00"));
    let active_label = Rc::new(build_info_row(&session_row, "Active Threads", "0"));
    let peak_label = Rc::new(build_info_row(&session_row, "Peak Speed", "0 B/s"));
    let conn_label = Rc::new(build_info_row(&session_row, "Connection", "Online"));
    page.append(&session_row);

    // Speed sampling timer (every 3 seconds)
    let start_time = std::time::Instant::now();
    let peak_speed: Rc<RefCell<u64>> = Rc::new(RefCell::new(0));
    let total_downloaded: Rc<RefCell<u64>> = Rc::new(RefCell::new(0));
    let total_uploaded: Rc<RefCell<u64>> = Rc::new(RefCell::new(0));
    let sample_count: Rc<RefCell<u64>> = Rc::new(RefCell::new(0));
    let speed_sum: Rc<RefCell<u64>> = Rc::new(RefCell::new(0));

    {
        let model = model.clone();
        let chart_area = chart_area.clone();
        let samples = samples.clone();
        let peak_speed = peak_speed.clone();
        let total_downloaded = total_downloaded.clone();
        let total_uploaded = total_uploaded.clone();
        let sample_count = sample_count.clone();
        let speed_sum = speed_sum.clone();
        let dl_total_label = dl_total_label.clone();
        let ul_total_label = ul_total_label.clone();
        let avg_speed_label = avg_speed_label.clone();
        let uptime_label = uptime_label.clone();
        let active_label = active_label.clone();
        let peak_label = peak_label.clone();
        let conn_label = conn_label.clone();

        glib::timeout_add_seconds_local(3, move || {
            let dl_speed = model.download_speed();
            let ul_speed = model.upload_speed();

            // Record sample
            let now = start_time.elapsed().as_secs_f64();
            let sample = SpeedSample {
                timestamp: now,
                download_speed: dl_speed,
                upload_speed: ul_speed,
            };
            samples.borrow_mut().push_back(sample.clone());
            while samples.borrow().len() > 1200 {
                samples.borrow_mut().pop_front();
            }
            model.add_speed_sample(sample);

            // Update peak
            let mut peak = peak_speed.borrow_mut();
            if dl_speed > *peak { *peak = dl_speed; }

            // Accumulate totals (approximate: speed * interval)
            *total_downloaded.borrow_mut() += dl_speed * 3;
            *total_uploaded.borrow_mut() += ul_speed * 3;
            *sample_count.borrow_mut() += 1;
            *speed_sum.borrow_mut() += dl_speed;

            // Update stat cards
            dl_total_label.set_label(&format_size(*total_downloaded.borrow()));
            ul_total_label.set_label(&format_size(*total_uploaded.borrow()));
            let count = *sample_count.borrow();
            if count > 0 {
                avg_speed_label.set_label(&format_speed(*speed_sum.borrow() / count));
            }

            // Update session info
            let elapsed = start_time.elapsed().as_secs();
            uptime_label.set_label(&format!("{}:{:02}:{:02}", elapsed / 3600, (elapsed % 3600) / 60, elapsed % 60));
            active_label.set_label(&model.num_active().to_string());
            peak_label.set_label(&format_speed(*peak));
            conn_label.set_label(if model.is_connected() { "Online" } else { "Offline" });

            // Redraw chart
            chart_area.queue_draw();

            glib::ControlFlow::Continue
        });
    }

    page
}

fn build_stat_card(parent: &gtk::Box, title: &str, initial_value: &str) -> gtk::Label {
    let card = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(4)
        .css_classes(["card"])
        .margin_start(4)
        .margin_end(4)
        .margin_top(8)
        .margin_bottom(8)
        .halign(gtk::Align::Center)
        .build();

    let title_label = gtk::Label::builder()
        .label(title)
        .css_classes(["caption", "dim-label"])
        .build();
    card.append(&title_label);

    let value_label = gtk::Label::builder()
        .label(initial_value)
        .css_classes(["title-3"])
        .build();
    card.append(&value_label);

    parent.append(&card);
    value_label
}

fn build_info_row(parent: &gtk::Box, title: &str, initial_value: &str) -> gtk::Label {
    let vbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(2)
        .halign(gtk::Align::Center)
        .build();

    let title_label = gtk::Label::builder()
        .label(title)
        .css_classes(["caption", "dim-label"])
        .build();
    vbox.append(&title_label);

    let value_label = gtk::Label::builder()
        .label(initial_value)
        .css_classes(["body"])
        .build();
    vbox.append(&value_label);

    parent.append(&vbox);
    value_label
}

/// Draw the speed chart using cairo with Catmull-Rom spline interpolation.
fn draw_speed_chart(
    cr: &CairoContext,
    width: f64,
    height: f64,
    samples: &VecDeque<SpeedSample>,
    period_secs: u32,
) {
    let padding_left = 60.0;
    let padding_right = 20.0;
    let padding_top = 20.0;
    let padding_bottom = 30.0;

    let chart_width = width - padding_left - padding_right;
    let chart_height = height - padding_top - padding_bottom;

    if chart_width <= 0.0 || chart_height <= 0.0 || samples.is_empty() {
        return;
    }

    // Filter samples by period
    let now = samples.back().map(|s| s.timestamp).unwrap_or(0.0);
    let start = if period_secs > 0 {
        now - period_secs as f64
    } else {
        samples.front().map(|s| s.timestamp).unwrap_or(0.0)
    };

    let visible: Vec<&SpeedSample> = samples.iter()
        .filter(|s| s.timestamp >= start)
        .collect();

    if visible.is_empty() {
        return;
    }

    // Find max speed for Y-axis
    let max_speed = visible.iter()
        .map(|s| s.download_speed.max(s.upload_speed))
        .max()
        .unwrap_or(1024);
    let nice_max = get_nice_max(max_speed as f64);

    // Background
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
    cr.paint().ok();

    // Grid lines
    cr.set_source_rgba(0.5, 0.5, 0.5, 0.15);
    cr.set_line_width(0.5);
    for i in 0..=4 {
        let y = padding_top + chart_height * (1.0 - i as f64 / 4.0);
        cr.move_to(padding_left, y);
        cr.line_to(width - padding_right, y);
        cr.stroke().ok();

        // Y-axis label
        let val = nice_max * i as f64 / 4.0;
        cr.set_source_rgba(0.7, 0.7, 0.7, 1.0);
        cr.move_to(5.0, y + 4.0);
        cr.show_text(&format_speed(val as u64)).ok();
    }

    let time_range = now - start;
    if time_range <= 0.0 { return; }

    // Draw download speed line (blue gradient fill + line)
    draw_speed_line(cr, &visible, padding_top, chart_height, nice_max, start, time_range,
                    padding_left, chart_width, 0.33, 0.55, 0.95, |s| s.download_speed as f64);

    // Draw upload speed line (green, thinner)
    draw_speed_line(cr, &visible, padding_top, chart_height, nice_max, start, time_range,
                    padding_left, chart_width, 0.2, 0.8, 0.4, |s| s.upload_speed as f64);
}

fn draw_speed_line(
    cr: &CairoContext,
    samples: &[&SpeedSample],
    padding_top: f64,
    chart_height: f64,
    nice_max: f64,
    start: f64,
    time_range: f64,
    padding_left: f64,
    chart_width: f64,
    r: f64, g: f64, b: f64,
    get_speed: impl Fn(&SpeedSample) -> f64,
) {
    if samples.len() < 2 { return; }

    let to_x = |t: f64| -> f64 {
        padding_left + ((t - start) / time_range) * chart_width
    };
    let to_y = |speed: f64| -> f64 {
        padding_top + chart_height * (1.0 - speed / nice_max)
    };

    let baseline_y = padding_top + chart_height;

    // Build path using Catmull-Rom to Bezier conversion
    let points: Vec<(f64, f64)> = samples.iter()
        .map(|s| (to_x(s.timestamp), to_y(get_speed(s))))
        .collect();

    // Start path for fill
    cr.move_to(points[0].0, baseline_y);
    cr.line_to(points[0].0, points[0].1);

    for i in 0..points.len() - 1 {
        let p0 = if i > 0 { points[i - 1] } else { points[i] };
        let p1 = points[i];
        let p2 = points[i + 1];
        let p3 = if i + 2 < points.len() { points[i + 2] } else { points[i + 1] };

        // Catmull-Rom to Bezier control points
        let cp1x = p1.0 + (p2.0 - p0.0) / 6.0;
        let cp1y = p1.1 + (p2.1 - p0.1) / 6.0;
        let cp2x = p2.0 - (p3.0 - p1.0) / 6.0;
        let cp2y = p2.1 - (p3.1 - p1.1) / 6.0;

        cr.curve_to(cp1x, cp1y, cp2x, cp2y, p2.0, p2.1);
    }

    // Close fill path
    let last_x = points.last().unwrap().0;
    cr.line_to(last_x, baseline_y);
    cr.close_path();

    // Gradient fill
    let gradient = cairo::LinearGradient::new(0.0, padding_top, 0.0, baseline_y);
    gradient.add_color_stop_rgba(0.0, r, g, b, 0.3);
    gradient.add_color_stop_rgba(1.0, r, g, b, 0.02);
    cr.set_source(&gradient).ok();
    cr.fill_preserve().ok();

    // Stroke the line
    cr.set_source_rgba(r, g, b, 0.9);
    cr.set_line_width(2.0);
    // Rebuild just the line path (without the baseline closure)
    cr.new_path();
    cr.move_to(points[0].0, points[0].1);
    for i in 0..points.len() - 1 {
        let p0 = if i > 0 { points[i - 1] } else { points[i] };
        let p1 = points[i];
        let p2 = points[i + 1];
        let p3 = if i + 2 < points.len() { points[i + 2] } else { points[i + 1] };

        let cp1x = p1.0 + (p2.0 - p0.0) / 6.0;
        let cp1y = p1.1 + (p2.1 - p0.1) / 6.0;
        let cp2x = p2.0 - (p3.0 - p1.0) / 6.0;
        let cp2y = p2.1 - (p3.1 - p1.1) / 6.0;

        cr.curve_to(cp1x, cp1y, cp2x, cp2y, p2.0, p2.1);
    }
    cr.stroke().ok();
}

/// Round up to a "nice" value for the Y-axis maximum.
fn get_nice_max(max: f64) -> f64 {
    if max <= 0.0 { return 1024.0; } // Minimum 1 KB
    let magnitude = 10f64.powf(max.log10().floor());
    let normalized = max / magnitude;
    let nice = if normalized <= 1.0 { 1.0 }
        else if normalized <= 2.0 { 2.0 }
        else if normalized <= 5.0 { 5.0 }
        else { 10.0 };
    nice * magnitude
}

fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * KB;
    const GB: f64 = 1024.0 * MB;
    let b = bytes as f64;
    if b >= GB { format!("{:.2} GB", b / GB) }
    else if b >= MB { format!("{:.1} MB", b / MB) }
    else if b >= KB { format!("{:.1} KB", b / KB) }
    else { format!("{} B", bytes) }
}
