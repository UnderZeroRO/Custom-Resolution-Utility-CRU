//! Monitor illustration and mini aspect-ratio previews.
//!
//! Everything is painted with `egui::Painter` primitives — no raster assets,
//! no SVG — so the art scales crisply at any DPI and works in both themes.

use egui::epaint::{EllipseShape, Shape};
use egui::{
    Align2, Color32, CornerRadius, FontFamily, FontId, Mesh, Pos2, Rect, Sense, Stroke,
    StrokeKind, Ui, Vec2,
};
use std::sync::Arc;

use super::theme::{Palette, HAIRLINE};

/// Largest rect with the given `aspect` (w/h) that fits inside `available`,
/// centered.
pub fn aspect_box(available: Rect, aspect: f32) -> Rect {
    let safe_aspect = if aspect.is_finite() && aspect > 0.1 {
        aspect.clamp(0.4, 4.5)
    } else {
        16.0 / 9.0
    };
    let area_w = available.width().max(1.0);
    let area_h = available.height().max(1.0);
    let (w, h) = if area_w / area_h > safe_aspect {
        let h = area_h;
        let w = safe_aspect * h;
        (w, h)
    } else {
        let w = area_w;
        let h = w / safe_aspect;
        (w, h)
    };
    let x = available.left() + (area_w - w) / 2.0;
    let y = available.top() + (area_h - h) / 2.0;
    Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h))
}

/// Paints a scaled aspect-ratio badge inside `slot`.
pub fn paint_aspect_badge(painter: &egui::Painter, slot: Rect, p: &Palette, aspect: f32) {
    let rect = aspect_box(slot, aspect);
    painter.rect_filled(rect, CornerRadius::same(3), p.accent_soft.gamma_multiply(0.85));
    painter.rect_stroke(
        rect,
        CornerRadius::same(3),
        Stroke::new(HAIRLINE, p.accent.gamma_multiply(0.75)),
        StrokeKind::Inside,
    );
    let inner = rect.shrink(2.5);
    if inner.width() > 6.0 && inner.height() > 4.0 {
        painter.line_segment(
            [
                Pos2::new(inner.left() + 1.0, inner.top() + 0.5),
                Pos2::new(inner.right() - 1.0, inner.top() + 0.5),
            ],
            Stroke::new(HAIRLINE, p.hairline),
        );
    }
}

/// Mini aspect-ratio preview badge: 26×16 slot containing a scaled frame that
/// matches `aspect` (e.g. 16:9 vs 21:9 vs 4:3) with accent-soft fill and hairline.
#[allow(dead_code)]
pub fn aspect_chip(ui: &mut Ui, p: &Palette, aspect: f32) -> egui::Response {
    let slot = Vec2::new(26.0, 16.0);
    let (outer, response) = ui.allocate_exact_size(slot, Sense::hover());
    paint_aspect_badge(ui.painter(), outer, p, aspect);
    response
}

/// Human-friendly ratio tag (e.g. "16:9", "21:9", "16:10", "4:3") for the hero screen.
pub fn aspect_label(aspect: f32) -> String {
    let known = [
        (16.0 / 9.0, "16:9"),
        (16.0 / 10.0, "16:10"),
        (21.0 / 9.0, "21:9"),
        (32.0 / 9.0, "32:9"),
        (4.0 / 3.0, "4:3"),
        (5.0 / 4.0, "5:4"),
        (3.0 / 2.0, "3:2"),
    ];
    for (ratio, label) in known {
        if (aspect - ratio).abs() < 0.04 {
            return label.to_string();
        }
    }
    format!("{aspect:.2}:1")
}

