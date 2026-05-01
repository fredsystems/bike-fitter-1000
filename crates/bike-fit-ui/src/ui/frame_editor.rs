//! Frame editor side panel.
//!
//! Renders a grouped form of every published [`Frame`] field. Returns `true`
//! whenever the user mutated something so the caller can flag state dirty
//! and persist.

use bike_fit_core::{frame::WheelSize, Frame};
use eframe::egui::{self, DragValue, Grid, Ui};

/// Show the frame editor inside a [`Ui`]. Mutates `frame` in place. Returns
/// `true` if any field was changed this frame.
pub fn show(ui: &mut Ui, frame: &mut Frame) -> bool {
    let mut changed = false;

    egui::CollapsingHeader::new("Identity")
        .default_open(true)
        .show(ui, |ui| {
            Grid::new("frame-identity")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    changed |= text_row(ui, "Manufacturer", &mut frame.manufacturer);
                    changed |= text_row(ui, "Model", &mut frame.model);
                    changed |= text_row(ui, "Size", &mut frame.size_label);
                    changed |= year_row(ui, "Year", &mut frame.year);
                });
        });

    egui::CollapsingHeader::new("Frame triangle")
        .default_open(true)
        .show(ui, |ui| {
            Grid::new("frame-triangle")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    changed |= mm_row(ui, "Stack", &mut frame.stack_mm, 0.0..=900.0);
                    changed |= mm_row(ui, "Reach", &mut frame.reach_mm, 0.0..=600.0);
                    changed |= deg_row(
                        ui,
                        "Head tube angle",
                        &mut frame.head_tube_angle_deg,
                        60.0..=80.0,
                    );
                    changed |= mm_row(
                        ui,
                        "Head tube length",
                        &mut frame.head_tube_length_mm,
                        0.0..=300.0,
                    );
                    changed |= deg_row(
                        ui,
                        "Seat tube angle",
                        &mut frame.seat_tube_angle_deg,
                        65.0..=80.0,
                    );
                    changed |= mm_row(
                        ui,
                        "Seat tube length",
                        &mut frame.seat_tube_length_mm,
                        0.0..=700.0,
                    );
                    changed |= mm_row(
                        ui,
                        "Top tube (effective)",
                        &mut frame.top_tube_effective_mm,
                        0.0..=700.0,
                    );
                });
        });

    egui::CollapsingHeader::new("Drivetrain & axles")
        .default_open(true)
        .show(ui, |ui| {
            Grid::new("frame-axles")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    changed |= mm_row(ui, "BB drop", &mut frame.bb_drop_mm, -20.0..=120.0);
                    changed |= mm_row(ui, "Chainstay", &mut frame.chainstay_mm, 380.0..=460.0);
                    changed |= mm_row(ui, "Fork rake", &mut frame.fork_rake_mm, 30.0..=70.0);
                    changed |= optional_mm_row(
                        ui,
                        "Front-center horiz.",
                        &mut frame.front_center_horizontal_mm,
                        500.0..=700.0,
                    );
                });
        });

    egui::CollapsingHeader::new("Wheels & tires")
        .default_open(true)
        .show(ui, |ui| {
            Grid::new("frame-wheels")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    changed |= wheel_size_row(ui, "Wheel size", &mut frame.wheel_size);
                    changed |= mm_row(ui, "Tire width", &mut frame.tire_width_mm, 18.0..=60.0);
                });
        });

    changed
}

fn text_row(ui: &mut Ui, label: &str, value: &mut String) -> bool {
    ui.label(label);
    let resp = ui.add(egui::TextEdit::singleline(value).desired_width(220.0));
    ui.end_row();
    resp.changed()
}

fn year_row(ui: &mut Ui, label: &str, value: &mut Option<u16>) -> bool {
    ui.label(label);
    let mut changed = false;
    ui.horizontal(|ui| {
        let mut have = value.is_some();
        if ui.checkbox(&mut have, "known").changed() {
            *value = if have { Some(2025) } else { None };
            changed = true;
        }
        if let Some(y) = value.as_mut() {
            let resp = ui.add(DragValue::new(y).range(1900..=2100).speed(1.0));
            if resp.changed() {
                changed = true;
            }
        }
    });
    ui.end_row();
    changed
}

fn mm_row(ui: &mut Ui, label: &str, value: &mut f64, range: std::ops::RangeInclusive<f64>) -> bool {
    ui.label(label);
    let resp = ui.add(
        DragValue::new(value)
            .range(range)
            .speed(0.5)
            .suffix(" mm")
            .max_decimals(1),
    );
    ui.end_row();
    resp.changed()
}

fn deg_row(
    ui: &mut Ui,
    label: &str,
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
) -> bool {
    ui.label(label);
    let resp = ui.add(
        DragValue::new(value)
            .range(range)
            .speed(0.05)
            .suffix("°")
            .max_decimals(2),
    );
    ui.end_row();
    resp.changed()
}

fn optional_mm_row(
    ui: &mut Ui,
    label: &str,
    value: &mut Option<f64>,
    range: std::ops::RangeInclusive<f64>,
) -> bool {
    ui.label(label);
    let mut changed = false;
    ui.horizontal(|ui| {
        let mut have = value.is_some();
        if ui.checkbox(&mut have, "known").changed() {
            *value = if have {
                Some(*range.start() + (*range.end() - *range.start()) * 0.5)
            } else {
                None
            };
            changed = true;
        }
        if let Some(v) = value.as_mut() {
            let resp = ui.add(
                DragValue::new(v)
                    .range(range)
                    .speed(0.5)
                    .suffix(" mm")
                    .max_decimals(1),
            );
            if resp.changed() {
                changed = true;
            }
        }
    });
    ui.end_row();
    changed
}

fn wheel_size_row(ui: &mut Ui, label: &str, value: &mut WheelSize) -> bool {
    ui.label(label);
    let mut changed = false;
    ui.horizontal(|ui| {
        let current_label = match value {
            WheelSize::Iso622 => "700C / 29\" (622)",
            WheelSize::Iso584 => "650B / 27.5\" (584)",
            WheelSize::Custom(_) => "Custom",
        };
        egui::ComboBox::from_id_salt(format!("wheel-{label}"))
            .selected_text(current_label)
            .show_ui(ui, |ui| {
                for opt in [
                    WheelSize::Iso622,
                    WheelSize::Iso584,
                    WheelSize::Custom(value.bsd_mm() as u16),
                ] {
                    let label_text = match opt {
                        WheelSize::Iso622 => "700C / 29\" (622)",
                        WheelSize::Iso584 => "650B / 27.5\" (584)",
                        WheelSize::Custom(_) => "Custom",
                    };
                    if ui
                        .selectable_label(
                            std::mem::discriminant(value) == std::mem::discriminant(&opt),
                            label_text,
                        )
                        .clicked()
                    {
                        *value = opt;
                        changed = true;
                    }
                }
            });
        if let WheelSize::Custom(b) = value {
            let resp = ui.add(
                DragValue::new(b)
                    .range(400u16..=900u16)
                    .speed(1.0)
                    .suffix(" mm BSD"),
            );
            if resp.changed() {
                changed = true;
            }
        }
    });
    ui.end_row();
    changed
}
