//! Scheduler page — 7×24 weekly grid with drag-to-paint, speed limiting,
//! and on-completion actions. Uses cairo for grid rendering.

use adw::prelude::*;
use cairo::Context as CairoContext;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::engine_bridge::EngineBridge;
use crate::model::AppModel;

/// Cell modes for the scheduler grid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CellMode {
    FullSpeed,
    Limited,
    Paused,
}

impl Default for CellMode {
    fn default() -> Self { CellMode::FullSpeed }
}

const DAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const HOURS: usize = 24;

pub fn build_scheduler_page(_model: &AppModel, bridge: &EngineBridge) -> gtk::Box {
    let page = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_start(12)
        .margin_end(12)
        .margin_top(12)
        .margin_bottom(12)
        .build();

    // Title
    let title = gtk::Label::builder()
        .label("Download Scheduler")
        .css_classes(["title-2"])
        .xalign(0.0)
        .build();
    page.append(&title);

    // Paint mode selector
    let mode_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .halign(gtk::Align::Center)
        .build();

    let paint_mode: Rc<Cell<CellMode>> = Rc::new(Cell::new(CellMode::FullSpeed));

    let modes = [
        ("Full Speed", CellMode::FullSpeed),
        ("Limited", CellMode::Limited),
        ("Paused", CellMode::Paused),
    ];

    for (label, mode) in &modes {
        let btn = gtk::ToggleButton::builder()
            .label(*label)
            .build();
        if *mode == CellMode::FullSpeed { btn.set_active(true); }
        let paint = paint_mode.clone();
        let m = *mode;
        btn.connect_toggled(move |b| {
            if b.is_active() { paint.set(m); }
        });
        mode_box.append(&btn);
    }
    page.append(&mode_box);

    // Grid data: 7 days × 24 hours
    let grid_data: Rc<RefCell<Vec<Vec<CellMode>>>> = Rc::new(RefCell::new(
        vec![vec![CellMode::FullSpeed; HOURS]; 7]
    ));

    // Drawing area for the grid
    let grid_area = gtk::DrawingArea::builder()
        .height_request(300)
        .hexpand(true)
        .css_classes(["card"])
        .build();

    // Draw the grid
    {
        let grid_data = grid_data.clone();
        grid_area.set_draw_func(move |_, cr, width, height| {
            draw_scheduler_grid(cr, width as f64, height as f64, &grid_data.borrow());
        });
    }

    // Handle drag-to-paint via GtkGestureDrag
    let gesture = gtk::GestureDrag::new();
    let is_painting = Rc::new(Cell::new(false));

    {
        let grid_data = grid_data.clone();
        let paint_mode = paint_mode.clone();
        let grid_area = grid_area.clone();
        let is_painting = is_painting.clone();

        gesture.connect_drag_begin(move |_gesture, x, y| {
            is_painting.set(true);
            let width = grid_area.width() as f64;
            let height = grid_area.height() as f64;
            if let Some((day, hour)) = pixel_to_cell(x, y, width, height) {
                grid_data.borrow_mut()[day][hour] = paint_mode.get();
                grid_area.queue_draw();
            }
        });
    }

    {
        let grid_data = grid_data.clone();
        let paint_mode = paint_mode.clone();
        let grid_area_ref = grid_area.clone();
        let is_painting = is_painting.clone();

        gesture.connect_drag_update(move |gesture, offset_x, offset_y| {
            if !is_painting.get() { return; }
            if let Some((start_x, start_y)) = gesture.start_point() {
                let x = start_x + offset_x;
                let y = start_y + offset_y;
                let width = grid_area_ref.width() as f64;
                let height = grid_area_ref.height() as f64;
                if let Some((day, hour)) = pixel_to_cell(x, y, width, height) {
                    grid_data.borrow_mut()[day][hour] = paint_mode.get();
                    grid_area_ref.queue_draw();
                }
            }
        });
    }

    {
        let is_painting = is_painting.clone();
        gesture.connect_drag_end(move |_, _, _| {
            is_painting.set(false);
        });
    }

    grid_area.add_controller(gesture);
    page.append(&grid_area);

    // Config section
    let config_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(16)
        .homogeneous(true)
        .build();

    // Speed limit config
    let speed_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .halign(gtk::Align::Center)
        .build();
    let speed_label = gtk::Label::new(Some("Speed limit:"));
    let speed_spin = gtk::SpinButton::with_range(1.0, 10000.0, 100.0);
    speed_spin.set_value(1024.0);
    let unit_combo = gtk::DropDown::from_strings(&["KB/s", "MB/s"]);
    speed_box.append(&speed_label);
    speed_box.append(&speed_spin);
    speed_box.append(&unit_combo);
    config_box.append(&speed_box);

    // Enable schedule toggle
    let enable_row = adw::SwitchRow::builder()
        .title("Enable Schedule")
        .active(false)
        .build();
    config_box.append(&enable_row);

    page.append(&config_box);

    // On-completion action
    let completion_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .margin_top(8)
        .build();
    let completion_label = gtk::Label::new(Some("On completion:"));
    let completion_combo = gtk::DropDown::from_strings(&[
        "Do Nothing", "Close App", "Sleep", "Shutdown"
    ]);
    completion_box.append(&completion_label);
    completion_box.append(&completion_combo);
    page.append(&completion_box);

    // Save button
    let save_btn = gtk::Button::builder()
        .label("Save Schedule")
        .css_classes(["suggested-action"])
        .halign(gtk::Align::End)
        .margin_top(12)
        .build();
    {
        let grid_data = grid_data.clone();
        let bridge = bridge.clone();
        let speed_spin = speed_spin.clone();
        let unit_combo = unit_combo.clone();
        save_btn.connect_clicked(move |_| {
            let rules = grid_to_rules(&grid_data.borrow(), speed_spin.value() as u64, unit_combo.selected());
            bridge.set_schedule_rules(rules);
        });
    }
    page.append(&save_btn);

    page
}

