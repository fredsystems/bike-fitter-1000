//! bike-fitter-1000 — native GUI entry point.
//!
//! This binary is a thin shell: it wires up logging, the eframe window, and a
//! file-backed [`Persistence`] impl, then hands everything off to the shared
//! [`bike_fit_ui::App`].

#![warn(clippy::all)]

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use bike_fit_ui::{App, AppState, Persistence};
use eframe::egui;

fn main() -> eframe::Result<()> {
    // Default filter: our crates at info, the graphics stack at warn (wgpu's
    // per-frame `Device::maintain: waiting for submission index N` log at
    // info level is too chatty for an interactive app). `RUST_LOG` overrides.
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(
        "info,wgpu=warn,wgpu_core=warn,wgpu_hal=warn,naga=warn,eframe=warn,egui_wgpu=warn",
    ))
    .init();

    let persistence: Arc<dyn Persistence> = Arc::new(FilePersistence::new(state_file_path()));

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("bike-fitter-1000"),
        ..Default::default()
    };

    eframe::run_native(
        "bike-fitter-1000",
        native_options,
        Box::new(move |_cc| Ok(Box::new(App::new(persistence)))),
    )
}

/// Where to persist app state on this machine.
///
/// Uses `$XDG_STATE_HOME/bike-fitter-1000/state.json` (or
/// `$HOME/.local/state/...` per the XDG spec) on Linux, and best-effort
/// equivalents elsewhere.
fn state_file_path() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_STATE_HOME") {
        return PathBuf::from(dir)
            .join("bike-fitter-1000")
            .join("state.json");
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".local")
            .join("state")
            .join("bike-fitter-1000")
            .join("state.json");
    }
    // Fallback: alongside the executable.
    PathBuf::from("bike-fitter-1000-state.json")
}

/// JSON-on-disk persistence. Reads on startup, writes on every save call.
///
/// The mutex serializes the rare-but-possible re-entrant write; in practice
/// `save` is only called from the UI thread.
struct FilePersistence {
    path: PathBuf,
    last: Mutex<Option<AppState>>,
}

impl FilePersistence {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            last: Mutex::new(None),
        }
    }
}

impl Persistence for FilePersistence {
    fn load(&self) -> Option<AppState> {
        let bytes = std::fs::read(&self.path).ok()?;
        match serde_json::from_slice::<AppState>(&bytes) {
            Ok(s) => {
                if let Ok(mut g) = self.last.lock() {
                    *g = Some(s.clone());
                }
                Some(s)
            }
            Err(e) => {
                log::warn!("ignoring malformed state at {}: {e}", self.path.display());
                None
            }
        }
    }

    fn save(&self, state: &AppState) {
        // Skip the write if the state hasn't changed; this gets called every
        // frame today.
        if let Ok(mut g) = self.last.lock() {
            if g.as_ref() == Some(state) {
                return;
            }
            *g = Some(state.clone());
        }
        if let Some(parent) = self.path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                log::warn!("could not create state dir {}: {e}", parent.display());
                return;
            }
        }
        let json = match serde_json::to_vec_pretty(state) {
            Ok(j) => j,
            Err(e) => {
                log::warn!("could not serialize state: {e}");
                return;
            }
        };
        if let Err(e) = std::fs::write(&self.path, json) {
            log::warn!("could not write state to {}: {e}", self.path.display());
        }
    }
}
