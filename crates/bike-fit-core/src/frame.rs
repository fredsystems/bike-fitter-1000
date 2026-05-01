//! Bike frame geometry.
//!
//! A [`Frame`] holds the manufacturer-published geometry numbers and exposes
//! derived points in BB-relative coordinates. See `docs/geometry-math.md`
//! for the formulas.

use serde::{Deserialize, Serialize};

use crate::Point;

/// Wheel size, expressed by bead-seat diameter (BSD) in millimeters.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum WheelSize {
    /// 700C / 29" — 622 mm BSD.
    Iso622,
    /// 650B / 27.5" — 584 mm BSD.
    Iso584,
    /// Custom BSD in millimeters.
    Custom(u16),
}

impl WheelSize {
    pub fn bsd_mm(self) -> f64 {
        match self {
            Self::Iso622 => 622.0,
            Self::Iso584 => 584.0,
            Self::Custom(b) => f64::from(b),
        }
    }
}

/// Manufacturer-published frame geometry.
///
/// Field names follow the conventions used on geometrygeeks.bike /
/// bike-insights. Angles are measured from the ground (industry standard).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    pub manufacturer: String,
    pub model: String,
    pub size_label: String,
    pub year: Option<u16>,

    /// Vertical distance from BB center to top of head tube (mm).
    pub stack_mm: f64,
    /// Horizontal distance from BB center to top of head tube (mm).
    pub reach_mm: f64,

    /// Head tube angle, measured from the ground (deg).
    pub head_tube_angle_deg: f64,
    /// Head tube length (mm).
    pub head_tube_length_mm: f64,

    /// Seat tube angle (effective), measured from the ground (deg).
    pub seat_tube_angle_deg: f64,
    /// Seat tube length, BB to top of seat tube along the seat tube axis (mm).
    pub seat_tube_length_mm: f64,
    /// Effective (horizontal) top tube length (mm). Informational only;
    /// not used by the solver.
    pub top_tube_effective_mm: f64,

    /// Bottom bracket drop — vertical distance the BB sits below the wheel
    /// axle line (mm). Positive means BB is below the axles.
    pub bb_drop_mm: f64,
    /// Chainstay length, BB center to rear axle along the stay (mm).
    pub chainstay_mm: f64,
    /// Fork rake / offset (mm). Currently informational; the front axle is
    /// taken from `front_center_horizontal_mm` when available.
    pub fork_rake_mm: f64,

    /// Horizontal distance from BB to front axle, when published. If `None`,
    /// front-axle position is approximated from HTA + fork rake (less
    /// accurate; not all manufacturers publish axle-to-crown).
    pub front_center_horizontal_mm: Option<f64>,

    pub wheel_size: WheelSize,
    /// Tire width in millimeters (used to compute outer wheel radius).
    pub tire_width_mm: f64,
}

impl Frame {
    /// Outer radius of the tire-on-rim assembly (mm).
    ///
    /// Approximation: `BSD/2 + tire_width`. Tires aren't perfect circles and
    /// real-world OD depends on rim width and pressure, but this is within a
    /// few mm and matches what most geometry charts assume.
    pub fn wheel_outer_radius_mm(&self) -> f64 {
        self.wheel_size.bsd_mm() / 2.0 + self.tire_width_mm
    }

    /// Top of the head tube — the point at which the steerer enters the
    /// frame from above. By definition: `(reach, stack)`.
    pub fn top_of_head_tube(&self) -> Point {
        Point::new(self.reach_mm, self.stack_mm)
    }

    /// Bottom of the head tube. The head tube goes down-and-forward from the
    /// top along an axis at angle `HTA` from the ground.
    pub fn bottom_of_head_tube(&self) -> Point {
        let hta = self.head_tube_angle_deg.to_radians();
        let (s, c) = hta.sin_cos();
        self.top_of_head_tube() + Point::new(c, -s) * self.head_tube_length_mm
    }

    /// "Up along the steerer" unit vector. Points up and slightly back, since
    /// modern frames lean the head tube backward at the top.
    pub fn steerer_up_unit(&self) -> Point {
        let hta = self.head_tube_angle_deg.to_radians();
        let (s, c) = hta.sin_cos();
        Point::new(-c, s)
    }

    /// Unit vector perpendicular to the steerer, pointing forward (toward the
    /// front of the bike). This is the direction a stem at angle 0° would
    /// point.
    pub fn forward_perpendicular_to_steerer_unit(&self) -> Point {
        let hta = self.head_tube_angle_deg.to_radians();
        let (s, c) = hta.sin_cos();
        Point::new(s, c)
    }

    /// Top of the seat tube. The seat tube goes up-and-back from the BB along
    /// an axis at angle `STA` from the ground.
    pub fn top_of_seat_tube(&self) -> Point {
        let sta = self.seat_tube_angle_deg.to_radians();
        let (s, c) = sta.sin_cos();
        Point::new(-c, s) * self.seat_tube_length_mm
    }