/// Draw the 7×24 scheduler grid.
fn draw_scheduler_grid(
    cr: &CairoContext,
    width: f64,
    height: f64,
    grid: &[Vec<CellMode>],
) {
    let label_width = 40.0;
    let label_height = 25.0;
    let cell_w = (width - label_width) / HOURS as f64;
    let cell_h = (height - label_height) / 7.0;

    // Hour labels
    cr.set_source_rgba(0.7, 0.7, 0.7, 1.0);
    cr.set_font_size(10.0);
    for h in 0..HOURS {
        let x = label_width + h as f64 * cell_w + cell_w / 2.0 - 4.0;
        cr.move_to(x, label_height - 5.0);
        cr.show_text(&format!("{}", h)).ok();
    }

    // Day labels and cells
    for (day, row) in grid.iter().enumerate() {
        let y = label_height + day as f64 * cell_h;

        // Day label
        cr.set_source_rgba(0.7, 0.7, 0.7, 1.0);
        cr.move_to(5.0, y + cell_h / 2.0 + 4.0);
        cr.show_text(DAYS[day]).ok();

        for (hour, mode) in row.iter().enumerate() {
            let x = label_width + hour as f64 * cell_w;

            // Cell color
            match mode {
                CellMode::FullSpeed => cr.set_source_rgba(0.2, 0.5, 0.9, 0.7),
                CellMode::Limited => cr.set_source_rgba(0.9, 0.7, 0.2, 0.7),
                CellMode::Paused => cr.set_source_rgba(0.3, 0.3, 0.3, 0.7),
            }
            cr.rectangle(x + 1.0, y + 1.0, cell_w - 2.0, cell_h - 2.0);
            cr.fill().ok();

            // Cell border
            cr.set_source_rgba(0.4, 0.4, 0.4, 0.3);
            cr.set_line_width(0.5);
            cr.rectangle(x, y, cell_w, cell_h);
            cr.stroke().ok();
        }
    }
}

/// Convert pixel coordinates to grid cell (day, hour).
fn pixel_to_cell(x: f64, y: f64, width: f64, height: f64) -> Option<(usize, usize)> {
    let label_width = 40.0;
    let label_height = 25.0;
    let cell_w = (width - label_width) / HOURS as f64;
    let cell_h = (height - label_height) / 7.0;

    let hour = ((x - label_width) / cell_w) as usize;
    let day = ((y - label_height) / cell_h) as usize;

    if day < 7 && hour < HOURS && x >= label_width && y >= label_height {
        Some((day, hour))
    } else {
        None
    }
}

/// Convert grid state to schedule rules JSON for the engine.
fn grid_to_rules(grid: &[Vec<CellMode>], speed_limit: u64, unit_index: u32) -> serde_json::Value {
    let speed_bytes = if unit_index == 1 {
        speed_limit * 1024 * 1024 // MB/s
    } else {
        speed_limit * 1024 // KB/s
    };

    let mut rules = Vec::new();

    for (day, row) in grid.iter().enumerate() {
        let mut hour = 0;
        while hour < HOURS {
            let mode = row[hour];
            if mode == CellMode::FullSpeed {
                hour += 1;
                continue;
            }

            // Find contiguous range of same mode
            let start_hour = hour;
            while hour < HOURS && row[hour] == mode {
                hour += 1;
            }

            let rule = serde_json::json!({
                "day": day,
                "start_hour": start_hour,
                "end_hour": hour,
                "mode": match mode {
                    CellMode::Limited => "limited",
                    CellMode::Paused => "paused",
                    CellMode::FullSpeed => "full",
                },
                "download_limit": if mode == CellMode::Limited { speed_bytes } else { 0 },
            });
            rules.push(rule);
        }
    }

    serde_json::Value::Array(rules)
}
