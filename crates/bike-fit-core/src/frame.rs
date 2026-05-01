//! Bike frame geometry.
//!
//! A [`Frame`] holds the manufacturer-published geometry numbers (stack, reach,
//! head tube angle, etc.) and exposes derived points in BB-relative coordinates
//! (top of head tube, top of seat tube, front/rear axle, etc.).
//!
//! The full math lives in the dedicated geometry module (added in milestone 2).
//! For now we just hold the data.

use serde::{Deserialize, Serialize};

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
/// Field names follow the conventions used on geometrygeeks.bike / bike-insights.
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
    /// Effective (horizontal) top tube length (mm).
    pub top_tube_effective_mm: f64,

    /// Bottom bracket drop — vertical distance the BB sits below the wheel
    /// axle line (mm). Positive means BB is below the axles.
    pub bb_drop_mm: f64,
    /// Chainstay length, BB center to rear axle along the stay (mm).
    pub chainstay_mm: f64,
    /// Fork rake / offset (mm).
    pub fork_rake_mm: f64,

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
}
