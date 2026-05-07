//! Shared egui UI for bike-fitter-1000.
//!
//! This crate contains the [`App`] (an [`eframe::App`] implementer), the
//! side-view renderer, and panel components. It is target-agnostic: both the
//! native `bike-fit-app` binary and (eventually) the wasm `bike-fit-web`
//! crate hand it a [`Persistence`] impl and let it run.

pub mod frames;
pub mod render;
pub mod ui;

use std::sync::Arc;

use bike_fit_core::{Cockpit, FitProfile, Frame, Point};
use eframe::egui;

pub use frames::Preset;
pub use render::{
    paint_achieved_overlay, paint_frame, paint_frame_with_overlay, show_frame, RenderStyle,
};

/// Pluggable storage backend.
///
/// Native uses a JSON file on disk; web uses `localStorage`. Both go
/// through this trait so the rest of the UI doesn't care.
///
/// The trait is intentionally not `Send + Sync`: the wasm `localStorage`
/// implementation uses `RefCell` (single-threaded by construction) and
/// the native impl already serializes through a `Mutex` internally. The
/// app holds the persistence as an `Rc<dyn Persistence>` since eframe's
/// per-frame update loop is single-threaded on both targets.
pub trait Persistence: 'static {
    /// Load the persisted state, if any.
    fn load(&self) -> Option<AppState>;
    /// Persist the state. Errors are logged and otherwise swallowed; this
    /// fires often (every interactive change), so transient I/O failures
    /// shouldn't crash the UI.
    fn save(&self, state: &AppState);
}

/// One bike: a chosen frame plus the cockpit configuration the solver should
/// search over for it.
///
/// `active_frame_key` is the stable key from `data/bikes.json`; it's a
/// breadcrumb for "which preset is this based on" so the user can reset to
/// the preset definition. The `frame` field is authoritative — edits in the
/// frame editor write directly into it and the key is just a label.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Bike {
    pub active_frame_key: String,
    pub frame: Frame,
    pub cockpit: Cockpit,
}

impl Bike {
    fn from_preset(p: Preset) -> Self {
        Self {
            active_frame_key: p.key,
            frame: p.frame,
            cockpit: p.default_cockpit.unwrap_or_else(Cockpit::default_traditional),
        }
    }

    /// Pick the nth preset (clamped); used for sensible per-bike defaults.
    fn from_nth_preset(n: usize) -> Self {
        let presets = frames::all();
        let idx = n.min(presets.len().saturating_sub(1));
        Self::from_preset(
            presets
                .into_iter()
                .nth(idx)
                .expect("at least one preset; verified by frames::all"),
        )
    }
}

/// Whether the app is showing one bike or comparing two.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AppMode {
    #[default]
    Single,
    Compare,
}

/// Which bike a UI control is editing.
///
/// Internal helper, not persisted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BikeSlot {
    A,
    B,
}

/// In-memory persistent state. Plain data, no UI.
///
/// Compare mode keeps two [`Bike`]s in lockstep against a single shared
/// [`FitProfile`]: that's the whole point — same fit, two frames, what
/// cockpit gets you there?
///
/// `bike_b` and `mode` are `#[serde(default)]` so loading an older persisted
/// state file (pre-milestone-8) keeps working: the user comes back to Single
/// mode with their original bike and a freshly-defaulted bike B sitting in
/// the wings.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AppState {
    pub bike_a: Bike,
    #[serde(default = "Bike::default_b")]
    pub bike_b: Bike,
    pub fit: FitProfile,
    #[serde(default)]
    pub mode: AppMode,
}

impl Bike {
    fn default_b() -> Self {
        // Default bike B to the second preset so a fresh comparison shows
        // two visibly different frames.
        Self::from_nth_preset(1)
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            bike_a: Bike::from_nth_preset(0),
            bike_b: Bike::default_b(),
            fit: FitProfile::new(
                "rider-1",
                Point::new(-50.0, 730.0),
                Point::new(420.0, 580.0),
            ),
            mode: AppMode::Single,
        }
    }
}

/// The shared application — an [`eframe::App`] that doesn't know whether it's
/// running on the desktop or in a browser.
pub struct App {
    state: AppState,
    persistence: Arc<dyn Persistence>,
    /// Which bike the left frame-editor panel is currently editing. Only
    /// meaningful in `AppMode::Compare`. Not persisted.
    editing_slot: BikeSlot,
}

impl App {
    pub fn new(persistence: Arc<dyn Persistence>) -> Self {
        let state = persistence.load().unwrap_or_default();
        Self {
            state,
            persistence,
            editing_slot: BikeSlot::A,
        }
    }

    fn persist(&self) {
        self.persistence.save(&self.state);
    }

    fn bike(&self, slot: BikeSlot) -> &Bike {
        match slot {
            BikeSlot::A => &self.state.bike_a,
            BikeSlot::B => &self.state.bike_b,
        }
    }

