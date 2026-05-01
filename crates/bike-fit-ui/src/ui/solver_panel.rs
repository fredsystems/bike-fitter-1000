//! Solver output panel.
//!
//! Runs [`bike_fit_core::solver::solve_for_profile`] and renders the result
//! as a human-readable build sheet: which stem to buy, how to stack the
//! spacers, and how far the resulting bar position is from the target.

use bike_fit_core::{solver, Cockpit, FitProfile, Frame};
use eframe::egui::{self, Color32, Grid, RichText, Ui};

/// Show the solver output. Read-only; never mutates state.
pub fn show(ui: &mut Ui, frame: &Frame, fit: &FitProfile, cockpit: &Cockpit) {
    let result = solver::solve_for_profile(frame, fit, cockpit);
    match result {
        Err(e) => {
            ui.label(
                RichText::new(format!("Cannot solve: {e}")).color(Color32::from_rgb(220, 120, 110)),
            );
        }
        Ok(build) => {
            // Recommended stem + spacers.
            egui::CollapsingHeader::new("Recommended build")
                .default_open(true)
                .show(ui, |ui| {
                    Grid::new("solver-build")
                        .num_columns(2)
                        .spacing([8.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Stem");
                            ui.label(format!(
                                "{:.0} mm × {:+.0}°",
                                build.stem.length_mm, build.stem.angle_deg,
                            ));
                            ui.end_row();

                            ui.label("Spacer stack");
                            ui.label(format!("{} mm total", build.spacer_stack.total_mm));
                            ui.end_row();

                            ui.label("Breakdown");
                            ui.label(spacer_breakdown_text(&build.spacer_stack));
                            ui.end_row();
                        });
                });

            // Residual error — the heart of the UX. Color codes how good
            // the fit landed.
            egui::CollapsingHeader::new("Fit accuracy")
                .default_open(true)
                .show(ui, |ui| {
                    let err = build.error_mm();
                    let (color, verdict) = verdict_for_err(err);
                    ui.horizontal(|ui| {
                        ui.label("Residual:");
                        ui.label(
                            RichText::new(format!("{err:.1} mm — {verdict}"))
                                .color(color)
                                .strong(),
                        );
                    });
                    ui.label(
                        RichText::new(format!(
                            "{} {:.1} mm, {} {:.1} mm",
                            if build.residual.x >= 0.0 {
                                "forward"
                            } else {
                                "behind"
                            },
                            build.residual.x.abs(),
                            if build.residual.y >= 0.0 {
                                "above"
                            } else {
                                "below"
                            },
                            build.residual.y.abs(),
                        ))
                        .small()
                        .weak(),
                    );
                });

            // Achieved position — nice for debugging.
            egui::CollapsingHeader::new("Geometry")
                .default_open(false)
                .show(ui, |ui| {
                    Grid::new("solver-geom")
                        .num_columns(2)
                        .spacing([8.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Target (bar)");
                            ui.label(format!(
                                "({:.1}, {:.1}) mm",
                                fit.bar_target.x, fit.bar_target.y
                            ));
                            ui.end_row();
                            ui.label("Achieved");
                            ui.label(format!(
                                "({:.1}, {:.1}) mm",
                                build.achieved_bar_position.x, build.achieved_bar_position.y,
                            ));
                            ui.end_row();
                        });
                });
        }
    }
}

fn spacer_breakdown_text(stack: &bike_fit_core::SpacerStack) -> String {
    if stack.total_mm == 0 {
        return "(none)".into();
    }
    stack
        .breakdown
        .iter()
        .map(|(sp, count)| format!("{}× {} mm", count, sp.height_mm))
        .collect::<Vec<_>>()
        .join(" + ")
}

fn verdict_for_err(err_mm: f64) -> (Color32, &'static str) {
    if err_mm < 1.0 {
        (Color32::from_rgb(120, 200, 120), "excellent")
    } else if err_mm < 3.0 {
        (Color32::from_rgb(180, 200, 120), "good")
    } else if err_mm < 6.0 {
        (Color32::from_rgb(220, 200, 100), "acceptable")
    } else {
        (Color32::from_rgb(220, 130, 110), "off-target")
    }
}
