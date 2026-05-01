//! 2D side-view renderer for a frame.
//!
//! Renders in the line-art style of `docs/reference-rendering.png`: a blue
//! background with thin white strokes for tubes, fork, and stays, and white
//! concentric circles for the wheels.
//!
//! All input geometry is in BB-relative millimeters with `+y` up. The
//! renderer projects onto an egui [`Rect`] preserving aspect ratio and
//! flipping `y` so up-on-screen matches up-in-world.

use bike_fit_core::{Frame, Point};
use eframe::egui;
use egui::{Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};

/// Visual styling for the renderer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderStyle {
    pub background: Color32,
    pub stroke: Color32,
    pub stroke_width: f32,
    /// Wheel rim band thickness, drawn as a slightly-darker concentric annulus
    /// (outer circle + inner circle of same color, eye reads it as a band).
    pub wheel_rim_band_alpha: u8,
    /// Padding (in screen pixels) between the bike's bounding rect and the
    /// edges of the canvas.
    pub padding_px: f32,
    /// Generic accent color (kept for future overlays / tests).
    pub accent: Color32,
    /// Color of the saddle target dot.
    pub saddle_accent: Color32,
    /// Color of the bar target dot.
    pub bar_accent: Color32,
    /// Color of the achieved-bar-position ring (solver output overlay).
    pub achieved_accent: Color32,
}

impl Default for RenderStyle {
    fn default() -> Self {
        Self {
            // Bike Insights-ish blue.
            background: Color32::from_rgb(74, 134, 188),
            stroke: Color32::from_rgb(245, 248, 252),
            stroke_width: 1.5,
            wheel_rim_band_alpha: 80,
            padding_px: 24.0,
            accent: Color32::from_rgb(255, 200, 60),
            // Warm yellow for saddle, soft orange-red for bar — both legible
            // against the muted-blue palette and visually distinct.
            saddle_accent: Color32::from_rgb(255, 210, 80),
            bar_accent: Color32::from_rgb(240, 110, 90),
            // Cool cyan/green for the solver result so it reads as
            // "computed" against the warm target dots.
            achieved_accent: Color32::from_rgb(110, 220, 200),
        }
    }
}

impl RenderStyle {
    /// Return a copy with a background color deterministically derived from
    /// `key`. The palette is a handful of muted, similar-luminance blues and
    /// teals so the line-art look is preserved while different frames feel
    /// visibly distinct when swapped.
    pub fn with_background_for_key(mut self, key: &str) -> Self {
        // Tiny FNV-1a so we don't pull in a hasher dep, and so the choice is
        // stable across runs.
        const PALETTE: &[Color32] = &[
            Color32::from_rgb(74, 134, 188),  // default blue
            Color32::from_rgb(82, 142, 168),  // teal-blue
            Color32::from_rgb(96, 122, 168),  // indigo-blue
            Color32::from_rgb(70, 150, 170),  // cool teal
            Color32::from_rgb(110, 130, 170), // slate-blue
            Color32::from_rgb(64, 148, 160),  // deep teal
        ];
        let mut h: u32 = 0x811c_9dc5;
        for b in key.as_bytes() {
            h ^= u32::from(*b);
            h = h.wrapping_mul(0x0100_0193);
        }
        self.background = PALETTE[(h as usize) % PALETTE.len()];
        self
    }
}

/// One frame rendered into one egui rect. Returns the [`Response`] for the
/// allocated canvas so the caller can wire interaction later.
pub fn show_frame(ui: &mut Ui, frame: &Frame, style: &RenderStyle) -> egui::Response {
    let desired = ui.available_size_before_wrap().max(Vec2::new(200.0, 150.0));
    let (rect, response) = ui.allocate_exact_size(desired, Sense::hover());
    paint_frame(ui.painter(), rect, frame, style);
    response
}

/// Paint a [`Frame`] into `rect` with the given `style`. The painter clips to
/// `rect` automatically when called via `Ui::painter`.
pub fn paint_frame(painter: &egui::Painter, rect: Rect, frame: &Frame, style: &RenderStyle) {
    painter.rect_filled(rect, 0.0, style.background);

    let world_bounds = bike_world_bounds(frame);
    let xform = Transform::fit(world_bounds, rect, style.padding_px);

    paint_bike(painter, frame, &xform, style);
}

/// Convenience: paint a frame plus saddle and bar target dots.
pub fn paint_frame_with_overlay(
    painter: &egui::Painter,
    rect: Rect,
    frame: &Frame,
    saddle: Option<Point>,
    bar_target: Option<Point>,
    style: &RenderStyle,
) {
    paint_frame(painter, rect, frame, style);
    let world_bounds = bike_world_bounds(frame);
    let xform = Transform::fit(world_bounds, rect, style.padding_px);
    if let Some(p) = saddle {
        paint_target_dot(
            painter,
            xform.world_to_screen(p),
            style.saddle_accent,
            style,
        );
    }
    if let Some(p) = bar_target {
        paint_target_dot(painter, xform.world_to_screen(p), style.bar_accent, style);
    }
}

