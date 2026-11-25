//! Visual language: Sequoia 2027 palette, two-layer shadows, fonts and animated pill controls.

use std::sync::Arc;

use egui::epaint::Shadow;
use egui::{
    Align2, Color32, CornerRadius, FontFamily, FontId, Frame, Id, Margin, Mesh, Pos2, Rect,
    Response, RichText, Sense, Stroke, StrokeKind, TextStyle, Ui, Vec2,
};

use super::icons::{self, Icon};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
}

#[derive(Clone, Copy)]
pub struct Palette {
    pub bg_top: Color32,
    pub bg_bottom: Color32,
    pub panel: Color32,
    pub raised: Color32,
    pub card: Color32,
    pub hover: Color32,
    pub hairline: Color32,
    pub shadow: Color32,
    pub track: Color32,
    pub border: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub faint: Color32,
    pub accent: Color32,
    pub accent_top: Color32,
    pub accent_bottom: Color32,
    pub accent_hover: Color32,
    pub accent_text: Color32,
    pub accent_soft: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub danger: Color32,
}

impl From<&Palette> for Palette {
    fn from(value: &Palette) -> Self {
        *value
    }
}

/// Hairline border width used across both themes.
pub const HAIRLINE: f32 = 1.0;

/// Radius of full cards (details hero, content cards, dialogs).
pub const RADIUS_CARD: u8 = 18;
/// Radius of pill controls (buttons, chips, tabs, editor fields).
pub const RADIUS_CONTROL: u8 = 15;

pub const DARK: Palette = Palette {
    bg_top: Color32::from_rgb(0x0B, 0x0D, 0x12),
    bg_bottom: Color32::from_rgb(0x0E, 0x11, 0x17),
    panel: Color32::from_rgb(0x0D, 0x10, 0x16),
    raised: Color32::from_rgb(0x16, 0x1B, 0x24),
    card: Color32::from_rgb(0x12, 0x15, 0x1C),
    hover: Color32::from_rgb(0x1D, 0x23, 0x30),
    hairline: Color32::from_rgba_premultiplied(22, 22, 22, 22),
    shadow: Color32::from_rgba_premultiplied(0, 0, 0, 95),
    track: Color32::from_rgb(0x0D, 0x10, 0x15),
    border: Color32::from_rgb(0x23, 0x2A, 0x38),
    text: Color32::from_rgb(0xF1, 0xF4, 0xF9),
    muted: Color32::from_rgb(0x9A, 0xA3, 0xB4),
    faint: Color32::from_rgb(0x63, 0x6C, 0x7E),
    accent: Color32::from_rgb(0x0A, 0x84, 0xFF),
    accent_top: Color32::from_rgb(0x24, 0x94, 0xFF),
    accent_bottom: Color32::from_rgb(0x00, 0x6D, 0xE6),
    accent_hover: Color32::from_rgb(0x36, 0x9E, 0xFF),
    accent_text: Color32::WHITE,
    accent_soft: Color32::from_rgba_premultiplied(0x08, 0x28, 0x4E, 0x66),
    success: Color32::from_rgb(0x30, 0xD1, 0x58),
    warning: Color32::from_rgb(0xFF, 0x9F, 0x0A),
    danger: Color32::from_rgb(0xFF, 0x45, 0x3A),
};

pub const LIGHT: Palette = Palette {
    bg_top: Color32::from_rgb(0xF7, 0xF8, 0xFA),
    bg_bottom: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    panel: Color32::from_rgb(0xF8, 0xF9, 0xFB),
    raised: Color32::from_rgb(0xF1, 0xF4, 0xF8),
    card: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    hover: Color32::from_rgb(0xE9, 0xEE, 0xF5),
    hairline: Color32::from_rgba_premultiplied(210, 210, 210, 210),
    shadow: Color32::from_rgba_premultiplied(2, 3, 6, 26),
    track: Color32::from_rgb(0xED, 0xF0, 0xF4),
    border: Color32::from_rgb(0xDC, 0xE2, 0xEB),
    text: Color32::from_rgb(0x14, 0x17, 0x1D),
    muted: Color32::from_rgb(0x5E, 0x67, 0x7A),
    faint: Color32::from_rgb(0x8E, 0x97, 0xA8),
    accent: Color32::from_rgb(0x00, 0x7A, 0xFF),
    accent_top: Color32::from_rgb(0x1B, 0x89, 0xFF),
    accent_bottom: Color32::from_rgb(0x00, 0x64, 0xD8),
    accent_hover: Color32::from_rgb(0x28, 0x91, 0xFF),
    accent_text: Color32::WHITE,
    accent_soft: Color32::from_rgba_premultiplied(0x18, 0x48, 0x8C, 0x26),
    success: Color32::from_rgb(0x28, 0xA7, 0x45),
    warning: Color32::from_rgb(0xC9, 0x7A, 0x00),
    danger: Color32::from_rgb(0xFF, 0x3B, 0x30),
};

