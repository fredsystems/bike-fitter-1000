//! Cockpit configuration editor.
//!
//! Lets the user pick a [`Cockpit`] kind, set the headset top cap, and tweak
//! the spacer catalog's max stack. Stem-catalog and spacer-SKU editing is
//! intentionally minimal in v1: the defaults cover what the major
//! manufacturers offer, and the panel exposes "Reset to defaults" buttons
//! for each so the user can recover after experimenting.

use bike_fit_core::{Cockpit, SpacerCatalog, StemCatalog};
use eframe::egui::{self, DragValue, Grid, Ui};

/// Show the cockpit editor inside `ui`. Returns `true` if anything changed.
pub fn show(ui: &mut Ui, cockpit: &mut Cockpit) -> bool {
    let mut changed = false;

    // Kind picker. Switching kind preserves the shared fields (top cap,
    // spacers) where possible and falls back to defaults for kind-specific
    // pieces.
    egui::CollapsingHeader::new("Kind")
        .default_open(true)
        .show(ui, |ui| {
            let current = kind_label(cockpit);
            let mut next: Option<&'static str> = None;
            egui::ComboBox::from_id_salt("cockpit-kind")
                .selected_text(current)
                .width(220.0)
                .show_ui(ui, |ui| {
                    for opt in ["Traditional", "Aero stem", "Integrated"] {
                        if ui.selectable_label(current == opt, opt).clicked() {
                            next = Some(opt);
                        }
                    }
                });
            if let Some(opt) = next {
                if opt != current {
                    *cockpit = switch_kind(cockpit, opt);
                    changed = true;
                }
            }
        });

    // Shared fields, edited in place by destructuring.
    egui::CollapsingHeader::new("Headset & spacers")
        .default_open(true)
        .show(ui, |ui| {
            let (top_cap, spacers) = match cockpit {
                Cockpit::Traditional {
                    headset_top_cap_mm,
                    spacers,
                    ..
                }
                | Cockpit::AeroStem {
                    headset_top_cap_mm,
                    spacers,
                    ..
                }
                | Cockpit::Integrated {
                    headset_top_cap_mm,
                    spacers,
                    ..
                } => (headset_top_cap_mm, spacers),
            };

            Grid::new("cockpit-shared")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Headset top cap");
                    let r = ui.add(
                        DragValue::new(top_cap)
                            .range(0.0..=20.0)
                            .speed(0.1)
                            .suffix(" mm")
                            .max_decimals(1),
                    );
                    ui.end_row();
                    if r.changed() {
                        changed = true;
                    }

                    ui.label("Max spacer stack");
                    let mut max = spacers.max_stack_mm;
                    let r = ui.add(
                        DragValue::new(&mut max)
                            .range(0u16..=120u16)
                            .speed(1.0)
                            .suffix(" mm"),
                    );
                    ui.end_row();
                    if r.changed() {
                        spacers.max_stack_mm = max;
                        changed = true;
                    }
                });

            // Display the available SKUs read-only with a reset button.
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(format!("SKUs: {}", spacer_sku_summary(spacers)))
                    .small()
                    .weak(),
            );
            if ui.button("Reset spacer SKUs to defaults").clicked() {
                let max_was = spacers.max_stack_mm;
                *spacers = SpacerCatalog::default_set();
                spacers.max_stack_mm = max_was;
                changed = true;
            }
        });

    // Kind-specific pieces.
    match cockpit {
        Cockpit::Traditional { stems, .. } | Cockpit::AeroStem { stems, .. } => {
            egui::CollapsingHeader::new("Stem catalog")
                .default_open(true)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(format!("{} stems available", stems.stems.len()))
                            .small()
                            .weak(),
                    );
                    ui.label(egui::RichText::new(stem_summary(stems)).small().weak());
                    ui.add_space(4.0);
                    // Show every stem so the user can see exactly what the
                    // solver is choosing from. Wrapped in a small scroll
                    // area so a 28-stem default catalog doesn't push the
                    // solver panel off-screen.
                    egui::ScrollArea::vertical()
                        .id_salt("stem-list")
                        .max_height(140.0)
                        .show(ui, |ui| {
                            for stem in &stems.stems {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "  {:>3.0} mm  ×  {:+.0}°",
                                        stem.length_mm, stem.angle_deg,
                                    ))
                                    .monospace()
                                    .small(),
                                );
                            }
                        });
                    ui.add_space(4.0);
                    if ui.button("Reset to default catalog").clicked() {
                        *stems = StemCatalog::default_traditional();
                        changed = true;
                    }
                });
        }
        Cockpit::Integrated { skus, .. } => {
            egui::CollapsingHeader::new("Integrated SKUs")
                .default_open(true)
                .show(ui, |ui| {
                    if skus.is_empty() {
                        ui.label(
                            egui::RichText::new(
                                "No SKUs configured. The solver has nothing \
                                 to search over until you add one. Inline \
                                 editor TBD; for now, switch back to \
                                 Traditional or Aero stem.",
                            )
                            .small()
                            .weak(),
                        );
                    } else {
                        for sku in skus.iter() {
                            ui.label(
                                egui::RichText::new(format!(
                                    "  {:>3.0} mm  ×  {:+.0}°  (bar reach {:.0}, drop {:.0})",
                                    sku.length_mm, sku.angle_deg, sku.bar_reach_mm, sku.bar_drop_mm,
                                ))
                                .monospace()
                                .small(),
                            );
                        }
                    }
                });
        }
    }

    changed
}

