# CRU Modern UI Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restyle CRU Modern to a restrained Apple-like visual language with a vector monitor illustration, drawn icons, and resolution mini-previews.

**Architecture:** Pure presentation change. Add two focused modules (`ui/icons.rs`, `ui/illustration.rs`), retune `ui/theme.rs`, and rework the header/sidebar/status/toast layout in `ui/app.rs`. No EDID, registry, or restart logic changes.

**Tech Stack:** Rust 2021, eframe/egui 0.31 (glow), egui painter for vector art, Windows DWM for window corners.

**Spec:** `docs/superpowers/specs/2026-11-25-cru-modern-ui-redesign-design.md`

## Global Constraints

- No new crate dependencies. Only `egui`, `eframe`, `winreg`, `rfd`, `thiserror`.
- No raster assets, no SVG, no font files added to the repo.
- Both themes (dark and light) must stay legible; minimum text contrast target 4.5:1 for body text.
- Every shell needs the PATH prefix:
  `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:USERPROFILE\.rustup\toolchains\stable-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin\self-contained;$env:Path"`
- Build to a fresh `--target-dir` when the previous `cru-modern.exe` is still running.
- This directory is not a git repo: no commit steps; each task ends with a clean build plus a screenshot check.
- App must keep working at the 900×560 minimum window size.

## Review Focus

1. **Long monitor names / 10 entries in the sidebar** — names must ellipsize, rows must not grow, selection pill must stay inside the item.
2. **Minimum window 900×560** — hero illustration must shrink, never clip the chips or push tabs off-screen.
3. **Light theme** — hairline highlights and glass gradients must not wash out; borders must stay visible.
4. **DPI scaling (125%/150%)** — icon stroke widths and illustration must scale with `pixels_per_point`, no 1px hairlines that vanish.
5. **Existing dialogs** (resolution editor, restart confirm, clear confirm) — new button/segmented sizes must not break their layouts.
6. **Toast in a small window** — toast must not cover the Apply button or the status bar.

---

### Task 1: Vector icon set

**Files:**
- Create: `src/ui/icons.rs`
- Modify: `src/ui/mod.rs`
- Test: `src/ui/icons.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Produces:
  - `pub enum Icon { Logo, Monitor, Stack, Refresh, Import, Export, Sun, Moon, Plus, Minus, ArrowUp, ArrowDown, Shield, Check, Alert }`
  - `pub fn draw(icon: Icon, painter: &egui::Painter, rect: egui::Rect, color: egui::Color32)`
  - `pub fn stroke_width(rect: egui::Rect) -> f32`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn stroke_width_keeps_hairlines_readable() {
    assert_eq!(stroke_width(Rect::from_min_size(Pos2::ZERO, Vec2::splat(16.0))), 1.0);
    assert_eq!(stroke_width(Rect::from_min_size(Pos2::ZERO, Vec2::splat(64.0))), 2.0);
    assert!((1.0..=2.0).contains(&stroke_width(Rect::from_min_size(Pos2::ZERO, Vec2::splat(24.0)))));
}
```

- [ ] **Step 2: Run it and see it fail**

Run: `cargo test --bin cru-modern stroke_width_keeps_hairlines_readable`
Expected: FAIL — `stroke_width` not found.

- [ ] **Step 3: Implement the module**

`stroke_width` returns `(min(w,h) / 16.0).clamp(1.0, 2.0)`. `draw` matches on `Icon` and paints with `painter.line_segment`, `painter.rect_stroke`, `painter.circle_stroke`, `painter.add(egui::Shape::convex_polygon(...))`, all rounded (use two-point polylines to fake round caps; egui strokes are butt-capped). Icons are drawn inside a square inset 10% from `rect`.

- [ ] **Step 4: Wire the module and verify**

Add `pub mod icons;` to `src/ui/mod.rs`. Run the test again — PASS. Run `cargo build` — clean.

- [ ] **Step 5: Visual checkpoint**

Temporarily render every icon in a row (dev-only, removed in Task 5) and screenshot with `tools/shot.ps1`; confirm none are blank and shapes read at 16–20 px. Revert the dev row.

---