impl ThemeMode {
    pub fn palette(&self) -> Palette {
        match self {
            ThemeMode::Dark => DARK,
            ThemeMode::Light => LIGHT,
        }
    }
}

pub fn color_lerp(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let lerp = |x: u8, y: u8| -> u8 { (x as f32 + (y as f32 - x as f32) * t).round() as u8 };
    Color32::from_rgba_premultiplied(
        lerp(a.r(), b.r()),
        lerp(a.g(), b.g()),
        lerp(a.b(), b.b()),
        lerp(a.a(), b.a()),
    )
}

/// Vertical gradient mesh spanning `rect`, top → bottom.
pub fn background_gradient(p: &Palette, rect: Rect) -> Mesh {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), p.bg_top);
    mesh.colored_vertex(rect.left_bottom(), p.bg_bottom);
    mesh.colored_vertex(rect.right_bottom(), p.bg_bottom);
    mesh.colored_vertex(rect.right_top(), p.bg_top);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 3, 2);
    mesh
}

fn vertical_gradient_mesh(rect: Rect, top: Color32, bottom: Color32) -> Mesh {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 3, 2);
    mesh
}

fn border_line(color: Color32) -> Stroke {
    Stroke::new(HAIRLINE, color)
}

pub fn font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("semibold".into()))
}

pub fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}

pub fn load_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    for path in [
        "C:\\Windows\\Fonts\\SegUIVar.ttf",
        "C:\\Windows\\Fonts\\segoeui.ttf",
    ] {
        if let Ok(data) = std::fs::read(path) {
            fonts.font_data.insert(
                "segoe".to_owned(),
                Arc::new(egui::FontData::from_owned(data)),
            );
            fonts
                .families
                .entry(FontFamily::Proportional)
                .or_default()
                .insert(0, "segoe".to_owned());
            break;
        }
    }

    if let Ok(data) = std::fs::read("C:\\Windows\\Fonts\\seguisb.ttf") {
        fonts.font_data.insert(
            "segoe-semibold".to_owned(),
            Arc::new(egui::FontData::from_owned(data)),
        );
        let mut chain = vec!["segoe-semibold".to_owned()];
        if let Some(prop) = fonts.families.get(&FontFamily::Proportional) {
            chain.extend(prop.iter().cloned());
        }
        fonts
            .families
            .insert(FontFamily::Name("semibold".into()), chain);
    } else {
        let fallback = fonts
            .families
            .get(&FontFamily::Proportional)
            .cloned()
            .unwrap_or_default();
        fonts
            .families
            .insert(FontFamily::Name("semibold".into()), fallback);
    }

    if let Ok(data) = std::fs::read("C:\\Windows\\Fonts\\consola.ttf") {
        fonts.font_data.insert(
            "consola".to_owned(),
            Arc::new(egui::FontData::from_owned(data)),
        );
        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .insert(0, "consola".to_owned());
    }

    ctx.set_fonts(fonts);
}