/// Paint an "achieved" bar position dot drawn as an open ring (so it doesn't
/// occlude the underlying bar-target dot when the two are close), plus a
/// dashed-ish line connecting target → achieved when both are supplied.
pub fn paint_achieved_overlay(
    painter: &egui::Painter,
    rect: Rect,
    frame: &Frame,
    bar_target: Option<Point>,
    achieved: Point,
    style: &RenderStyle,
) {
    let world_bounds = bike_world_bounds(frame);
    let xform = Transform::fit(world_bounds, rect, style.padding_px);
    let achieved_screen = xform.world_to_screen(achieved);
    if let Some(t) = bar_target {
        let target_screen = xform.world_to_screen(t);
        // Connector line — same color as achieved ring, semi-transparent.
        let mut col = style.achieved_accent;
        col = Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), 180);
        painter.line_segment([target_screen, achieved_screen], Stroke::new(1.5, col));
    }
    let r = 6.0;
    painter.circle_stroke(achieved_screen, r, Stroke::new(2.0, style.achieved_accent));
    painter.circle_stroke(achieved_screen, r, Stroke::new(0.5, style.stroke));
}

/// Filled disc with a thin white ring around it, so target dots read on any
/// background.
fn paint_target_dot(painter: &egui::Painter, pos: Pos2, fill: Color32, style: &RenderStyle) {
    let r = 5.0;
    painter.circle_filled(pos, r, fill);
    painter.circle_stroke(pos, r, Stroke::new(1.0, style.stroke));
}

// --- internals ---------------------------------------------------------------

/// World-space bounding box, in BB-relative mm coordinates.
struct WorldBounds {
    min: Point,
    max: Point,
}

impl WorldBounds {
    fn from_points<I: IntoIterator<Item = Point>>(pts: I) -> Self {
        let mut iter = pts.into_iter();
        let first = iter.next().expect("need at least one point");
        let mut min = first;
        let mut max = first;
        for p in iter {
            min.x = min.x.min(p.x);
            min.y = min.y.min(p.y);
            max.x = max.x.max(p.x);
            max.y = max.y.max(p.y);
        }
        Self { min, max }
    }
}

fn bike_world_bounds(frame: &Frame) -> WorldBounds {
    let r_wheel = frame.wheel_outer_radius_mm();
    let rear = frame.rear_axle();
    let front = frame.front_axle();
    let top_ht = frame.top_of_head_tube();
    let top_st = frame.top_of_seat_tube();
    // Include wheel circles via axle ± r_wheel in both axes.
    WorldBounds::from_points([
        Point::new(rear.x - r_wheel, rear.y - r_wheel),
        Point::new(rear.x + r_wheel, rear.y + r_wheel),
        Point::new(front.x - r_wheel, front.y - r_wheel),
        Point::new(front.x + r_wheel, front.y + r_wheel),
        top_ht,
        top_st,
        Point::new(0.0, 0.0), // BB
    ])
}

/// Linear world→screen transform that preserves aspect ratio and flips y.
struct Transform {
    scale: f32, // pixels per mm
    /// Screen-space origin corresponding to world (0, 0).
    screen_origin: Pos2,
}

impl Transform {
    fn fit(world: WorldBounds, rect: Rect, padding: f32) -> Self {
        let inner = rect.shrink(padding);
        let w_w = (world.max.x - world.min.x) as f32;
        let w_h = (world.max.y - world.min.y) as f32;
        let scale = (inner.width() / w_w).min(inner.height() / w_h);

        // Screen position of world.min (after scaling) such that the world
        // bounding box is centered inside `inner`.
        let used_w = w_w * scale;
        let used_h = w_h * scale;
        let pad_x = (inner.width() - used_w) * 0.5;
        let pad_y = (inner.height() - used_h) * 0.5;

        // Screen pos of world.min: the lower-left of world bbox, but in screen
        // y grows downward, so world.min.y maps to inner.bottom() - pad_y.
        let screen_min_x = inner.left() + pad_x;
        let screen_min_y_for_world_min = inner.bottom() - pad_y;

        // From world.min, world (0,0) is offset by (-world.min.x, -world.min.y).
        // In screen: origin_x = screen_min_x + (-world.min.x) * scale
        //            origin_y = screen_min_y_for_world_min - (-world.min.y) * scale
        let origin_x = screen_min_x + (-world.min.x as f32) * scale;
        let origin_y = screen_min_y_for_world_min - (-world.min.y as f32) * scale;

        Self {
            scale,
            screen_origin: Pos2::new(origin_x, origin_y),
        }
    }