### Task 2: Theme foundation (palette, surfaces, type, controls)

**Files:**
- Modify: `src/ui/theme.rs`
- Modify: `src/ui/app.rs` (background gradient call only)

**Interfaces:**
- Produces:
  - `Palette` gains: `bg_top: Color32`, `bg_bottom: Color32`, `hairline: Color32`, `shadow: Color32`, `track: Color32`.
  - `pub const RADIUS_CARD: u8 = 18;`, `pub const RADIUS_CONTROL: u8 = 15;`
  - `pub fn background_gradient(p: &Palette) -> egui::Mesh`
  - `pub fn card(p: impl Into<Palette>) -> Frame` (now: radius 18, 1px border, inner top highlight, layered shadow)
  - `pub fn primary_button(...)`, `pub fn ghost_button(...)`, `pub fn icon_button(ui, p, icon: Icon, tooltip: &str) -> Response`, `pub fn tabs(...)` — same call sites, new visuals.

- [ ] **Step 1: Extend the palette**

Dark: `bg_top #0B0D12`, `bg_bottom #0E1117`, `card #12151C`, `hairline rgba(255,255,255,15)`, `shadow rgba(0,0,0,90)`, `track #0D1015`.
Light: `bg_top #F7F8FA`, `bg_bottom #FFFFFF`, `card #FFFFFF`, `hairline rgba(255,255,255,200)`, `shadow rgba(15,23,42,28)`, `track #EDF0F4`.

- [ ] **Step 2: Fonts**

Prefer `C:\Windows\Fonts\SegUIVar.ttf` for proportional text, fall back to `segoeui.ttf`; keep `seguisb.ttf` as the "semibold" family. No new files.

- [ ] **Step 3: Components**

Pill buttons with `CornerRadius::same(RADIUS_CONTROL)`, hover via `ui.ctx().animate_bool_with_time(id, hovered, 0.14)`. Segmented tabs: recessed track (`p.track`), selected pill with `p.card` + shadow; keep the signature `tabs(ui, p, current: &mut usize, labels: &[&str])`. Chips become 11 px text, radius 9, 6×10 padding. Add `toast_frame(p)`.

- [ ] **Step 4: Background gradient**

`App::update` paints `background_gradient(&p)` behind everything (first thing in `CentralPanel` frame) instead of flat `p.bg`.

- [ ] **Step 5: Verify**

`cargo build`; run; screenshot dark and light (toggle with the sun/moon button). Check: hairlines visible in light theme, body text readable, no clipped button labels.

---

### Task 3: Illustration module

**Files:**
- Create: `src/ui/illustration.rs`
- Modify: `src/ui/mod.rs`

**Interfaces:**
- Produces:
  - `pub fn aspect_box(available: egui::Rect, aspect: f32) -> egui::Rect`
  - `pub fn monitor(ui: &mut egui::Ui, p: &Palette, size: egui::Vec2, title: &str, subtitle: &str, aspect: f32)`
  - `pub fn aspect_chip(ui: &mut egui::Ui, p: &Palette, aspect: f32) -> egui::Response`

- [ ] **Step 1: Write the failing test for `aspect_box`**

```rust
#[test]
fn aspect_box_fits_and_centers() {
    let area = Rect::from_min_size(Pos2::ZERO, Vec2::new(100.0, 100.0));
    let r = aspect_box(area, 16.0 / 9.0);
    assert!((r.width() - 100.0).abs() < 0.01);
    assert!((r.height() - 56.25).abs() < 0.01);
    assert!((r.center().y - 50.0).abs() < 0.01);
}
```

- [ ] **Step 2: Run it and see it fail**

Run: `cargo test --bin cru-modern aspect_box_fits_and_centers`
Expected: FAIL — function not found.

- [ ] **Step 3: Implement `aspect_box` and `aspect_chip`**

`aspect_box` fits the largest rect of `aspect` inside `available`, centered. `aspect_chip` allocates 26×16, paints a rounded rect with `p.accent_soft` fill and `p.accent` hairline.

- [ ] **Step 4: Implement `monitor`**