pub fn apply_style(ctx: &egui::Context, mode: ThemeMode) {
    let p = mode.palette();
    let mut style = (*ctx.style()).clone();

    style.text_styles = [
        (TextStyle::Heading, font(22.0)),
        (TextStyle::Body, font(13.5)),
        (TextStyle::Monospace, mono(12.5)),
        (TextStyle::Button, font(13.5)),
        (TextStyle::Small, font(11.5)),
        (TextStyle::Name("h1".into()), semibold(22.0)),
        (TextStyle::Name("h2".into()), semibold(15.0)),
        (TextStyle::Name("caps".into()), semibold(11.0)),
        (TextStyle::Name("mono-small".into()), mono(11.5)),
    ]
    .into();

    let visuals = &mut style.visuals;
    visuals.dark_mode = mode == ThemeMode::Dark;
    visuals.override_text_color = Some(p.text);
    visuals.panel_fill = p.bg_top;
    visuals.window_fill = p.card;
    visuals.window_stroke = border_line(p.border);
    visuals.extreme_bg_color = p.track;
    visuals.faint_bg_color = p.raised;
    visuals.selection.bg_fill = p.accent.gamma_multiply(0.35);
    visuals.selection.stroke = border_line(p.accent);
    visuals.hyperlink_color = p.accent;

    let radius = CornerRadius::same(10);
    visuals.widgets.noninteractive.bg_fill = p.card;
    visuals.widgets.noninteractive.weak_bg_fill = p.card;
    visuals.widgets.noninteractive.bg_stroke = border_line(p.border);
    visuals.widgets.noninteractive.fg_stroke = border_line(p.text);
    visuals.widgets.noninteractive.corner_radius = radius;

    visuals.widgets.inactive.bg_fill = p.raised;
    visuals.widgets.inactive.weak_bg_fill = p.raised;
    visuals.widgets.inactive.bg_stroke = border_line(p.border);
    visuals.widgets.inactive.fg_stroke = border_line(p.text);
    visuals.widgets.inactive.corner_radius = radius;

    visuals.widgets.hovered.bg_fill = p.hover;
    visuals.widgets.hovered.weak_bg_fill = p.hover;
    visuals.widgets.hovered.bg_stroke = border_line(p.border);
    visuals.widgets.hovered.fg_stroke = border_line(p.text);
    visuals.widgets.hovered.corner_radius = radius;
    visuals.widgets.hovered.expansion = 0.0;

    visuals.widgets.active.bg_fill = p.accent;
    visuals.widgets.active.weak_bg_fill = p.accent;
    visuals.widgets.active.bg_stroke = border_line(p.accent);
    visuals.widgets.active.fg_stroke = border_line(p.accent_text);
    visuals.widgets.active.corner_radius = radius;

    visuals.widgets.open = visuals.widgets.inactive;

    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.spacing.button_padding = Vec2::new(12.0, 6.0);
    style.spacing.interact_size.y = 28.0;
    style.spacing.scroll = egui::style::ScrollStyle::thin();
    style.animation_time = 0.14;

    ctx.set_style(style);
}

/// Paints card surface, 1 px border and an inset top specular hairline highlight.
pub fn paint_card_surface(painter: &egui::Painter, rect: Rect, radius: u8, p: &Palette) {
    let cr = CornerRadius::same(radius);
    painter.rect_filled(rect, cr, p.card);
    painter.rect_stroke(rect, cr, border_line(p.border), StrokeKind::Inside);

    // Inset top specular highlight so it follows the rounded edge cleanly.
    let inset = (radius as f32 * 0.75).max(4.0);
    if rect.width() > inset * 2.0 {
        painter.line_segment(
            [
                Pos2::new(rect.left() + inset, rect.top() + 1.0),
                Pos2::new(rect.right() - inset, rect.top() + 1.0),
            ],
            Stroke::new(HAIRLINE, p.hairline),
        );
    }
}

/// Rounded card with two-layer shadow (diffuse drop + tight contact), 1 px border
/// and an inset top hairline highlight.
pub fn card_panel<R>(
    ui: &mut Ui,
    p: impl Into<Palette>,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> egui::InnerResponse<R> {
    let p = p.into();
    let dark = p.bg_top.r() < 128;
    let diffuse = Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: p.shadow,
    };
    let contact = Shadow {
        offset: [0, 1],
        blur: 3,
        spread: 0,
        color: Color32::from_rgba_premultiplied(0, 0, 0, if dark { 68 } else { 16 }),
    };

    Frame::NONE
        .shadow(diffuse)
        .show(ui, |ui| {
            let inner = Frame::new()
                .fill(p.card)
                .stroke(border_line(p.border))
                .corner_radius(CornerRadius::same(RADIUS_CARD))
                .inner_margin(Margin::same(18))
                .shadow(contact)
                .show(ui, add_contents);

            let rect = inner.response.rect;
            let inset = (RADIUS_CARD as f32 * 0.75).max(6.0);
            if rect.width() > inset * 2.0 {
                ui.painter().line_segment(
                    [
                        Pos2::new(rect.left() + inset, rect.top() + 1.0),
                        Pos2::new(rect.right() - inset, rect.top() + 1.0),
                    ],
                    Stroke::new(HAIRLINE, p.hairline),
                );
            }
            inner.inner
        })
}

