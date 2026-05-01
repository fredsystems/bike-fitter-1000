//! Bundled preset frames.
//!
//! A small hand-curated library so the app has interesting bikes to draw out
//! of the box. The full DB will graduate to `data/bikes.json` at milestone 7;
//! this module is the staging ground.
//!
//! Each preset has a stable `key` used as the persisted identifier so we can
//! rename labels later without losing a user's selection.

use bike_fit_core::{frame::WheelSize, Frame};

/// One preset entry: a stable lookup key plus the frame itself.
#[derive(Debug, Clone)]
pub struct Preset {
    pub key: &'static str,
    pub frame: Frame,
}

/// Returns the bundled preset list. Order is the order shown in pickers.
pub fn all() -> Vec<Preset> {
    vec![
        Preset {
            key: "canyon-aeroad-cf-slx-7-2xs-2025",
            frame: aeroad_2xs_2025(),
        },
        Preset {
            key: "canyon-endurace-cf-slx-3xs-2025",
            frame: endurace_3xs_2025(),
        },
        Preset {
            key: "specialized-tarmac-sl8-m-2025",
            frame: tarmac_sl8_m_2025(),
        },
    ]
}

/// Look up a preset by its stable key.
pub fn by_key(key: &str) -> Option<Preset> {
    all().into_iter().find(|p| p.key == key)
}

/// Reference frame from `AGENTS.md` §13. The same numbers used by the math
/// unit tests.
pub fn aeroad_2xs_2025() -> Frame {
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

/// Canyon Endurace CF SLX 3XS, 2025.
///
/// Note: the published TT length is "Effective/Horizontal HT-Top" (501 mm);
/// we record it under `top_tube_effective_mm` even though our nominal
/// convention is "Center". TT length isn't used by any geometry math (it's
/// redundant with reach/stack/HTA/STA), so the convention mismatch is
/// cosmetic — the side-view rendering uses the derived points instead.
///
/// Wheel: 650B / 27.5" (BSD 584). Tire 32 mm. Geometry that actually drives
/// the renderer (stack, reach, HTA/STA, HTL/STL, BB drop, chainstay,
/// front-center-horizontal, fork rake) is taken straight from the published
/// numbers.
pub fn endurace_3xs_2025() -> Frame {
    Frame {
        manufacturer: "Canyon".into(),
        model: "Endurace CF SLX".into(),
        size_label: "3XS".into(),
        year: Some(2025),
        stack_mm: 510.0,
        reach_mm: 350.0,
        head_tube_angle_deg: 70.3,
        head_tube_length_mm: 123.0,
        seat_tube_angle_deg: 73.5,
        seat_tube_length_mm: 402.0,
        top_tube_effective_mm: 501.0,
        bb_drop_mm: 60.0,
        chainstay_mm: 405.0,
        fork_rake_mm: 44.6,
        front_center_horizontal_mm: Some(558.5),
        wheel_size: WheelSize::Iso584,
        tire_width_mm: 32.0,
    }
}

/// Specialized Tarmac SL8 size M, 2025.
pub fn tarmac_sl8_m_2025() -> Frame {
    Frame {
        manufacturer: "Specialized".into(),
        model: "Tarmac SL8".into(),
        size_label: "M".into(),
        year: Some(2025),
        stack_mm: 501.0,
        reach_mm: 366.0,
        head_tube_angle_deg: 70.5,
        head_tube_length_mm: 99.0,
        seat_tube_angle_deg: 75.5,
        seat_tube_length_mm: 433.0,
        top_tube_effective_mm: 496.0,
        bb_drop_mm: 74.0,
        chainstay_mm: 410.0,
        fork_rake_mm: 46.7,
        front_center_horizontal_mm: Some(566.7),
        wheel_size: WheelSize::Iso622,
        tire_width_mm: 26.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_unique() {
        let presets = all();
        let mut keys: Vec<&str> = presets.iter().map(|p| p.key).collect();
        keys.sort_unstable();
        let n = keys.len();
        keys.dedup();
        assert_eq!(keys.len(), n, "duplicate preset key");
    }

    #[test]
    fn by_key_round_trips() {
        for p in all() {
            let got = by_key(p.key).expect("missing");
            assert_eq!(got.frame.manufacturer, p.frame.manufacturer);
            assert_eq!(got.frame.model, p.frame.model);
            assert_eq!(got.frame.size_label, p.frame.size_label);
        }
        assert!(by_key("not-a-real-key").is_none());
    }

    #[test]
    fn endurace_uses_650b_wheels() {
        let f = endurace_3xs_2025();
        assert_eq!(f.wheel_size, WheelSize::Iso584);
        // BSD 584 / 2 + 32 mm tire = 324 mm outer radius.
        let r = f.wheel_outer_radius_mm();
        assert!((r - 324.0).abs() < 1e-9, "got {r}");
    }
}
