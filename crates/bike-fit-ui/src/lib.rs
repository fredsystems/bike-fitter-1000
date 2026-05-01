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
pub use render::{paint_frame, paint_frame_with_overlay, show_frame, RenderStyle};

/// Pluggable storage backend.
///
/// Native uses a JSON file on disk; web will use `localStorage`. Both go
/// through this trait so the rest of the UI doesn't care.
pub trait Persistence: Send + Sync + 'static {
    /// Load the persisted state, if any.
    fn load(&self) -> Option<AppState>;
    /// Persist the state. Errors are logged and otherwise swallowed; this
    /// fires often (every interactive change), so transient I/O failures
    /// shouldn't crash the UI.
    fn save(&self, state: &AppState);
}

/// In-memory persistent state. Plain data, no UI.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AppState {
    /// Stable identifier for the active frame. Looked up against
    /// [`frames::all`]; falls back to the first preset if the key is unknown
    /// (e.g. if a user-edited `frame` was persisted under a key we no longer
    /// know about, but the inline `frame` field is still authoritative).
    pub active_frame_key: String,
    /// Authoritative active frame. Initially populated from a preset, but
    /// edits in the frame editor (milestone 4) write into this field
    /// directly, decoupling it from the preset library.
    pub frame: Frame,
    pub fit: FitProfile,
    pub cockpit: Cockpit,
}

impl Default for AppState {
    fn default() -> Self {
        let presets = frames::all();
        let first = presets.into_iter().next().expect("at least one preset");
        Self {
            active_frame_key: first.key.to_string(),
            frame: first.frame,
            fit: FitProfile::new(
                "rider-1",
                Point::new(-50.0, 730.0),
                Point::new(420.0, 580.0),
            ),
            cockpit: Cockpit::default_traditional(),
        }
    }
}

/// The shared application — an [`eframe::App`] that doesn't know whether it's
/// running on the desktop or in a browser.
pub struct App {
    state: AppState,
    persistence: Arc<dyn Persistence>,
}

impl App {
    pub fn new(persistence: Arc<dyn Persistence>) -> Self {
        let state = persistence.load().unwrap_or_default();
        Self { state, persistence }
    }

    fn persist(&self) {
        self.persistence.save(&self.state);
    }

    /// Replace the active frame with the named preset, if it exists.
    fn select_preset(&mut self, key: &str) {
        if let Some(p) = frames::by_key(key) {
            self.state.active_frame_key = p.key.to_string();
            self.state.frame = p.frame;
        }
    }

    /// Re-apply the active preset to the live frame, discarding any local
    /// edits. No-op if the active key isn't in the preset list (e.g. for a
    /// frame the user fully authored).
    fn reset_frame_to_preset(&mut self) -> bool {
        if let Some(p) = frames::by_key(&self.state.active_frame_key) {
            if self.state.frame != p.frame {
                self.state.frame = p.frame;
                return true;
            }
        }
        false
    }

    /// Has the live frame diverged from its preset?
    fn frame_diverged_from_preset(&self) -> bool {
        frames::by_key(&self.state.active_frame_key).is_some_and(|p| p.frame != self.state.frame)
    }

    /// Render style for the currently-active frame: default style tinted by
    /// the active key so each preset has a visibly distinct background.
    fn current_style(&self) -> RenderStyle {
        RenderStyle::default().with_background_for_key(&self.state.active_frame_key)
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut state_dirty = false;

        egui::TopBottomPanel::top("title-bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("bike-fitter-1000");
                ui.separator();
                ui.label("Frame:");

                let presets = frames::all();
                let current_label = preset_label(&self.state);
                let mut chosen: Option<String> = None;
                egui::ComboBox::from_id_salt("frame-picker")
                    .selected_text(current_label)
                    .width(280.0)
                    .show_ui(ui, |ui| {
                        for p in &presets {
                            let label = format!(
                                "{} {} ({})",
                                p.frame.manufacturer, p.frame.model, p.frame.size_label,
                            );
                            let selected = self.state.active_frame_key == p.key;
                            if ui.selectable_label(selected, label).clicked() {
                                chosen = Some(p.key.to_string());
                            }
                        }
                    });
                if let Some(key) = chosen {
                    if key != self.state.active_frame_key {
                        self.select_preset(&key);
                        state_dirty = true;
                    }
                }
            });
        });

        let style = self.current_style();

        egui::SidePanel::left("frame-editor")
            .resizable(true)
            .default_width(280.0)
            .min_width(240.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(4.0);
                    ui.heading("Frame");
                    ui.add_space(4.0);

                    if ui::frame_editor::show(ui, &mut self.state.frame) {
                        state_dirty = true;
                    }

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let diverged = self.frame_diverged_from_preset();
                        let btn = egui::Button::new("Reset to preset");
                        if ui.add_enabled(diverged, btn).clicked() && self.reset_frame_to_preset() {
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

                    if ui::fit_editor::show(ui, &mut self.state.fit) {
                        state_dirty = true;
                    }
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(style.background))
            .show(ctx, |ui| {
                let painter_rect = ui.available_rect_before_wrap();
                let painter = ui.painter_at(painter_rect);
                render::paint_frame_with_overlay(
                    &painter,
                    painter_rect,
                    &self.state.frame,
                    Some(self.state.fit.saddle),
                    Some(self.state.fit.bar_target),
                    &style,
                );
            });

        if state_dirty {
            self.persist();
        }
    }
}

fn preset_label(state: &AppState) -> String {
    format!(
        "{} {} ({})",
        state.frame.manufacturer, state.frame.model, state.frame.size_label,
    )
}