/// Frame for the bottom-center toast with two-layer depth.
pub fn toast_frame(p: impl Into<Palette>) -> Frame {
    let p = p.into();
    Frame::new()
        .fill(p.card)
        .stroke(border_line(p.border))
        .corner_radius(CornerRadius::same(RADIUS_CONTROL))
        .inner_margin(Margin::symmetric(16, 10))
        .shadow(Shadow {
            offset: [0, 8],
            blur: 24,
            spread: 0,
            color: p.shadow,
        })
}

pub fn section_title(ui: &mut Ui, p: impl Into<Palette>, title: &str) {
    let p = p.into();
    ui.label(
        RichText::new(title.to_uppercase())
            .text_style(TextStyle::Name("caps".into()))
            .color(p.faint),
    );
    ui.add_space(4.0);
}

pub fn h1(ui: &mut Ui, p: impl Into<Palette>, text: &str) {
    let p = p.into();
    ui.label(
        RichText::new(text)
            .text_style(TextStyle::Name("h1".into()))
            .color(p.text),
    );
}

pub fn h2(ui: &mut Ui, p: impl Into<Palette>, text: &str) {
    let p = p.into();
    ui.label(
        RichText::new(text)
            .text_style(TextStyle::Name("h2".into()))
            .color(p.text),
    );
}

pub fn muted(ui: &mut Ui, p: impl Into<Palette>, text: impl Into<String>) {
    let p = p.into();
    ui.label(RichText::new(text.into()).size(12.5).color(p.muted));
}

/// Primary Apple-blue pill button with vertical gradient and 140 ms hover transition.
pub fn primary_button(ui: &mut Ui, p: impl Into<Palette>, text: &str, enabled: bool) -> Response {
    let p = p.into();
    let id = ui.make_persistent_id(("primary_btn", text));
    let galley = ui.painter().layout_no_wrap(
        text.to_owned(),
        semibold(13.5),
        if enabled { p.accent_text } else { p.accent_text.gamma_multiply(0.55) },
    );
    let desired = Vec2::new((galley.size().x + 28.0).max(74.0), 30.0);
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(desired, sense);
    let hovered = ui
        .ctx()
        .animate_bool_with_time(id, enabled && response.hovered(), 0.14);

    let radius = CornerRadius::same(RADIUS_CONTROL);
    if enabled {
        let top = color_lerp(p.accent_top, p.accent_hover, hovered);
        let bot = color_lerp(p.accent_bottom, p.accent, hovered);
        // Base rounded pill, overlaid with clipped gradient feel via top/bottom fill.
        ui.painter().rect_filled(rect, radius, bot);
        let top_half = Rect::from_min_max(rect.min, Pos2::new(rect.max.x, rect.center().y + 2.0));
        ui.painter().rect_filled(top_half, radius, top);
        // Smooth center band mesh for subtle vertical gradient.
        let inner = rect.shrink2(Vec2::new(4.0, 2.0));
        if inner.height() > 2.0 {
            ui.painter()
                .add(egui::epaint::Shape::Mesh(Arc::new(vertical_gradient_mesh(
                    inner, top, bot,
                ))));
        }
        // Subtle top inner sheen.
        ui.painter().line_segment(
            [
                Pos2::new(rect.left() + 10.0, rect.top() + 1.0),
                Pos2::new(rect.right() - 10.0, rect.top() + 1.0),
            ],
            Stroke::new(HAIRLINE, Color32::from_white_alpha(48)),
        );
    } else {
        ui.painter()
            .rect_filled(rect, radius, p.accent.gamma_multiply(0.28));
    }

    ui.painter().galley(
        rect.center() - galley.size() / 2.0,
        galley,
        if enabled {
            p.accent_text
        } else {
            p.accent_text.gamma_multiply(0.55)
        },
    );
    response
}

/// Pill ghost button with 140 ms animated hover surface and optional leading vector icon.
pub fn ghost_button(ui: &mut Ui, p: impl Into<Palette>, text: &str, enabled: bool) -> Response {
    ghost_button_inner(ui, p.into(), None, text, enabled, false)
}