    fn bike_mut(&mut self, slot: BikeSlot) -> &mut Bike {
        match slot {
            BikeSlot::A => &mut self.state.bike_a,
            BikeSlot::B => &mut self.state.bike_b,
        }
    }

    /// Re-apply the active preset to `slot`'s live frame, discarding any
    /// local edits. No-op if the active key isn't in the preset list (e.g.
    /// for a frame the user fully authored).
    fn reset_frame_to_preset(&mut self, slot: BikeSlot) -> bool {
        let key = self.bike(slot).active_frame_key.clone();
        if let Some(p) = frames::by_key(&key) {
            let bike = self.bike_mut(slot);
            if bike.frame != p.frame {
                bike.frame = p.frame;
                return true;
            }
        }
        false
    }

    /// Has `slot`'s live frame diverged from its preset?
    fn frame_diverged_from_preset(&self, slot: BikeSlot) -> bool {
        let bike = self.bike(slot);
        frames::by_key(&bike.active_frame_key).is_some_and(|p| p.frame != bike.frame)
    }

    /// Render style for `slot`: default style tinted by the active key so
    /// each bike has a visibly distinct background, even side-by-side.
    fn style_for(&self, slot: BikeSlot) -> RenderStyle {
        RenderStyle::default().with_background_for_key(&self.bike(slot).active_frame_key)
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut state_dirty = false;

        // --- Title bar: app name, mode toggle, frame picker(s) ---
        egui::TopBottomPanel::top("title-bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("bike-fitter-1000");
                ui.separator();

                // Mode toggle.
                let mut mode = self.state.mode;
                ui.label("Mode:");
                if ui
                    .selectable_label(mode == AppMode::Single, "Single")
                    .clicked()
                {
                    mode = AppMode::Single;
                }
                if ui
                    .selectable_label(mode == AppMode::Compare, "Compare")
                    .clicked()
                {
                    mode = AppMode::Compare;
                }
                if mode != self.state.mode {
                    self.state.mode = mode;
                    // When entering Compare, default editing focus to A so
                    // the side-panel labels make sense.
                    if mode == AppMode::Compare {
                        self.editing_slot = BikeSlot::A;
                    }
                    state_dirty = true;
                }
                ui.separator();

                // Frame picker(s).
                if matches!(self.state.mode, AppMode::Single) {
                    if frame_picker(ui, "frame-picker-a", "Frame", &mut self.state.bike_a) {
                        state_dirty = true;
                    }
                } else {
                    if frame_picker(ui, "frame-picker-a", "A", &mut self.state.bike_a) {
                        state_dirty = true;
                    }
                    ui.separator();
                    if frame_picker(ui, "frame-picker-b", "B", &mut self.state.bike_b) {
                        state_dirty = true;
                    }
                }
            });
        });

        // --- Left side panel: frame editor (for one slot) + fit editor ---
        egui::SidePanel::left("frame-editor")
            .resizable(true)
            .default_width(280.0)
            .min_width(240.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    // In Compare mode, let the user pick which bike's frame
                    // they're editing. In Single mode, slot is forced to A.
                    if matches!(self.state.mode, AppMode::Compare) {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label("Editing:");
                            if ui
                                .selectable_label(self.editing_slot == BikeSlot::A, "Bike A")
                                .clicked()
                            {
                                self.editing_slot = BikeSlot::A;
                            }
                            if ui
                                .selectable_label(self.editing_slot == BikeSlot::B, "Bike B")
                                .clicked()
                            {
                                self.editing_slot = BikeSlot::B;
                            }
                        });
                        ui.separator();
                    } else {
                        self.editing_slot = BikeSlot::A;
                    }

                    let slot = self.editing_slot;

                    ui.add_space(4.0);
                    ui.heading("Frame");
                    ui.add_space(4.0);

                    if ui::frame_editor::show(ui, &mut self.bike_mut(slot).frame) {
                        state_dirty = true;
                    }

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let diverged = self.frame_diverged_from_preset(slot);
                        let btn = egui::Button::new("Reset to preset");
                        if ui.add_enabled(diverged, btn).clicked()
                            && self.reset_frame_to_preset(slot)
                        {
                            state_dirty = true;
                        }
                        if diverged {
                            ui.label(
                                egui::RichText::new("edited")
                                    .italics()
                                    .color(egui::Color32::from_rgb(180, 140, 80)),
                            );
                        }
                    });

                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(4.0);
                    ui.heading("Fit");
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("Shared between both bikes — same rider, same target.")
                            .small()
                            .weak(),
                    );
                    ui.add_space(4.0);

                    if ui::fit_editor::show(ui, &mut self.state.fit) {
                        state_dirty = true;
                    }
                });
            });

        // --- Right side panel: cockpit + solver, per bike ---
        egui::SidePanel::right("cockpit-and-solver")
            .resizable(true)
            .default_width(310.0)
            .min_width(270.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(4.0);
                    show_cockpit_and_solver(
                        ui,
                        if matches!(self.state.mode, AppMode::Compare) {
                            "Bike A"
                        } else {
                            "Cockpit"
                        },
                        &self.state.fit,
                        &mut self.state.bike_a,
                        &mut state_dirty,
                    );
                    if matches!(self.state.mode, AppMode::Compare) {
                        ui.add_space(16.0);
                        ui.separator();
                        ui.add_space(4.0);
                        show_cockpit_and_solver(
                            ui,
                            "Bike B",
                            &self.state.fit,
                            &mut self.state.bike_b,
                            &mut state_dirty,
                        );
                    }
                });
            });

        // --- Central panel: bike rendering(s) ---
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                let outer = ui.available_rect_before_wrap();
                match self.state.mode {
                    AppMode::Single => {
                        let style = self.style_for(BikeSlot::A);
                        paint_canvas_for(ui, outer, &style, &self.state.bike_a, &self.state.fit);
                    }
                    AppMode::Compare => {
                        // Split horizontally at the midpoint. A small gutter
                        // between the two halves keeps them visually separate.
                        let gutter = 2.0;
                        let mid = outer.center().x;
                        let left = egui::Rect::from_min_max(
                            outer.min,
                            egui::pos2(mid - gutter / 2.0, outer.max.y),
                        );
                        let right = egui::Rect::from_min_max(
                            egui::pos2(mid + gutter / 2.0, outer.min.y),
                            outer.max,
                        );
                        let style_a = self.style_for(BikeSlot::A);
                        let style_b = self.style_for(BikeSlot::B);
                        paint_canvas_for(ui, left, &style_a, &self.state.bike_a, &self.state.fit);
                        paint_canvas_for(ui, right, &style_b, &self.state.bike_b, &self.state.fit);
                    }
                }
            });

        if state_dirty {
            self.persist();
        }
    }
}

