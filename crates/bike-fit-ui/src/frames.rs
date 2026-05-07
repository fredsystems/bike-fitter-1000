//! Bundled preset frames.
//!
//! The frame database lives in `data/bikes.json` at the repo root and is
//! compiled into the binary via `include_str!`. This is what makes the same
//! crate work identically for the native shell and the (future) wasm shell:
//! neither has to read from disk, both ship with the same library of bikes.
//!
//! Each preset has a stable `key` used as the persisted identifier. Once a
//! key has shipped, never rename it, or persisted user state will silently
//! lose the user's selection.
//!
//! The JSON is parsed once on first use and cached behind a `OnceLock`.

use std::sync::OnceLock;

use bike_fit_core::{Cockpit, Frame};
use serde::Deserialize;

/// One preset entry: a stable lookup key, the frame itself, and an optional
/// OEM-prescribed cockpit (e.g. the integrated bar/stem catalog the
/// manufacturer ships with the bike).
#[derive(Debug, Clone)]
pub struct Preset {
    pub key: String,
    pub frame: Frame,
    /// If `Some`, selecting this preset adopts this cockpit by default
    /// (overwriting any prior cockpit on the bike). If `None`, the bike
    /// falls back to [`Cockpit::default_traditional`].
    pub default_cockpit: Option<Cockpit>,
}

/// Returns the bundled preset list. Order is the order shown in pickers,
/// preserved from `data/bikes.json`.
pub fn all() -> Vec<Preset> {
    presets().to_vec()
}

/// Look up a preset by its stable key.
pub fn by_key(key: &str) -> Option<Preset> {
    presets().iter().find(|p| p.key == key).cloned()
}

// --- internals ---------------------------------------------------------------

/// Raw JSON shape for the bundled DB. Mirrors `data/bikes.json`.
#[derive(Debug, Deserialize)]
struct BikesFile {
    frames: Vec<RawPreset>,
}

#[derive(Debug, Deserialize)]
struct RawPreset {
    key: String,
    frame: Frame,
    #[serde(default)]
    default_cockpit: Option<Cockpit>,
}

const BIKES_JSON: &str = include_str!("../../../data/bikes.json");

fn presets() -> &'static [Preset] {
    static CELL: OnceLock<Vec<Preset>> = OnceLock::new();
    CELL.get_or_init(|| {
        let parsed: BikesFile = serde_json::from_str(BIKES_JSON)
            .expect("data/bikes.json is malformed; this is a build-time bug");
        // Sanity: keys must be unique. We assert at startup (rather than at
        // every `by_key`) so a typo in the JSON fails loudly during dev.
        let mut seen = std::collections::HashSet::new();
        for p in &parsed.frames {
            assert!(
                seen.insert(p.key.as_str()),
                "duplicate preset key in data/bikes.json: {}",
                p.key,
            );
        }
        parsed
            .frames
            .into_iter()
            .map(|r| Preset {
                key: r.key,
                frame: r.frame,
                default_cockpit: r.default_cockpit,
            })
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bike_fit_core::frame::WheelSize;

    #[test]
    fn bundled_json_parses() {
        let ps = all();
        assert!(!ps.is_empty(), "bundled bikes.json yielded zero presets");
    }

    #[test]
    fn keys_are_unique() {
        let presets = all();
        let mut keys: Vec<String> = presets.iter().map(|p| p.key.clone()).collect();
        keys.sort();
        let n = keys.len();
        keys.dedup();
        assert_eq!(keys.len(), n, "duplicate preset key");
    }

    #[test]
    fn by_key_round_trips() {
        for p in all() {
            let got = by_key(&p.key).expect("missing");
            assert_eq!(got.frame.manufacturer, p.frame.manufacturer);
            assert_eq!(got.frame.model, p.frame.model);
            assert_eq!(got.frame.size_label, p.frame.size_label);
        }
        assert!(by_key("not-a-real-key").is_none());
    }

    #[test]
    fn endurace_uses_650b_wheels() {
        let p = by_key("canyon-endurace-cf-slx-3xs-2025").expect("Endurace preset present");
        assert_eq!(p.frame.wheel_size, WheelSize::Iso584);
        // BSD 584 / 2 + 32 mm tire = 324 mm outer radius.
        let r = p.frame.wheel_outer_radius_mm();
        assert!((r - 324.0).abs() < 1e-9, "got {r}");
    }

    #[test]
    fn aeroad_matches_reference_numbers() {
        // The Aeroad 2XS preset is the reference frame from AGENTS.md §13;
        // the math unit tests in bike-fit-core depend on these numbers.
        // Catch any drift here too.
        let p = by_key("canyon-aeroad-cf-slx-7-2xs-2025").expect("Aeroad preset present");
        let f = &p.frame;
        assert_eq!(f.stack_mm, 498.0);
        assert_eq!(f.reach_mm, 372.0);
        assert_eq!(f.head_tube_angle_deg, 70.0);
        assert_eq!(f.front_center_horizontal_mm, Some(571.0));
        assert_eq!(f.wheel_size, WheelSize::Iso622);
    }

    #[test]
    fn missing_default_cockpit_is_none() {
        // Frames without a "default_cockpit" key in the JSON should leave
        // the field as None so the bike falls back to the global default.
        let p = by_key("canyon-aeroad-cf-slx-7-2xs-2025").expect("Aeroad preset present");
        assert!(p.default_cockpit.is_none());
    }

    #[test]
    fn tarmac_sl8_ships_with_integrated_cockpit() {
        let p = by_key("specialized-tarmac-sl8-44-2025").expect("Tarmac preset present");
        let cockpit = p
            .default_cockpit
            .as_ref()
            .expect("Tarmac SL8 should have a default integrated cockpit");
        match cockpit {
            bike_fit_core::Cockpit::Integrated { skus, .. } => {
                // 75..=135 in 5 mm steps = 13 SKUs.
                assert_eq!(skus.len(), 13);
                for sku in skus {
                    assert!((sku.angle_deg - -10.0).abs() < 1e-9);
                }
                assert!((skus.first().unwrap().length_mm - 75.0).abs() < 1e-9);
                assert!((skus.last().unwrap().length_mm - 135.0).abs() < 1e-9);
            }
            other => panic!("expected Integrated cockpit, got {other:?}"),
        }
    }
}
