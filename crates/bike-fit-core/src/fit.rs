//! User fit profile: BB-relative target points for saddle and bar.

use serde::{Deserialize, Serialize};

use crate::Point;

/// A rider's "ideal fit" — the saddle position and bar target point in
/// BB-relative coordinates, plus a name to refer to it by.
///
/// See `AGENTS.md` §4.1 for the exact meaning of `bar_target` (the geometric
/// center of the stem's far clamp face) and §4.2 for `saddle`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FitProfile {
    pub name: String,

    /// Saddle position in BB coords. Typically `x` is negative (saddle is
    /// behind the BB) and `y` is positive (saddle is above the BB).
    pub saddle: Point,

    /// Bar target point in BB coords: the geometric center of the stem's far
    /// clamp face. Typically `x` is positive (forward of BB) and `y` is
    /// positive (above BB) but lower than the saddle.
    pub bar_target: Point,
}

impl FitProfile {
    pub fn new(name: impl Into<String>, saddle: Point, bar_target: Point) -> Self {
        Self {
            name: name.into(),
            saddle,
            bar_target,
        }
    }

    /// Saddle-to-bar drop (positive means saddle is higher than the bar).
    pub fn saddle_drop_mm(&self) -> f64 {
        self.saddle.y - self.bar_target.y
    }

    /// Saddle-to-bar reach: horizontal distance from saddle x to bar x.
    /// Positive means the bar is forward of the saddle.
    pub fn saddle_to_bar_reach_mm(&self) -> f64 {
        self.bar_target.x - self.saddle.x
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_profile() -> FitProfile {
        FitProfile::new(
            "rider-1",
            Point::new(-50.0, 730.0), // saddle: 50mm behind BB, 730mm up
            Point::new(420.0, 580.0), // bar: 420mm forward, 580mm up
        )
    }

    #[test]
    fn saddle_drop_is_positive_when_saddle_higher() {
        let p = sample_profile();
        assert!((p.saddle_drop_mm() - 150.0).abs() < 1e-9);
    }

    #[test]
    fn saddle_to_bar_reach_is_horizontal_only() {
        let p = sample_profile();
        assert!((p.saddle_to_bar_reach_mm() - 470.0).abs() < 1e-9);
    }
}