fn kind_label(c: &Cockpit) -> &'static str {
    match c {
        Cockpit::Traditional { .. } => "Traditional",
        Cockpit::AeroStem { .. } => "Aero stem",
        Cockpit::Integrated { .. } => "Integrated",
    }
}

/// Build a new Cockpit of the requested kind, preserving as much of `prev`'s
/// shared state (top cap, spacer catalog) as possible.
fn switch_kind(prev: &Cockpit, target: &str) -> Cockpit {
    let top_cap = prev.headset_top_cap_mm();
    let spacers = prev.spacers().clone();
    match target {
        "Traditional" => Cockpit::Traditional {
            stems: StemCatalog::default_traditional(),
            spacers,
            headset_top_cap_mm: top_cap,
        },
        "Aero stem" => Cockpit::AeroStem {
            // Aero stems come at fixed angles; we model that with a catalog
            // restricted to 0° SKUs by default. The user can customize.
            stems: StemCatalog {
                stems: [70.0, 80.0, 90.0, 100.0, 110.0, 120.0, 130.0]
                    .into_iter()
                    .map(|l| bike_fit_core::Stem {
                        length_mm: l,
                        angle_deg: 0.0,
                    })
                    .collect(),
            },
            spacers,
            headset_top_cap_mm: top_cap,
        },
        "Integrated" => Cockpit::Integrated {
            skus: Vec::new(),
            spacers,
            headset_top_cap_mm: top_cap,
        },
        _ => prev.clone(),
    }
}

fn spacer_sku_summary(s: &SpacerCatalog) -> String {
    if s.spacers.is_empty() {
        return "(none)".into();
    }
    s.spacers
        .iter()
        .map(|sp| format!("{} mm", sp.height_mm))
        .collect::<Vec<_>>()
        .join(", ")
}

fn stem_summary(s: &StemCatalog) -> String {
    if s.stems.is_empty() {
        return "(none)".into();
    }
    let mut lengths: Vec<i32> = s.stems.iter().map(|st| st.length_mm as i32).collect();
    lengths.sort_unstable();
    lengths.dedup();
    let mut angles: Vec<i32> = s.stems.iter().map(|st| st.angle_deg as i32).collect();
    angles.sort_unstable();
    angles.dedup();
    format!(
        "lengths {}–{} mm, angles {}",
        lengths.first().copied().unwrap_or(0),
        lengths.last().copied().unwrap_or(0),
        angles
            .iter()
            .map(|a| format!("{a:+}°"))
            .collect::<Vec<_>>()
            .join("/"),
    )
}
