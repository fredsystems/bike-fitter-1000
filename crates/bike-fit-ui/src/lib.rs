//! Shared egui UI for bike-fitter-1000.
//!
//! This crate contains the [`App`] (an [`eframe::App`] implementer), the
//! side-view renderer, and panel components. It is target-agnostic: both the
//! native `bike-fit-app` binary and (eventually) the wasm `bike-fit-web`
//! crate hand it a [`Persistence`] impl and let it run.

pub mod render;

use std::sync::Arc;

use bike_fit_core::{frame::WheelSize, Cockpit, FitProfile, Frame, Point};
use eframe::egui;

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
    pub frame: Frame,
    pub fit: FitProfile,
    pub cockpit: Cockpit,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            frame: default_demo_frame(),
            fit: FitProfile::new(
                "rider-1",
                Point::new(-50.0, 730.0),
                Point::new(420.0, 580.0),
            ),
            cockpit: Cockpit::default_traditional(),
        }
    }
}

/// A reasonable starter frame so the UI has something to draw on first run.
/// Mirrors the Aeroad 2XS reference frame from `AGENTS.md` §13.
pub fn default_demo_frame() -> Frame {
    Frame {
        manufacturer: "Canyon".into(),
        model: "Aeroad CF SLX 7".into(),
        size_label: "2XS".into(),
        year: Some(2025),
        stack_mm: 498.0,
        reach_mm: 372.0,
        head_tube_angle_deg: 70.0,
        head_tube_length_mm: 88.0,
        seat_tube_angle_deg: 73.5,
        seat_tube_length_mm: 441.0,
        top_tube_effective_mm: 516.0,
        bb_drop_mm: 70.0,
        chainstay_mm: 410.0,
        fork_rake_mm: 40.6,
        front_center_horizontal_mm: Some(571.0),
        wheel_size: WheelSize::Iso622,
        tire_width_mm: 28.0,
    }
}

/// The shared application — an [`eframe::App`] that doesn't know whether it's
/// running on the desktop or in a browser.
pub struct App {
    state: AppState,
    // Wired up now; will be invoked from real edits in milestones 4–6.
    #[allow(dead_code)]
    persistence: Arc<dyn Persistence>,
    style: RenderStyle,
}

impl App {
    pub fn new(persistence: Arc<dyn Persistence>) -> Self {
        let state = persistence.load().unwrap_or_default();
        Self {
            state,
            persistence,
            style: RenderStyle::default(),
        }
    }

    #[allow(dead_code)]
    fn save(&self) {
        self.persistence.save(&self.state);
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("title-bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("bike-fitter-1000");
                ui.separator();
                ui.label(format!(
                    "{} {} ({})",
                    self.state.frame.manufacturer,
                    self.state.frame.model,
                    self.state.frame.size_label,
                ));
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // Render the frame side-view filling the central panel.
            let painter_rect = ui.available_rect_before_wrap();
            let painter = ui.painter_at(painter_rect);
            render::paint_frame_with_overlay(
                &painter,
                painter_rect,
                &self.state.frame,
                Some(self.state.fit.saddle),
                Some(self.state.fit.bar_target),
                &self.style,
            );
        });

        // Persistence is wired up but won't write until something actually
        // edits state — keeping the per-frame save call at no cost. We'll hook
        // it from real edits in milestones 4–6.
    }
}
