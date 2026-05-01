//! Cockpit placement geometry: spacer stack and stem clamp position.
//!
//! Given a [`Frame`] and a chosen spacer stack height + stem (length and
//! angle), compute the position of the stem's far clamp face (the "bar
//! target point" — see `AGENTS.md` §4.1).

use crate::{Frame, Point, Stem};

/// Position of the stem clamp on the steerer, just above the spacer stack.
///
/// `top_cap_mm` is the headset top-cap thickness (typically ~5 mm) and
/// `spacer_total_mm` is the total spacer stack height in millimeters.
pub fn stem_clamp_origin(frame: &Frame, top_cap_mm: f64, spacer_total_mm: f64) -> Point {
    frame.top_of_head_tube() + frame.steerer_up_unit() * (top_cap_mm + spacer_total_mm)
}

/// Stem direction unit vector for the given stem.
///
/// `stem.angle_deg` is measured relative to the line perpendicular to the
/// steerer (industry convention). Positive angle rises above the
/// perpendicular, negative drops below it.
pub fn stem_direction(frame: &Frame, stem: Stem) -> Point {
    frame
        .forward_perpendicular_to_steerer_unit()
        .rotated(stem.angle_deg)
}

/// Position of the stem's far clamp face (the "bar target point") for a
/// given spacer stack and stem.
pub fn stem_clamp_face(frame: &Frame, top_cap_mm: f64, spacer_total_mm: f64, stem: Stem) -> Point {
    stem_clamp_origin(frame, top_cap_mm, spacer_total_mm)
        + stem_direction(frame, stem) * stem.length_mm
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::WheelSize;

    fn aeroad_2xs() -> Frame {
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

    fn approx(a: f64, b: f64, tol: f64) {
        assert!(
            (a - b).abs() < tol,
            "expected {a} ≈ {b}, diff {}",
            (a - b).abs()
        );
    }

    #[test]
    fn zero_spacers_zero_top_cap_lands_on_top_of_ht() {
        let f = aeroad_2xs();
        let origin = stem_clamp_origin(&f, 0.0, 0.0);
        approx(origin.x, f.reach_mm, 1e-9);
        approx(origin.y, f.stack_mm, 1e-9);
    }

    #[test]
    fn spacers_move_clamp_along_steerer() {
        let f = aeroad_2xs();
        let zero = stem_clamp_origin(&f, 0.0, 0.0);
        let with_30 = stem_clamp_origin(&f, 0.0, 30.0);
        // Should differ by exactly 30 mm along the steerer-up direction.
        approx(zero.distance_to(with_30), 30.0, 1e-9);
        let dir = (with_30 - zero) * (1.0 / 30.0);
        let up = f.steerer_up_unit();
        approx(dir.x, up.x, 1e-9);
        approx(dir.y, up.y, 1e-9);
    }

    #[test]
    fn top_cap_and_spacers_compose_additively() {
        let f = aeroad_2xs();
        let combined = stem_clamp_origin(&f, 5.0, 30.0);
        let single = stem_clamp_origin(&f, 0.0, 35.0);
        approx(combined.x, single.x, 1e-9);
        approx(combined.y, single.y, 1e-9);
    }

    #[test]
    fn stem_zero_angle_is_perpendicular_to_steerer() {
        let f = aeroad_2xs();
        let stem = Stem {
            length_mm: 100.0,
            angle_deg: 0.0,
        };
        let dir = stem_direction(&f, stem);
        // Should equal the forward-perpendicular unit vector exactly.
        let fwd = f.forward_perpendicular_to_steerer_unit();
        approx(dir.x, fwd.x, 1e-12);
        approx(dir.y, fwd.y, 1e-12);
        // And it should be perpendicular to steerer_up.
        approx(dir.dot(f.steerer_up_unit()), 0.0, 1e-12);
    }

    #[test]
    fn stem_clamp_face_distance_equals_stem_length() {
        let f = aeroad_2xs();
        let stem = Stem {
            length_mm: 100.0,
            angle_deg: -6.0,
        };
        let origin = stem_clamp_origin(&f, 5.0, 20.0);
        let face = stem_clamp_face(&f, 5.0, 20.0, stem);
        approx(origin.distance_to(face), stem.length_mm, 1e-9);
    }

    #[test]
    fn negative_stem_angle_drops_clamp_face_lower() {
        // A -6° stem should produce a lower y for its clamp face than a +6°
        // stem given identical spacer stack and length.
        let f = aeroad_2xs();
        let common = (5.0, 20.0); // top_cap, spacer_total
        let down = stem_clamp_face(
            &f,
            common.0,
            common.1,
            Stem {
                length_mm: 100.0,
                angle_deg: -6.0,
            },
        );
        let up = stem_clamp_face(
            &f,
            common.0,
            common.1,
            Stem {
                length_mm: 100.0,
                angle_deg: 6.0,
            },
        );
        assert!(down.y < up.y, "down.y = {}, up.y = {}", down.y, up.y);
    }
}