pub fn ghost_button_icon(
    ui: &mut Ui,
    p: impl Into<Palette>,
    icon: Icon,
    text: &str,
    enabled: bool,
) -> Response {
    ghost_button_inner(ui, p.into(), Some(icon), text, enabled, false)
}

pub fn ghost_button_icon_sized(
    ui: &mut Ui,
    p: impl Into<Palette>,
    icon: Icon,
    text: &str,
    width: f32,
) -> Response {
    let p = p.into();
    let id = ui.make_persistent_id(("ghost_btn_sized", text));
    let desired = Vec2::new(width, 30.0);
    let (rect, response) = ui.allocate_exact_size(desired, Sense::click());
    let hovered = ui.ctx().animate_bool_with_time(id, response.hovered(), 0.14);

    let radius = CornerRadius::same(RADIUS_CONTROL);
    let fill = color_lerp(p.raised.gamma_multiply(0.5), p.hover, hovered);
    ui.painter().rect_filled(rect, radius, fill);
    ui.painter()
        .rect_stroke(rect, radius, border_line(p.border), StrokeKind::Inside);

    let galley = ui.painter().layout_no_wrap(text.to_owned(), font(12.5), p.text);
    let total_w = 14.0 + 6.0 + galley.size().x;
    let start_x = rect.center().x - total_w * 0.5;
    let icon_rect = Rect::from_center_size(
        Pos2::new(start_x + 7.0, rect.center().y),
        Vec2::splat(14.0),
    );
    icons::draw(icon, ui.painter(), icon_rect, color_lerp(p.muted, p.text, hovered));
    ui.painter().galley(
        Pos2::new(start_x + 20.0, rect.center().y - galley.size().y * 0.5),
        galley,
        p.text,
    );
    response
}

fn ghost_button_inner(
    ui: &mut Ui,
    p: Palette,
    icon: Option<Icon>,
    text: &str,
    enabled: bool,
    danger_hover: bool,
) -> Response {
    let id = ui.make_persistent_id(("ghost_btn", text));
    let text_color = if enabled { p.text } else { p.faint };
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font(13.0), text_color);
    let icon_extra = if icon.is_some() { 18.0 } else { 0.0 };
    let desired = Vec2::new((galley.size().x + 24.0 + icon_extra).max(58.0), 30.0);
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(desired, sense);

    let hovered = ui
        .ctx()
        .animate_bool_with_time(id, enabled && response.hovered(), 0.14);

    let radius = CornerRadius::same(RADIUS_CONTROL);
    let hover_bg = if danger_hover {
        p.danger.gamma_multiply(0.16)
    } else {
        p.hover
    };
    if hovered > 0.01 {
        ui.painter()
            .rect_filled(rect, radius, color_lerp(Color32::TRANSPARENT, hover_bg, hovered));
    }
    let stroke_color = if enabled {
        color_lerp(p.border, p.border.gamma_multiply(1.3), hovered)
    } else {
        p.border.gamma_multiply(0.55)
    };
    ui.painter()
        .rect_stroke(rect, radius, border_line(stroke_color), StrokeKind::Inside);

    if let Some(icon) = icon {
        let total_w = 14.0 + 6.0 + galley.size().x;
        let start_x = rect.center().x - total_w * 0.5;
        let icon_rect = Rect::from_center_size(
            Pos2::new(start_x + 7.0, rect.center().y),
            Vec2::splat(14.0),
        );
        let icon_col = if enabled {
            color_lerp(p.muted, p.text, hovered)
        } else {
            p.faint
        };
        icons::draw(icon, ui.painter(), icon_rect, icon_col);
        ui.painter().galley(
            Pos2::new(start_x + 20.0, rect.center().y - galley.size().y * 0.5),
            galley,
            text_color,
        );
    } else {
        ui.painter().galley(
            rect.center() - galley.size() / 2.0,
            galley,
            text_color,
        );
    }
    response
}