/// Draws a next-gen Apple-like vector monitor inside `rect`:
/// multi-monitor silhouette behind (when `multi_display`), soft ambient glow,
/// precision outer bezel hugging the `aspect` screen, glass sheen, active
/// resolution + refresh badge inside the screen, and a contoured aluminum stand.
#[allow(clippy::too_many_arguments)]
pub fn monitor(
    ui: &mut Ui,
    p: &Palette,
    rect: Rect,
    resolution: &str,
    refresh: &str,
    aspect: f32,
    multi_display: bool,
) {
    let painter = ui.painter();
    let dark = p.bg_top.r() < 128;

    // Reserve bottom ~22% for stand neck + tapered base + ambient shadow.
    let stand_zone_h = (rect.height() * 0.22).clamp(20.0, 32.0);
    let screen_zone = Rect::from_min_max(
        rect.min + Vec2::new(8.0, 4.0),
        Pos2::new(rect.max.x - 8.0, rect.max.y - stand_zone_h),
    );

    // Fit screen strictly to `aspect`, leaving 5 px margin for the bezel.
    let screen = aspect_box(screen_zone.shrink(5.0), aspect);
    let bezel = screen.expand(5.0);

    // Optional offset secondary monitor peeking behind on multi-display setups.
    if multi_display && rect.width() > 170.0 {
        let back_w = screen.width() * 0.62;
        let back_h = screen.height() * 0.62;
        let back_screen = Rect::from_min_size(
            Pos2::new(
                (bezel.left() - back_w * 0.22).max(rect.left() + 2.0),
                bezel.top() - 4.0,
            ),
            Vec2::new(back_w, back_h),
        );
        let back_bezel = back_screen.expand(3.0);
        let back_fill = if dark {
            Color32::from_rgb(0x13, 0x18, 0x22)
        } else {
            Color32::from_rgb(0xE6, 0xEA, 0xF2)
        };
        painter.rect_filled(back_bezel, CornerRadius::same(7), back_fill);
        painter.rect_stroke(
            back_bezel,
            CornerRadius::same(7),
            Stroke::new(HAIRLINE, p.border.gamma_multiply(0.55)),
            StrokeKind::Inside,
        );
    }

    // Soft dual-layer ambient glow beneath & behind the monitor.
    let glow_center = Pos2::new(bezel.center().x, bezel.bottom() + stand_zone_h * 0.45);
    painter.add(Shape::Ellipse(EllipseShape {
        center: glow_center,
        radius: Vec2::new(bezel.width() * 0.46, stand_zone_h * 0.55),
        fill: p.accent.gamma_multiply(if dark { 0.14 } else { 0.10 }),
        stroke: Stroke::NONE,
    }));
    painter.add(Shape::Ellipse(EllipseShape {
        center: Pos2::new(bezel.center().x, bezel.bottom() + stand_zone_h * 0.72),
        radius: Vec2::new(bezel.width() * 0.34, 4.0),
        fill: p.shadow.gamma_multiply(if dark { 0.55 } else { 0.35 }),
        stroke: Stroke::NONE,
    }));

    // Sleek contoured stand: slender vertical neck + tapered aluminum foot.
    let stand_top = bezel.bottom() - 1.0;
    let neck_h = (stand_zone_h * 0.54).max(10.0);
    let neck_w_top = (screen.width() * 0.15).clamp(16.0, 26.0);
    let neck_w_bot = neck_w_top + 6.0;
    let cx = bezel.center().x;
    let stand_fill = if dark {
        Color32::from_rgb(0x20, 0x26, 0x33)
    } else {
        Color32::from_rgb(0xD8, 0xDF, 0xEA)
    };
    let stand_foot_fill = if dark {
        Color32::from_rgb(0x28, 0x30, 0x3F)
    } else {
        Color32::from_rgb(0xE8, 0xEC, 0xF3)
    };
    painter.add(Shape::convex_polygon(
        vec![
            Pos2::new(cx - neck_w_top * 0.5, stand_top),
            Pos2::new(cx + neck_w_top * 0.5, stand_top),
            Pos2::new(cx + neck_w_bot * 0.5, stand_top + neck_h),
            Pos2::new(cx - neck_w_bot * 0.5, stand_top + neck_h),
        ],
        stand_fill,
        Stroke::new(HAIRLINE, p.border),
    ));

    let foot_w = (screen.width() * 0.44).clamp(52.0, 86.0);
    let foot_rect = Rect::from_min_size(
        Pos2::new(cx - foot_w * 0.5, stand_top + neck_h - 1.0),
        Vec2::new(foot_w, 5.0),
    );
    painter.rect_filled(foot_rect, CornerRadius::same(3), stand_foot_fill);
    painter.rect_stroke(
        foot_rect,
        CornerRadius::same(3),
        Stroke::new(HAIRLINE, p.border),
        StrokeKind::Inside,
    );
    painter.line_segment(
        [
            Pos2::new(foot_rect.left() + 3.0, foot_rect.top() + 0.5),
            Pos2::new(foot_rect.right() - 3.0, foot_rect.top() + 0.5),
        ],
        Stroke::new(HAIRLINE, p.hairline),
    );

    // Outer metallic/obsidian bezel frame.
    let bezel_fill = if dark {
        Color32::from_rgb(0x1B, 0x21, 0x2D)
    } else {
        Color32::from_rgb(0x1E, 0x24, 0x30)
    };
    painter.rect_filled(bezel, CornerRadius::same(10), bezel_fill);
    painter.rect_stroke(
        bezel,
        CornerRadius::same(10),
        Stroke::new(HAIRLINE, if dark { p.border } else { Color32::from_rgb(0x38, 0x41, 0x52) }),
        StrokeKind::Inside,
    );
    // Top specular highlight on bezel.
    painter.line_segment(
        [
            Pos2::new(bezel.left() + 10.0, bezel.top() + 0.75),
            Pos2::new(bezel.right() - 10.0, bezel.top() + 0.75),
        ],
        Stroke::new(HAIRLINE, Color32::from_white_alpha(if dark { 42 } else { 70 })),
    );

    // Tiny webcam / ambient light sensor dot in top bezel.
    painter.circle_filled(
        Pos2::new(bezel.center().x, bezel.top() + 2.5),
        1.2,
        Color32::from_white_alpha(65),
    );

    // Deep OLED screen gradient inside the bezel.
    let screen_top = Color32::from_rgb(0x0B, 0x10, 0x1B);
    let screen_bot = Color32::from_rgb(0x13, 0x1D, 0x30);
    let mut mesh = Mesh::default();
    mesh.colored_vertex(screen.left_top(), screen_top);
    mesh.colored_vertex(screen.left_bottom(), screen_bot);
    mesh.colored_vertex(screen.right_bottom(), screen_bot);
    mesh.colored_vertex(screen.right_top(), screen_top);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 3, 2);
    painter.add(Shape::Mesh(Arc::new(mesh)));

    // Subtle glass diagonal sheen across the upper-left corner of the OLED glass.
    painter.add(Shape::convex_polygon(
        vec![
            screen.left_top(),
            Pos2::new(screen.left() + screen.width() * 0.55, screen.top()),
            Pos2::new(screen.left() + screen.width() * 0.18, screen.bottom()),
            screen.left_bottom(),
        ],
        Color32::from_white_alpha(9),
        Stroke::NONE,
    ));

    // Inner aspect proportion guide.
    let inner_guide = screen.shrink(6.0);
    if inner_guide.width() > 24.0 && inner_guide.height() > 16.0 {
        painter.rect_stroke(
            inner_guide,
            CornerRadius::same(3),
            Stroke::new(HAIRLINE, Color32::from_white_alpha(20)),
            StrokeKind::Inside,
        );
    }

    // Active resolution centered inside the OLED display.
    let res_y_offset = if refresh.is_empty() { 0.0 } else { -8.0 };
    let res_size = if screen.width() < 120.0 { 11.5 } else { 14.5 };
    painter.text(
        screen.center() + Vec2::new(0.0, res_y_offset),
        Align2::CENTER_CENTER,
        resolution,
        FontId::new(res_size, FontFamily::Name("semibold".into())),
        Color32::from_rgb(0xF2, 0xF5, 0xFB),
    );

    // Refresh + aspect ratio pill inside the screen below the resolution.
    let meta = if refresh.is_empty() {
        aspect_label(aspect)
    } else {
        format!("{refresh}  ·  {}", aspect_label(aspect))
    };
    let galley = painter.layout_no_wrap(
        meta,
        FontId::new(9.5, FontFamily::Name("semibold".into())),
        Color32::from_rgb(0x88, 0xC4, 0xFF),
    );
    let pill_size = galley.size() + Vec2::new(12.0, 5.0);
    let pill_rect = Rect::from_center_size(
        screen.center() + Vec2::new(0.0, 12.0),
        pill_size,
    );
    if pill_rect.bottom() < screen.bottom() - 3.0 {
        painter.rect_filled(
            pill_rect,
            CornerRadius::same(7),
            Color32::from_rgba_unmultiplied(10, 132, 255, 36),
        );
        painter.rect_stroke(
            pill_rect,
            CornerRadius::same(7),
            Stroke::new(HAIRLINE, Color32::from_rgba_unmultiplied(10, 132, 255, 110)),
            StrokeKind::Inside,
        );
        painter.galley(
            pill_rect.center() - galley.size() / 2.0,
            galley,
            Color32::from_rgb(0x88, 0xC4, 0xFF),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Pos2, Rect, Vec2};

    #[test]
    fn aspect_box_fits_and_centers() {
        let area = Rect::from_min_size(Pos2::ZERO, Vec2::new(100.0, 100.0));
        let r = aspect_box(area, 16.0 / 9.0);
        assert!((r.width() - 100.0).abs() < 0.01);
        assert!((r.height() - 56.25).abs() < 0.01);
        assert!((r.center().y - 50.0).abs() < 0.01);
    }

    #[test]
    fn aspect_box_height_limited() {
        let area = Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 100.0));
        let r = aspect_box(area, 9.0 / 16.0);
        assert!((r.height() - 100.0).abs() < 0.01);
        assert!((r.width() - 56.25).abs() < 0.01);
        assert!((r.center().x - 200.0).abs() < 0.01);
    }

    #[test]
    fn aspect_label_formats_standard_ratios() {
        assert_eq!(aspect_label(16.0 / 9.0), "16:9");
        assert_eq!(aspect_label(21.0 / 9.0), "21:9");
        assert_eq!(aspect_label(4.0 / 3.0), "4:3");
    }
}