    fn world_to_screen(&self, p: Point) -> Pos2 {
        Pos2::new(
            self.screen_origin.x + (p.x as f32) * self.scale,
            self.screen_origin.y - (p.y as f32) * self.scale,
        )
    }

    fn world_to_screen_radius(&self, r_mm: f64) -> f32 {
        (r_mm as f32) * self.scale
    }
}

fn paint_bike(painter: &egui::Painter, frame: &Frame, xform: &Transform, style: &RenderStyle) {
    let stroke = Stroke::new(style.stroke_width, style.stroke);

    // --- key world-space points ---
    let bb = Point::new(0.0, 0.0);
    let top_ht = frame.top_of_head_tube();
    let bot_ht = frame.bottom_of_head_tube();
    let top_st = frame.top_of_seat_tube();
    let rear = frame.rear_axle();
    let front = frame.front_axle();

    // Top tube (effective): top of seat tube to top of head tube.
    line(painter, xform, top_st, top_ht, stroke);
    // Down tube: BB to bottom of head tube.
    line(painter, xform, bb, bot_ht, stroke);
    // Seat tube: BB to top of seat tube.
    line(painter, xform, bb, top_st, stroke);
    // Head tube: bottom to top.
    line(painter, xform, bot_ht, top_ht, stroke);
    // Chain stay: BB to rear axle.
    line(painter, xform, bb, rear, stroke);
    // Seat stay: top of seat tube to rear axle.
    line(painter, xform, top_st, rear, stroke);
    // Fork: bottom of head tube to front axle.
    line(painter, xform, bot_ht, front, stroke);

    // --- wheels ---
    let r_outer = frame.wheel_outer_radius_mm();
    let r_rim_inner = frame.wheel_size.bsd_mm() / 2.0; // inner edge of rim
    paint_wheel(painter, xform, rear, r_outer, r_rim_inner, style);
    paint_wheel(painter, xform, front, r_outer, r_rim_inner, style);
}

fn paint_wheel(
    painter: &egui::Painter,
    xform: &Transform,
    axle: Point,
    r_outer_mm: f64,
    r_rim_inner_mm: f64,
    style: &RenderStyle,
) {
    let center = xform.world_to_screen(axle);
    let r_outer = xform.world_to_screen_radius(r_outer_mm);
    let r_inner = xform.world_to_screen_radius(r_rim_inner_mm);
    let stroke = Stroke::new(style.stroke_width, style.stroke);
    // Outer (tire) and inner (rim/spoke boundary) circles, with a faint band
    // fill to suggest the rim depth.
    let band_color = Color32::from_rgba_unmultiplied(
        style.stroke.r(),
        style.stroke.g(),
        style.stroke.b(),
        style.wheel_rim_band_alpha,
    );
    painter.circle_filled(center, r_outer, band_color);
    painter.circle_filled(center, r_inner, style.background);
    painter.circle_stroke(center, r_outer, stroke);
    painter.circle_stroke(center, r_inner, stroke);
}

fn line(painter: &egui::Painter, xform: &Transform, a: Point, b: Point, stroke: Stroke) {
    painter.line_segment([xform.world_to_screen(a), xform.world_to_screen(b)], stroke);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bike_fit_core::frame::WheelSize;

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

    #[test]
    fn transform_round_trips_corners() {
        let frame = aeroad_2xs();
        let bounds = bike_world_bounds(&frame);
        let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(1000.0, 600.0));
        let xform = Transform::fit(bounds, rect, 24.0);

        // BB at world origin must land inside the inner rect.
        let bb = xform.world_to_screen(Point::new(0.0, 0.0));
        assert!(
            rect.shrink(24.0).contains(bb),
            "BB {bb:?} outside padded rect"
        );

        // Up in world should move up on screen (smaller y).
        let above_bb = xform.world_to_screen(Point::new(0.0, 100.0));
        assert!(above_bb.y < bb.y, "y not flipped: {bb:?} vs {above_bb:?}");

        // Forward in world should move right on screen.
        let fwd = xform.world_to_screen(Point::new(100.0, 0.0));
        assert!(fwd.x > bb.x);

        // Aspect ratio preserved: 100mm horizontal == 100mm vertical in pixels.
        assert!(((fwd.x - bb.x) - (bb.y - above_bb.y)).abs() < 1e-3);
    }

    #[test]
    fn world_bounds_include_both_wheels() {
        let frame = aeroad_2xs();
        let b = bike_world_bounds(&frame);
        let r = frame.wheel_outer_radius_mm();
        let rear = frame.rear_axle();
        let front = frame.front_axle();
        // Rear-most extent ≤ rear.x - r.
        assert!(b.min.x <= rear.x - r + 1e-9);
        // Front-most extent ≥ front.x + r.
        assert!(b.max.x >= front.x + r - 1e-9);
        // Bottom ≤ axle.y - r (axles sit at y = -bb_drop, both equal).
        assert!(b.min.y <= rear.y - r + 1e-9);
    }
}