/// Combo box for picking a preset frame. Mutates `bike` in place; returns
/// `true` if the selection changed.
fn frame_picker(ui: &mut egui::Ui, id: &str, label: &str, bike: &mut Bike) -> bool {
    ui.label(format!("{label}:"));
    let presets = frames::all();
    let current_label = format!(
        "{} {} ({})",
        bike.frame.manufacturer, bike.frame.model, bike.frame.size_label,
    );
    let mut chosen: Option<String> = None;
    egui::ComboBox::from_id_salt(id)
        .selected_text(current_label)
        .width(260.0)
        .show_ui(ui, |ui| {
            for p in &presets {
                let entry = format!(
                    "{} {} ({})",
                    p.frame.manufacturer, p.frame.model, p.frame.size_label,
                );
                let selected = bike.active_frame_key == p.key;
                if ui.selectable_label(selected, entry).clicked() {
                    chosen = Some(p.key.clone());
                }
            }
        });
    if let Some(key) = chosen {
        if key != bike.active_frame_key {
            if let Some(p) = frames::by_key(&key) {
                bike.active_frame_key = p.key;
                bike.frame = p.frame;
                bike.cockpit = p
                    .default_cockpit
                    .unwrap_or_else(Cockpit::default_traditional);
                return true;
            }
        }
    }
    false
}

/// Render the right-panel "Cockpit + Solver" stack for one bike. Sets
/// `*dirty = true` if the user edited the cockpit.
fn show_cockpit_and_solver(
    ui: &mut egui::Ui,
    title: &str,
    fit: &FitProfile,
    bike: &mut Bike,
    dirty: &mut bool,
) {
    ui.heading(title);
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(format!(
            "{} {} ({})",
            bike.frame.manufacturer, bike.frame.model, bike.frame.size_label,
        ))
        .small()
        .weak(),
    );
    ui.add_space(6.0);

    // Cockpit-editor and solver-panel namespace their widget IDs internally
    // by id_salt — but two instances of the same panel collide. Push a scope
    // per bike so egui considers them distinct.
    ui.push_id(title, |ui| {
        if ui::cockpit_editor::show(ui, &mut bike.cockpit) {
            *dirty = true;
        }
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);
        ui.label(egui::RichText::new("Solver").strong());
        ui.add_space(2.0);
        ui::solver_panel::show(ui, &bike.frame, fit, &bike.cockpit);
    });
}

/// Draw one bike's canvas (frame + saddle/bar overlay + solver achieved
/// overlay) into `rect`, with `style.background` filling the rect.
fn paint_canvas_for(
    ui: &egui::Ui,
    rect: egui::Rect,
    style: &RenderStyle,
    bike: &Bike,
    fit: &FitProfile,
) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, style.background);
    render::paint_frame_with_overlay(
        &painter,
        rect,
        &bike.frame,
        Some(fit.saddle),
        Some(fit.bar_target),
        style,
    );
    if let Ok(build) = bike_fit_core::solver::solve_for_profile(&bike.frame, fit, &bike.cockpit) {
        render::paint_achieved_overlay(
            &painter,
            rect,
            &bike.frame,
            Some(fit.bar_target),
            build.achieved_bar_position,
            style,
        );
    }
}
