//! Fit profile editor side panel.
//!
//! Edits a [`FitProfile`]'s name, saddle target, and bar target in
//! BB-relative millimeters (with `+y` up).

use bike_fit_core::FitProfile;
use eframe::egui::{self, DragValue, Grid, Ui};

/// Show the fit-profile editor inside `ui`. Returns `true` if anything
/// changed.
pub fn show(ui: &mut Ui, profile: &mut FitProfile) -> bool {
    let mut changed = false;

    egui::CollapsingHeader::new("Profile")
        .default_open(true)
        .show(ui, |ui| {
            Grid::new("fit-identity")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Name");
                    let resp =
                        ui.add(egui::TextEdit::singleline(&mut profile.name).desired_width(220.0));
                    ui.end_row();
                    if resp.changed() {
                        changed = true;
                    }
                });
        });

    egui::CollapsingHeader::new("Saddle target (BB-relative)")
        .default_open(true)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new("Setback (x, negative = behind BB) and height (y, above BB).")
                    .small()
                    .weak(),
            );
            Grid::new("fit-saddle")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    changed |=
                        mm_signed_row(ui, "x (setback)", &mut profile.saddle.x, -300.0..=200.0);
                    changed |= mm_row(ui, "y (height)", &mut profile.saddle.y, 400.0..=900.0);
                });
        });

    egui::CollapsingHeader::new("Bar target (BB-relative)")
        .default_open(true)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(
                    "Center of the stem's far clamp face — bar-agnostic. \
                     The solver matches stem + spacers to land here.",
                )
                .small()
                .weak(),
            );
            Grid::new("fit-bar")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    changed |= mm_row(ui, "x (forward)", &mut profile.bar_target.x, 0.0..=700.0);
                    changed |= mm_row(ui, "y (height)", &mut profile.bar_target.y, 300.0..=800.0);
                });
        });

    egui::CollapsingHeader::new("Derived")
        .default_open(false)
        .show(ui, |ui| {
            Grid::new("fit-derived")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Saddle-to-bar drop");
                    ui.label(format!("{:.1} mm", profile.saddle_drop_mm()));
                    ui.end_row();
                    ui.label("Saddle-to-bar reach");
                    ui.label(format!("{:.1} mm", profile.saddle_to_bar_reach_mm()));
                    ui.end_row();
                });
        });

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

fn mm_signed_row(
    ui: &mut Ui,
    label: &str,
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
) -> bool {
    // Same as mm_row but explicit about allowing signed values; kept separate
    // in case we want a different control later (e.g. a centered slider).
    mm_row(ui, label, value, range)
}
