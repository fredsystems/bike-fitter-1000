//! Core types and geometry math for bike-fitter-1000.
//!
//! All coordinates are in millimeters with the **bottom bracket (BB) at the
//! origin**. Positive X is forward (toward the front wheel). Positive Y is up.
//! Angles are in degrees in public APIs and converted to radians at the
//! boundary inside math functions.
//!
//! See `docs/geometry-math.md` for the math derivations and `AGENTS.md`
//! for project-wide conventions.

#![warn(clippy::all)]

pub mod cockpit;
pub mod fit;
pub mod frame;
pub mod geometry;
pub mod solver;

pub use cockpit::{Cockpit, IntegratedSku, Spacer, SpacerCatalog, Stem, StemCatalog};
pub use fit::FitProfile;
pub use frame::{Frame, WheelSize};
pub use solver::{Build, BuildError, SpacerStack};

/// 2D point/vector in millimeters, BB-relative.
///
/// Used both as a positional point ("top of head tube is at `Point::new(reach,
/// stack)`") and as a free vector ("the steerer points along this direction").
/// In a 2D Euclidean setting the distinction doesn't matter for the math, and
/// keeping a single type avoids a lot of boilerplate.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const ORIGIN: Self = Self { x: 0.0, y: 0.0 };

    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Euclidean distance to another point.
    pub fn distance_to(self, other: Self) -> f64 {
        (self - other).length()
    }

    /// Magnitude of the vector from origin to `self`.
    pub fn length(self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    /// Dot product treating `self` and `other` as vectors from the origin.
    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y
    }

    /// Rotate this vector counter-clockwise by `angle_deg` (in degrees).
    pub fn rotated(self, angle_deg: f64) -> Self {
        let r = angle_deg.to_radians();
        let (s, c) = r.sin_cos();
        Self::new(self.x * c - self.y * s, self.x * s + self.y * c)
    }

    /// Rotate 90° clockwise. Equivalent to `self.rotated(-90.0)` but exact.
    pub fn perpendicular_cw(self) -> Self {
        Self::new(self.y, -self.x)
    }
}

impl std::ops::Add for Point {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl std::ops::Sub for Point {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl std::ops::Mul<f64> for Point {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl std::ops::Neg for Point {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tolerance for floating-point comparisons in geometry tests.
    /// 1e-6 mm is well below any meaningful real-world precision.
    const EPS: f64 = 1e-6;

    fn approx(a: f64, b: f64) {
        assert!(
            (a - b).abs() < EPS,
            "expected {a} ≈ {b} (diff {})",
            (a - b).abs()
        );
    }

    fn approx_pt(a: Point, b: Point) {
        approx(a.x, b.x);
        approx(a.y, b.y);
    }

    #[test]
    fn rotate_90_ccw_takes_x_to_y() {
        approx_pt(Point::new(1.0, 0.0).rotated(90.0), Point::new(0.0, 1.0));
    }

    #[test]
    fn rotate_180_negates() {
        approx_pt(Point::new(3.0, -4.0).rotated(180.0), Point::new(-3.0, 4.0));
    }

    #[test]
    fn perpendicular_cw_takes_up_to_right() {
        approx_pt(
            Point::new(0.0, 1.0).perpendicular_cw(),
            Point::new(1.0, 0.0),
        );
    }

    #[test]
    fn length_of_3_4_5_triangle() {
        approx(Point::new(3.0, 4.0).length(), 5.0);
    }
}
