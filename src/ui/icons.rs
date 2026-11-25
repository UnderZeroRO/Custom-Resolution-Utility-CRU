//! Hand-drawn vector icons.
//!
//! Icons are drawn with primitives so they stay crisp at any DPI and need no
//! assets. Geometry is authored in a normalized 0..1 square, inset 10% from the
//! requested rect, so the same shape works at 14 px and 64 px.

use egui::{Color32, CornerRadius, Painter, Pos2, Rect, Stroke, StrokeKind, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Logo,
    Monitor,
    Grid,
    Stack,
    Code,
    Refresh,
    Import,
    Export,
    Sun,
    Moon,
    Plus,
    Minus,
    ArrowUp,
    ArrowDown,
    Shield,
    Check,
    Alert,
}

/// All icons, in drawing order. Used by tests and the dev icon sheet.
#[allow(dead_code)]
pub const ALL: [Icon; 17] = [
    Icon::Logo,
    Icon::Monitor,
    Icon::Grid,
    Icon::Stack,
    Icon::Code,
    Icon::Refresh,
    Icon::Import,
    Icon::Export,
    Icon::Sun,
    Icon::Moon,
    Icon::Plus,
    Icon::Minus,
    Icon::ArrowUp,
    Icon::ArrowDown,
    Icon::Shield,
    Icon::Check,
    Icon::Alert,
];

/// Stroke width in points: hairlines stay visible when small and grow a little
/// with the icon so large icons do not look anemic.
pub fn stroke_width(rect: Rect) -> f32 {
    (rect.width().min(rect.height()) / 16.0).clamp(1.0, 2.0)
}

/// Paint `icon` centered in `rect`, using at most a square inset 10% smaller.
pub fn draw(icon: Icon, painter: &Painter, rect: Rect, color: Color32) {
    let side = rect.width().min(rect.height());
    if side < 1.0 {
        return;
    }
    let square = Rect::from_center_size(rect.center(), Vec2::splat(side));
    let pen = Pen {
        painter,
        inner: square.shrink(side * 0.10),
        color,
        stroke: stroke_width(square),
        side,
    };

    match icon {
        Icon::Logo => {
            // Precision screen bezel + calibration timing wave inside.
            pen.frame((0.06, 0.14), (0.94, 0.78));
            pen.bezier((0.18, 0.52), (0.34, 0.24), (0.44, 0.24), (0.50, 0.46));
            pen.bezier((0.50, 0.46), (0.56, 0.68), (0.68, 0.68), (0.82, 0.40));
            pen.seg((0.36, 0.90), (0.64, 0.90));
        }
        Icon::Monitor => {
            pen.frame((0.07, 0.13), (0.93, 0.73));
            pen.seg((0.50, 0.73), (0.50, 0.87));
            pen.seg((0.31, 0.88), (0.69, 0.88));
        }
        Icon::Grid => {
            pen.frame((0.10, 0.10), (0.44, 0.44));
            pen.frame((0.56, 0.10), (0.90, 0.44));
            pen.frame((0.10, 0.56), (0.44, 0.90));
            pen.frame((0.56, 0.56), (0.90, 0.90));
        }
        Icon::Stack => {
            pen.frame((0.30, 0.10), (0.90, 0.62));
            pen.frame((0.10, 0.38), (0.70, 0.90));
        }
        Icon::Code => {
            pen.poly(&[(0.34, 0.26), (0.12, 0.50), (0.34, 0.74)], false);
            pen.poly(&[(0.66, 0.26), (0.88, 0.50), (0.66, 0.74)], false);
            pen.seg((0.55, 0.20), (0.45, 0.80));
        }
        Icon::Refresh => {
            pen.arc((0.5, 0.5), 0.36, 30.0, 300.0);
            let a = 300f32.to_radians();
            let tip = (0.5 + 0.36 * a.cos(), 0.5 + 0.36 * a.sin());
            let t = (-a.sin(), a.cos());
            let n = (a.cos(), a.sin());
            pen.seg(
                tip,
                (tip.0 - t.0 * 0.20 + n.0 * 0.11, tip.1 - t.1 * 0.20 + n.1 * 0.11),
            );
            pen.seg(
                tip,
                (tip.0 - t.0 * 0.20 - n.0 * 0.11, tip.1 - t.1 * 0.20 - n.1 * 0.11),
            );
        }
        Icon::Import => {
            pen.seg((0.50, 0.12), (0.50, 0.66));
            pen.seg((0.50, 0.66), (0.28, 0.44));
            pen.seg((0.50, 0.66), (0.72, 0.44));
            pen.poly(&[(0.16, 0.72), (0.16, 0.88), (0.84, 0.88), (0.84, 0.72)], false);
        }
        Icon::Export => {
            pen.seg((0.50, 0.66), (0.50, 0.12));
            pen.seg((0.50, 0.12), (0.28, 0.34));
            pen.seg((0.50, 0.12), (0.72, 0.34));
            pen.poly(&[(0.16, 0.72), (0.16, 0.88), (0.84, 0.88), (0.84, 0.72)], false);
        }
        Icon::Sun => {
            pen.ring((0.5, 0.5), 0.18);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::FRAC_PI_4;
                let (sin, cos) = a.sin_cos();
                pen.seg(
                    (0.5 + 0.30 * cos, 0.5 + 0.30 * sin),
                    (0.5 + 0.44 * cos, 0.5 + 0.44 * sin),
                );
            }
        }
        Icon::Moon => {
            pen.bezier((0.78, 0.14), (0.04, 0.18), (0.04, 0.82), (0.78, 0.86));
            pen.bezier((0.78, 0.86), (0.34, 0.70), (0.34, 0.30), (0.78, 0.14));
        }
        Icon::Plus => {
            pen.seg((0.50, 0.14), (0.50, 0.86));
            pen.seg((0.14, 0.50), (0.86, 0.50));
        }
        Icon::Minus => pen.seg((0.14, 0.50), (0.86, 0.50)),
        Icon::ArrowUp => {
            pen.seg((0.50, 0.84), (0.50, 0.16));
            pen.seg((0.50, 0.16), (0.26, 0.40));
            pen.seg((0.50, 0.16), (0.74, 0.40));
        }
        Icon::ArrowDown => {
            pen.seg((0.50, 0.16), (0.50, 0.84));
            pen.seg((0.50, 0.84), (0.26, 0.60));
            pen.seg((0.50, 0.84), (0.74, 0.60));
        }
        Icon::Shield => {
            pen.poly(
                &[
                    (0.50, 0.06),
                    (0.86, 0.20),
                    (0.86, 0.50),
                    (0.70, 0.76),
                    (0.50, 0.94),
                    (0.30, 0.76),
                    (0.14, 0.50),
                    (0.14, 0.20),
                ],
                true,
            );
            pen.poly(&[(0.33, 0.47), (0.45, 0.60), (0.68, 0.35)], false);
        }
        Icon::Check => pen.poly(&[(0.16, 0.53), (0.41, 0.78), (0.84, 0.24)], false),
        Icon::Alert => {
            pen.poly(&[(0.50, 0.08), (0.93, 0.88), (0.07, 0.88)], true);
            pen.seg((0.50, 0.36), (0.50, 0.61));
            pen.dot((0.50, 0.75), 0.045);
        }
    }
}