/// Draws `icon` inside a 30×30 circular button with smooth 140 ms hover fill.
pub fn icon_button(
    ui: &mut Ui,
    p: impl Into<Palette>,
    icon: Icon,
    tooltip: &str,
) -> Response {
    let p = p.into();
    let id = ui.make_persistent_id(("icon_btn", tooltip));
    let (button_rect, response) = ui.allocate_exact_size(Vec2::splat(30.0), Sense::click());
    let response = response.on_hover_text(tooltip);
    let hovered = ui.ctx().animate_bool_with_time(id, response.hovered(), 0.14);

    if hovered > 0.01 {
        let fill = color_lerp(Color32::TRANSPARENT, p.hover, hovered);
        ui.painter()
            .rect_filled(button_rect, CornerRadius::same(15), fill);
    }
    let icon_color = color_lerp(p.muted, p.text, hovered);
    icons::draw(icon, ui.painter(), button_rect.shrink(7.0), icon_color);

    response
}

/// Apple-style segmented control on a recessed `p.track` with an animated sliding pill.
pub fn tabs_with_icons(
    ui: &mut Ui,
    p: impl Into<Palette>,
    current: &mut usize,
    items: &[(&str, Icon)],
) {
    let p = p.into();
    let track_height = 34.0;
    let pad = 3.0;
    let item_h = track_height - pad * 2.0;

    // Pre-measure all segments so we can paint the recessed track & animated pill first.
    let mut widths = Vec::with_capacity(items.len());
    for (label, _) in items {
        let galley = ui
            .painter()
            .layout_no_wrap((*label).to_owned(), semibold(12.5), p.text);
        widths.push(galley.size().x + 14.0 + 6.0 + 22.0);
    }
    let total_w: f32 = widths.iter().sum::<f32>() + pad * 2.0 + (items.len().saturating_sub(1) as f32) * 2.0;

    let (track_rect, _) = ui.allocate_exact_size(Vec2::new(total_w, track_height), Sense::hover());
    ui.painter()
        .rect_filled(track_rect, CornerRadius::same(17), p.track);
    ui.painter().rect_stroke(
        track_rect,
        CornerRadius::same(17),
        border_line(p.border.gamma_multiply(0.8)),
        StrokeKind::Inside,
    );

    let mut seg_rects = Vec::with_capacity(items.len());
    let mut x = track_rect.left() + pad;
    for w in &widths {
        seg_rects.push(Rect::from_min_size(
            Pos2::new(x, track_rect.top() + pad),
            Vec2::new(*w, item_h),
        ));
        x += *w + 2.0;
    }

    let active_idx = (*current).min(seg_rects.len().saturating_sub(1));
    if let Some(target_rect) = seg_rects.get(active_idx) {
        let anim_x = ui.ctx().animate_value_with_time(
            Id::new("segmented_tab_x"),
            target_rect.left(),
            0.16,
        );
        let anim_w = ui.ctx().animate_value_with_time(
            Id::new("segmented_tab_w"),
            target_rect.width(),
            0.16,
        );
        let pill = Rect::from_min_size(
            Pos2::new(anim_x, target_rect.top()),
            Vec2::new(anim_w, item_h),
        );
        ui.painter()
            .rect_filled(pill, CornerRadius::same(14), p.card);
        ui.painter().rect_stroke(
            pill,
            CornerRadius::same(14),
            border_line(p.border),
            StrokeKind::Inside,
        );
        if pill.width() > 20.0 {
            ui.painter().line_segment(
                [
                    Pos2::new(pill.left() + 10.0, pill.top() + 1.0),
                    Pos2::new(pill.right() - 10.0, pill.top() + 1.0),
                ],
                Stroke::new(HAIRLINE, p.hairline),
            );
        }
    }

    let mut clicked = None;
    for (index, ((label, icon), seg_rect)) in items.iter().zip(seg_rects.iter()).enumerate() {
        let selected = *current == index;
        let id = Id::new(("segmented_tab", index));
        let response = ui.interact(*seg_rect, id, Sense::click());
        let hover_t = ui
            .ctx()
            .animate_bool_with_time(id, !selected && response.hovered(), 0.12);
        let active_t = ui
            .ctx()
            .animate_bool_with_time(Id::new(("segmented_active", index)), selected, 0.14);

        let fg = color_lerp(
            color_lerp(p.muted, p.text, hover_t),
            p.text,
            active_t,
        );
        let icon_col = color_lerp(
            color_lerp(p.faint, p.muted, hover_t),
            p.accent,
            active_t,
        );
        let galley = ui
            .painter()
            .layout_no_wrap((*label).to_owned(), semibold(12.5), fg);
        let content_w = 13.0 + 6.0 + galley.size().x;
        let start_x = seg_rect.center().x - content_w * 0.5;
        let icon_rect = Rect::from_center_size(
            Pos2::new(start_x + 6.5, seg_rect.center().y),
            Vec2::splat(13.0),
        );
        icons::draw(*icon, ui.painter(), icon_rect, icon_col);
        ui.painter().galley(
            Pos2::new(start_x + 19.0, seg_rect.center().y - galley.size().y * 0.5),
            galley,
            fg,
        );

        if response.clicked() {
            clicked = Some(index);
        }
    }

    if let Some(index) = clicked {
        *current = index;
    }
}