Draw at `size` (clamped to available width): screen rect from `aspect_box` inside an inset frame, bezel with `p.border`, stand as a trapezoid under the screen, soft glow ellipse below (`p.accent_soft`), `title` centered in the screen in 13 px semibold, `subtitle` under it in 10 px muted. Behind the main screen, when `ui.available_width() > 320`, draw a second smaller offset screen at 25% opacity to suggest a multi-monitor setup.

- [ ] **Step 5: Verify**

Run tests (PASS) and build; screenshot the hero once Task 4 wires it in.

---

### Task 4: Details hero panel

**Files:**
- Modify: `src/ui/app.rs` (`central()` header block)

**Interfaces:**
- Consumes: `illustration::monitor`, `illustration::aspect_chip`, `icons::draw`, `Icon`.
- Produces: `fn hero(&self, ui: &mut Ui, p: &Palette, doc: &Doc, actions: &mut Vec<Action>)`

- [ ] **Step 1: Replace the header card**

Layout: horizontal — left `monitor` at 240×160 (shrink to `available_width * 0.35` when narrow); right column: monitor name (22 px semibold), meta line (11 px muted), chips row (`EDID 1.3`, block count, digital/analog), `OVERRIDE ACTIVE` chip when present, and the `Remove override` ghost button pinned top-right. No clipping at 900 px width.

- [ ] **Step 2: Wire aspect from the active timing**

Use the first detailed timing's `hdisplay/vdisplay` ratio; fall back to 16/9. Draw `aspect_chip` next to the resolution text in the table rows (first column).

- [ ] **Step 3: Verify**

Build, run, screenshot; check 900×560 window (resize) and 125% DPI.

---

### Task 5: Sidebar, segmented tabs, toast, status bar

**Files:**
- Modify: `src/ui/app.rs`

- [ ] **Step 1: Sidebar rows**

Each row: drawn `Icon::Monitor` 16 px + name (13 px, ellipsized with `egui::text::LayoutJob` truncation) + resolution line (11 px muted) + active dot. Selected row: `p.card` fill, radius 14, inner hairline, 1 px accent left bar. Hover: `p.hover` at 60%.

- [ ] **Step 2: Tabs**

Replace the ad-hoc tab painting with `theme::tabs` segmented control; add drawn icons per tab (Detailed, Established, Extension, Raw) at 14 px.

- [ ] **Step 3: Toast**

Bottom-center above the status bar; fade + rise 8 px using `animate_bool_with_time(..., 0.18)`; drawn `Icon::Check`/`Icon::Alert`; auto-dismiss stays at the current 4 s; keep inside the content rect at 900×560.

- [ ] **Step 4: Status bar**

32 px, muted text, drawn `Icon::Shield` when elevated / `Icon::Alert` when not; registry path in 10 px monospace; keep all existing actions.

- [ ] **Step 5: Verify**

Build, run, screenshot dark + light; check toast placement and truncation with the longest monitor name (`RGT1352 (1&8713bca&0&U1...)`).

---

### Task 6: Rounded window corners

**Files:**
- Modify: `src/ui/win.rs`
- Modify: `src/ui/app.rs` (call site)

- [ ] **Step 1: Add the DWM attribute**

`pub fn set_rounded_corners(title: &str)` using `DwmSetWindowAttribute` with `DWMWA_WINDOW_CORNER_PREFERENCE = 33` and `DWMWCP_ROUND = 2`, `raw-dylib` like the existing dark-titlebar call. Ignore the error on Windows 10.

- [ ] **Step 2: Call it on the first frame**

Next to `set_dark_titlebar`. Verify visually (rounded corners on Win11).

---

### Task 7: Final verification and release build

- [ ] **Step 1: Tests and build**

`cargo test --lib` (8 existing) and `cargo test --bin cru-modern` (new illustration/icon tests) must pass; `cargo build` clean, no warnings.

- [ ] **Step 2: Visual pass**

Screenshots: dark 1120×720, light 1120×720, dark 900×560, and one dialog open. Check the Review Focus list.

- [ ] **Step 3: Release build**

`cargo build --release` → `target\release\cru-modern.exe`; report size and path.

- [ ] **Step 4: Docs**

Update `README.md` structure section with `icons.rs` and `illustration.rs`.