/// Rounded-cap drawing helper in normalized coordinates.
struct Pen<'a> {
    painter: &'a Painter,
    inner: Rect,
    color: Color32,
    stroke: f32,
    side: f32,
}

impl Pen<'_> {
    fn pt(&self, x: f32, y: f32) -> Pos2 {
        self.inner.min + Vec2::new(x * self.inner.width(), y * self.inner.height())
    }

    /// Line segment with round caps (egui caps are butt by default).
    fn seg(&self, a: (f32, f32), b: (f32, f32)) {
        let (a, b) = (self.pt(a.0, a.1), self.pt(b.0, b.1));
        self.painter
            .line_segment([a, b], Stroke::new(self.stroke, self.color));
        let r = self.stroke * 0.5;
        self.painter.circle_filled(a, r, self.color);
        self.painter.circle_filled(b, r, self.color);
    }

    fn poly(&self, pts: &[(f32, f32)], closed: bool) {
        if pts.len() < 2 {
            return;
        }
        for pair in pts.windows(2) {
            self.seg(pair[0], pair[1]);
        }
        if closed {
            self.seg(pts[pts.len() - 1], pts[0]);
        }
    }

    fn frame(&self, a: (f32, f32), b: (f32, f32)) {
        let rect = Rect::from_min_max(self.pt(a.0, a.1), self.pt(b.0, b.1));
        let radius = (self.side * 0.09).round().clamp(2.0, 6.0) as u8;
        self.painter.rect_stroke(
            rect,
            CornerRadius::same(radius),
            Stroke::new(self.stroke, self.color),
            StrokeKind::Inside,
        );
    }

    fn ring(&self, c: (f32, f32), radius: f32) {
        self.painter.circle_stroke(
            self.pt(c.0, c.1),
            radius * self.inner.width(),
            Stroke::new(self.stroke, self.color),
        );
    }

    fn dot(&self, c: (f32, f32), radius: f32) {
        self.painter
            .circle_filled(self.pt(c.0, c.1), radius * self.inner.width(), self.color);
    }

    fn arc(&self, c: (f32, f32), radius: f32, from_deg: f32, to_deg: f32) {
        let steps = 28;
        let mut pts = Vec::with_capacity(steps + 1);
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let a = (from_deg + (to_deg - from_deg) * t).to_radians();
            let (sin, cos) = a.sin_cos();
            pts.push((c.0 + radius * cos, c.1 + radius * sin));
        }
        self.poly(&pts, false);
    }

    fn bezier(&self, p0: (f32, f32), c1: (f32, f32), c2: (f32, f32), p3: (f32, f32)) {
        let steps = 18;
        let mut pts = Vec::with_capacity(steps + 1);
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let u = 1.0 - t;
            let (u2, t2) = (u * u, t * t);
            let x = u2 * u * p0.0 + 3.0 * u2 * t * c1.0 + 3.0 * u * t2 * c2.0 + t2 * t * p3.0;
            let y = u2 * u * p0.1 + 3.0 * u2 * t * c1.1 + 3.0 * u * t2 * c2.1 + t2 * t * p3.1;
            pts.push((x, y));
        }
        self.poly(&pts, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Pos2, Rect, Vec2};

    #[test]
    fn stroke_width_keeps_hairlines_readable() {
        assert_eq!(
            stroke_width(Rect::from_min_size(Pos2::ZERO, Vec2::splat(16.0))),
            1.0
        );
        assert_eq!(
            stroke_width(Rect::from_min_size(Pos2::ZERO, Vec2::splat(64.0))),
            2.0
        );
        assert!((1.0..=2.0).contains(&stroke_width(Rect::from_min_size(
            Pos2::ZERO,
            Vec2::splat(24.0)
        ))));
    }

    #[test]
    fn every_icon_paints_something() {
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::splat(240.0))),
            ..Default::default()
        };
        let output = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let rect = Rect::from_min_size(Pos2::ZERO, Vec2::splat(32.0));
                for icon in ALL {
                    draw(icon, ui.painter(), rect, Color32::WHITE);
                }
            });
        });
        assert!(
            output.shapes.len() > ALL.len(),
            "expected one shape per icon, got {}",
            output.shapes.len()
        );
    }
}