/// Pill chip with 11 px text, radius 9, 6×10 padding.
pub fn chip(ui: &mut Ui, _p: impl Into<Palette>, text: &str, color: Color32) {
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), semibold(11.0), color);
    let padding = Vec2::new(7.0, 4.0);
    let size = galley.size() + padding * 2.0;
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(10), color.gamma_multiply(0.14));
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(10),
        border_line(color.gamma_multiply(0.55)),
        StrokeKind::Inside,
    );
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, color);
}

/// Layout a single-line ellipsized galley constrained to `max_width`.
pub fn ellipsize_galley(
    painter: &egui::Painter,
    text: &str,
    font_id: FontId,
    color: Color32,
    max_width: f32,
) -> Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::single_section(
        text.to_owned(),
        egui::TextFormat {
            font_id,
            color,
            ..Default::default()
        },
    );
    job.wrap = egui::text::TextWrapping {
        max_width: max_width.max(16.0),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    painter.layout_job(job)
}

/// Row width helper used by tables.
pub struct Columns {
    pub widths: Vec<f32>,
}

impl Columns {
    pub fn new(widths: &[f32]) -> Self {
        Self {
            widths: widths.to_vec(),
        }
    }

    pub fn total(&self) -> f32 {
        self.widths.iter().sum()
    }
}

/// Draws one table row with optional leading aspect preview and returns its response.
#[allow(clippy::too_many_arguments)]
pub fn table_row(
    ui: &mut Ui,
    p: impl Into<Palette>,
    id: Id,
    cells: &[String],
    columns: &Columns,
    selected: bool,
    enabled: bool,
    row_height: f32,
    font_size: f32,
    aspect: Option<f32>,
) -> Response {
    let p = p.into();
    let width = columns.total().max(ui.available_width());
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, row_height), Sense::click());

    let hover_t = ui
        .ctx()
        .animate_bool_with_time(id, !selected && enabled && response.hovered(), 0.13);

    let row_rect = rect.shrink2(Vec2::new(0.0, 1.5));
    if selected {
        ui.painter()
            .rect_filled(row_rect, CornerRadius::same(9), p.accent_soft);
        ui.painter().rect_stroke(
            row_rect,
            CornerRadius::same(9),
            border_line(p.accent.gamma_multiply(0.55)),
            StrokeKind::Inside,
        );
    } else if hover_t > 0.01 {
        ui.painter().rect_filled(
            row_rect,
            CornerRadius::same(9),
            color_lerp(Color32::TRANSPARENT, p.hover, hover_t),
        );
    }

    let mut x = rect.left() + 10.0;
    for (index, cell) in cells.iter().enumerate() {
        let cell_width = columns.widths.get(index).copied().unwrap_or(120.0);
        let color = if !enabled {
            p.faint
        } else if index == 0 {
            p.text
        } else {
            p.muted
        };

        let mut text_x = x;
        if index == 0 {
            if let Some(ar) = aspect {
                let slot = Rect::from_min_size(
                    Pos2::new(x, rect.center().y - 8.0),
                    Vec2::new(26.0, 16.0),
                );
                super::illustration::paint_aspect_badge(ui.painter(), slot, &p, ar);
                text_x += 34.0;
            }
        }

        let cell_font = if index == 0 {
            semibold(font_size)
        } else if index <= 2 {
            // Tabular/monospace digits for refresh & pixel clock so columns don't jitter.
            mono(font_size - 0.5)
        } else {
            font(font_size)
        };
        ui.painter()
            .text(Pos2::new(text_x, rect.center().y), Align2::LEFT_CENTER, cell, cell_font, color);
        x += cell_width;
    }

    response
}

pub fn divider(ui: &mut Ui, p: impl Into<Palette>) {
    let p = p.into();
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), HAIRLINE), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::ZERO, p.border);
}
