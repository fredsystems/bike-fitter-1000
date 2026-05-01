//! Wasm entrypoint for bike-fitter-1000.
//!
//! This crate compiles to a `cdylib` that `trunk` packages with the static
//! HTML/CSS in this directory. It contains:
//!
//! - The `start` function exported via `wasm-bindgen`, which trunk's
//!   auto-generated bootstrap calls once the page loads.
//! - [`LocalStoragePersistence`], the [`bike_fit_ui::Persistence`] impl
//!   backed by `window.localStorage`.
//! - URL-fragment decoding so a self-contained `#fit=<base64>` link can
//!   seed the app with a shared fit profile (no backend required).
//!
//! Everything below is `cfg(target_arch = "wasm32")`-gated. On host
//! targets the lib compiles to an empty crate so `cargo test --workspace`
//! works without dragging in browser-only `web-sys` symbols.

#![cfg(target_arch = "wasm32")]

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use bike_fit_ui::{App, AppState, Persistence};
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::window;

/// Key used in `localStorage` for the persisted [`AppState`] JSON blob.
const STORAGE_KEY: &str = "bike-fitter-1000.state";

/// URL fragment parameter that, when present, seeds the app with a shared
/// fit instead of loading from `localStorage`. Format: `#fit=<base64-url>`
/// where the decoded bytes are the JSON of an [`AppState`].
const FRAGMENT_PARAM: &str = "fit";

/// `localStorage`-backed [`Persistence`].
///
/// Reads and writes are best-effort: if the browser denies storage access
/// (e.g. private mode quotas, third-party context) we log and move on.
/// Saves are deduplicated against the last value we wrote so the per-frame
/// `save()` call doesn't pound `JSON.stringify` and the storage API.
pub struct LocalStoragePersistence {
    last: std::cell::RefCell<Option<AppState>>,
}

impl LocalStoragePersistence {
    pub fn new() -> Self {
        Self {
            last: std::cell::RefCell::new(None),
        }
    }

    fn storage(&self) -> Option<web_sys::Storage> {
        window().and_then(|w| w.local_storage().ok().flatten())
    }
}

impl Default for LocalStoragePersistence {
    fn default() -> Self {
        Self::new()
    }
}

impl Persistence for LocalStoragePersistence {
    fn load(&self) -> Option<AppState> {
        // 1. Fragment seed wins over localStorage. Sharing is meant to be a
        //    one-click way to demonstrate someone else's fit; we don't want
        //    a stale localStorage value silently overriding the link.
        if let Some(state) = load_from_fragment() {
            // Cache it so we don't re-write the same value back on the
            // first auto-save tick.
            *self.last.borrow_mut() = Some(state.clone());
            return Some(state);
        }
        let storage = self.storage()?;
        let raw = storage.get_item(STORAGE_KEY).ok().flatten()?;
        match serde_json::from_str::<AppState>(&raw) {
            Ok(s) => {
                *self.last.borrow_mut() = Some(s.clone());
                Some(s)
            }
            Err(e) => {
                log::warn!("ignoring malformed localStorage state: {e}");
                None
            }
        }
    }

    fn save(&self, state: &AppState) {
        if self.last.borrow().as_ref() == Some(state) {
            return;
        }
        let Some(storage) = self.storage() else {
            return;
        };
        match serde_json::to_string(state) {
            Ok(json) => {
                if let Err(e) = storage.set_item(STORAGE_KEY, &json) {
                    log::warn!("localStorage.setItem failed: {e:?}");
                } else {
                    *self.last.borrow_mut() = Some(state.clone());
                }
            }
            Err(e) => log::warn!("serialize state failed: {e}"),
        }
    }
}

/// Inspect `window.location.hash` for `#fit=<base64-url>` and try to decode
/// it into an [`AppState`]. Returns `None` if the fragment is absent,
/// malformed, or doesn't deserialize.
fn load_from_fragment() -> Option<AppState> {
    let hash = window()?.location().hash().ok()?;
    // hash is "" or "#..." — strip the leading '#' if present.
    let hash = hash.strip_prefix('#').unwrap_or(&hash);
    if hash.is_empty() {
        return None;
    }
    // Hash payloads are key=value pairs joined by '&', same shape as a
    // query string. We only care about FRAGMENT_PARAM.
    for pair in hash.split('&') {
        let Some((k, v)) = pair.split_once('=') else {
            continue;
        };
        if k != FRAGMENT_PARAM {
            continue;
        }
        match URL_SAFE_NO_PAD.decode(v) {
            Ok(bytes) => match serde_json::from_slice::<AppState>(&bytes) {
                Ok(state) => {
                    log::info!("seeded AppState from URL fragment");
                    return Some(state);
                }
                Err(e) => log::warn!("URL fragment present but JSON parse failed: {e}"),
            },
            Err(e) => log::warn!("URL fragment present but base64 decode failed: {e}"),
        }
    }
    None
}

/// `wasm-bindgen` entrypoint. Trunk's generated bootstrap calls this once
/// the wasm module finishes loading.
///
/// Expects an `<canvas id="bike-fitter-canvas"></canvas>` element on the
/// host page; that ID is wired via `index.html`.
#[wasm_bindgen]
pub async fn start() -> Result<(), JsValue> {
    // Without these two, panics surface as an unhelpful "unreachable executed"
    // and `log::*` calls vanish into the void.
    console_error_panic_hook::set_once();
    // info-level by default; loud crates in release builds can be filtered
    // by adding ?log=warn to the URL later if it becomes a problem.
    let _ = console_log::init_with_level(log::Level::Info);

    let document = web_sys::window()
        .and_then(|w| w.document())
        .ok_or_else(|| JsValue::from_str("no document on window"))?;
    let canvas = document
        .get_element_by_id("bike-fitter-canvas")
        .ok_or_else(|| JsValue::from_str("canvas#bike-fitter-canvas not found"))?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| JsValue::from_str("element is not a <canvas>"))?;

    let web_options = eframe::WebOptions::default();
    let runner = eframe::WebRunner::new();
    runner
        .start(
            canvas,
            web_options,
            Box::new(|_cc| {
                let persistence: Arc<dyn Persistence> = Arc::new(LocalStoragePersistence::new());
                Ok(Box::new(App::new(persistence)))
            }),
        )
        .await?;
    Ok(())
}