    /// Rear axle position. Derived from chainstay (hypotenuse) and BB drop
    /// (vertical leg).
    pub fn rear_axle(&self) -> Point {
        let cs2 = self.chainstay_mm.powi(2);
        let drop2 = self.bb_drop_mm.powi(2);
        let horizontal = (cs2 - drop2).max(0.0).sqrt();
        Point::new(-horizontal, self.bb_drop_mm)
    }

    /// Front axle position. Uses published `front_center_horizontal_mm` when
    /// available; otherwise approximates from HTA + fork rake by projecting
    /// the steerer line down to axle height and offsetting perpendicular by
    /// the fork rake.
    pub fn front_axle(&self) -> Point {
        if let Some(fc) = self.front_center_horizontal_mm {
            return Point::new(fc, self.bb_drop_mm);
        }
        // Fallback approximation. The steerer continues down through the
        // bottom of the head tube along the head-tube axis. The fork's
        // dropout sits offset from that axis by `fork_rake_mm`, perpendicular
        // to the steerer in the forward direction.
        //
        // Without axle-to-crown we can't pin the dropout's position along the
        // steerer line, so we project to the wheel-axle height (y =
        // bb_drop_mm) along the steerer and then add the rake offset.
        let bottom = self.bottom_of_head_tube();
        let down_axis = -self.steerer_up_unit();
        // Distance along the steerer to drop from bottom_of_HT.y to axle_y.
        // axle_y = bottom.y + t * down_axis.y, solve for t.
        let dy = self.bb_drop_mm - bottom.y;
        let t = if down_axis.y.abs() > 1e-9 {
            dy / down_axis.y
        } else {
            0.0
        };
        let on_steerer = bottom + down_axis * t;
        let rake_dir = self.forward_perpendicular_to_steerer_unit();
        on_steerer + rake_dir * self.fork_rake_mm
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reference frame from `AGENTS.md`: a 2025 Canyon Aeroad CF SLX 7,
    /// size 2XS. Used for hand-checking the math.
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
            "expected {a} ≈ {b} (tol {tol}, diff {})",
            (a - b).abs()
        );
    }

    #[test]
    fn top_of_head_tube_is_reach_stack() {
        let f = aeroad_2xs();
        let p = f.top_of_head_tube();
        approx(p.x, 372.0, 1e-9);
        approx(p.y, 498.0, 1e-9);
    }

    #[test]
    fn rear_axle_horizontal_matches_published() {
        // Published "Chainstay Length Horizontal" = 404 mm.
        // sqrt(410² − 70²) ≈ 403.98.
        let f = aeroad_2xs();
        let axle = f.rear_axle();
        approx(axle.x, -403.98, 0.05);
        approx(axle.y, 70.0, 1e-9);
    }

    #[test]
    fn front_axle_uses_published_front_center() {
        let f = aeroad_2xs();
        let axle = f.front_axle();
        approx(axle.x, 571.0, 1e-9);
        approx(axle.y, 70.0, 1e-9);
    }

    #[test]
    fn wheel_outer_radius_aero_28mm() {
        // 622 BSD / 2 + 28 = 339 mm.
        let f = aeroad_2xs();
        approx(f.wheel_outer_radius_mm(), 339.0, 1e-9);
    }

    #[test]
    fn steerer_up_at_70_deg_is_mostly_up() {
        let f = aeroad_2xs();
        let u = f.steerer_up_unit();
        // (-cos 70°, sin 70°) ≈ (-0.342, 0.940). Should be unit-length.
        approx((u.x * u.x + u.y * u.y).sqrt(), 1.0, 1e-12);
        approx(u.x, -0.342_020_143, 1e-6);
        approx(u.y, 0.939_692_620, 1e-6);
    }

    #[test]
    fn forward_perp_is_steerer_up_rotated_minus_90() {
        // forward_perp should equal steerer_up rotated 90° clockwise.
        let f = aeroad_2xs();
        let up = f.steerer_up_unit();
        let fwd = f.forward_perpendicular_to_steerer_unit();
        let rotated = up.perpendicular_cw();
        approx(fwd.x, rotated.x, 1e-12);
        approx(fwd.y, rotated.y, 1e-12);
    }

    #[test]
    fn head_tube_endpoints_are_one_htl_apart() {
        let f = aeroad_2xs();
        let top = f.top_of_head_tube();
        let bottom = f.bottom_of_head_tube();
        approx(top.distance_to(bottom), f.head_tube_length_mm, 1e-9);
    }

    #[test]
    fn top_of_seat_tube_is_one_stl_from_bb() {
        let f = aeroad_2xs();
        let top = f.top_of_seat_tube();
        approx(top.distance_to(Point::ORIGIN), f.seat_tube_length_mm, 1e-9);
    }
}
