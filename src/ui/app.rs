//! Application state and views.

use std::time::{Duration, Instant};

use egui::{
    Align, Align2, Color32, CornerRadius, FontFamily, Frame, Id, Layout, Margin, Pos2, Rect,
    RichText, Sense, Stroke, StrokeKind, TextStyle, Ui, Vec2,
};

use cru_modern::edid::{cea::Cta, timing, Descriptor, Edid, Timing, ESTABLISHED_GROUPS, ESTABLISHED_MODES};
use cru_modern::system::{self, DisplayInfo};

use super::icons::{self, Icon};
use super::illustration;
use super::theme::{self, Palette, ThemeMode, HAIRLINE, RADIUS_CARD};
use super::win;

pub const APP_TITLE: &str = "CRU Modern";

/// Minimal debug trace, active only in debug builds. Used to verify UI wiring
/// without guessing from screenshots.
#[cfg(debug_assertions)]
pub fn debug_log(message: impl AsRef<str>) {
    use std::io::Write;
    let path = std::env::temp_dir().join("cru-modern-debug.log");
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{}", message.as_ref());
    }
}

#[cfg(not(debug_assertions))]
pub fn debug_log(_message: impl AsRef<str>) {}

pub struct App {
    displays: Vec<DisplayInfo>,
    selected: Option<usize>,
    doc: Option<Doc>,
    tab: usize,
    mode: ThemeMode,
    selected_timing: Option<usize>,
    editor: Option<Editor>,
    toast: Option<Toast>,
    pending_select: Option<usize>,
    confirm_clear: bool,
    pending_restart: Option<String>,
    exit_at: Option<Instant>,
    first_frame: bool,
    admin: bool,
}

struct Doc {
    display: DisplayInfo,
    original: Vec<u8>,
    edid: Edid,
    source: String,
}

impl Doc {
    fn bytes(&self) -> Vec<u8> {
        self.edid.serialize()
    }

    fn dirty(&self) -> bool {
        self.bytes() != self.original
    }

    fn detailed(&self) -> Vec<(usize, Timing)> {
        self.edid
            .base
            .descriptors
            .iter()
            .enumerate()
            .filter_map(|(i, d)| match d {
                Descriptor::Detailed(t) => Some((i, *t)),
                _ => None,
            })
            .collect()
    }
}

struct Toast {
    text: String,
    error: bool,
    at: Instant,
}

struct Editor {
    index: Option<usize>,
    timing: Timing,
    refresh_target: f32,
    preset: usize,
    advanced: bool,
    error: Option<String>,
}

impl Editor {
    fn new(index: Option<usize>, timing: Timing) -> Self {
        Self {
            index,
            timing,
            refresh_target: timing.frame_hz() as f32,
            preset: 0,
            advanced: false,
            error: None,
        }
    }

    fn apply_preset(&mut self, preset: usize) {
        match preset {
            1 => {
                self.timing = timing::cvt_rb(
                    self.timing.hactive,
                    self.timing.vactive,
                    self.refresh_target as f64,
                );
            }
            n if n >= 2 => {
                if let Some(p) = timing::presets().get(n - 2) {
                    self.timing = p.timing;
                    self.refresh_target = p.refresh as f32;
                }
            }
            _ => {}
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Detailed,
    Established,
    Extensions,
    Raw,
}

const TABS: [(&str, Icon); 4] = [
    ("Detailed resolutions", Icon::Monitor),
    ("Established", Icon::Grid),
    ("Extension blocks", Icon::Stack),
    ("Raw EDID", Icon::Code),
];

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::load_fonts(&cc.egui_ctx);
        theme::apply_style(&cc.egui_ctx, ThemeMode::Dark);

        let displays = system::enumerate_displays();
        let selected = displays.iter().position(|d| d.active).or(if displays.is_empty() {
            None
        } else {
            Some(0)
        });

        let mut app = Self {
            displays,
            selected,
            doc: None,
            tab: 0,
            mode: ThemeMode::Dark,
            selected_timing: None,
            editor: None,
            toast: None,
            pending_select: None,
            confirm_clear: false,
            pending_restart: None,
            exit_at: None,
            first_frame: true,
            admin: system::is_elevated(),
        };
        app.load_document();
        if let Some(result) = system::take_last_restart(Duration::from_secs(180)) {
            match result {
                Ok(message) => app.notify(format!("{message}. Noul EDID este activ."), false),
                Err(err) => app.notify(err, true),
            }
        }
        debug_log(format!(
            "app started pid={} elevated={} displays={}",
            std::process::id(),
            app.admin,
            app.displays.len()
        ));
        app
    }

    fn palette(&self) -> Palette {
        self.mode.palette()
    }

    fn notify(&mut self, text: impl Into<String>, error: bool) {
        self.toast = Some(Toast {
            text: text.into(),
            error,
            at: Instant::now(),
        });
    }

    fn refresh_displays(&mut self) {
        let current_key = self.doc.as_ref().map(|d| d.display.key_path.clone());
        self.displays = system::enumerate_displays();
        self.selected = current_key
            .and_then(|key| self.displays.iter().position(|d| d.key_path == key))
            .or_else(|| self.displays.iter().position(|d| d.active))
            .or(if self.displays.is_empty() { None } else { Some(0) });
        self.load_document();
    }

    fn load_document(&mut self) {
        self.selected_timing = None;
        self.doc = None;
        let Some(index) = self.selected else {
            return;
        };
        let Some(display) = self.displays.get(index).cloned() else {
            return;
        };

        let active_bytes = display
            .override_edid
            .as_ref()
            .filter(|b| !b.is_empty())
            .unwrap_or(&display.edid)
            .clone();

        match Edid::parse(&active_bytes) {
            Ok(edid) => {
                self.doc = Some(Doc {
                    original: active_bytes,
                    display,
                    edid,
                    source: "Registry".to_string(),
                });
            }
            Err(err) => self.notify(format!("Cannot parse EDID: {err}"), true),
        }
    }

    /// Starts the driver restart. The GUI cannot survive the adapter reset with
    /// a live OpenGL context, so a helper process does the restart after this
    /// process exits, then relaunches the app.
    fn start_restart(&mut self, name: String) {
        debug_log(format!("start_restart name={name:?}"));
        if self.exit_at.is_some() {
            self.notify("Restartul driverului e deja în desfășurare...", true);
            return;
        }

        match system::spawn_restart_helper() {
            Ok(()) => {
                if name.is_empty() {
                    self.notify("Se repornește driverul grafic. Aplicația va porni din nou automat...", false);
                } else {
                    self.notify(
                        format!("Se repornește driverul grafic pentru {name}. Aplicația va porni din nou automat..."),
                        false,
                    );
                }
                self.exit_at = Some(Instant::now() + Duration::from_millis(900));
            }
            Err(err) => self.notify(err, true),
        }
    }

fn request_select(&mut self, index: usize) {
        if index == self.selected.unwrap_or(usize::MAX) {
            return;
        }
        if self.doc.as_ref().is_some_and(|d| d.dirty()) {
            self.pending_select = Some(index);
        } else {
            self.selected = Some(index);
            self.load_document();
        }
    }

    fn apply(&mut self) {
        let (bytes, display, admin) = {
            let Some(doc) = &self.doc else { return };
            (doc.bytes(), doc.display.clone(), self.admin)
        };

        if !admin {
            self.notify("Drepturi de administrator necesare pentru a scrie substituirea EDID", true);
            return;
        }

        let name = display.monitor_name();
        match system::write_override(&display, &bytes) {
            Ok(()) => {
                if let Some(doc) = &mut self.doc {
                    doc.original = bytes;
                    doc.display.override_edid = Some(doc.original.clone());
                }
                if let Some(slot) = self.displays.iter_mut().find(|d| d.key_path == display.key_path) {
                    slot.override_edid = Some(self.doc.as_ref().unwrap().original.clone());
                }
                self.pending_restart = Some(name.clone());
                self.notify(format!("Override scris pentru {name}. Repornești driverul pentru a-l activa."), false);
            }
            Err(err) => self.notify(err, true),
        }
    }

    fn clear_override(&mut self) {
        let Some(doc) = &self.doc else { return };
        if !self.admin {
            self.notify("Administrator rights required to modify the EDID override", true);
            return;
        }
        match system::clear_override(&doc.display) {
            Ok(()) => {
                if let Some(doc) = &mut self.doc {
                    doc.display.override_edid = None;
                    doc.original = doc.display.edid.clone();
                    if let Ok(edid) = Edid::parse(&doc.original) {
                        doc.edid = edid;
                    }
                }
                self.refresh_displays();
                self.notify("Override removed; the monitor's own EDID is used again", false);
            }
            Err(err) => self.notify(err, true),
        }
    }

    fn revert(&mut self) {
        if let Some(doc) = &mut self.doc {
            if let Ok(edid) = Edid::parse(&doc.original) {
                doc.edid = edid;
                self.selected_timing = None;
                self.notify("Changes reverted", false);
            }
        }
    }

    fn import(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("EDID binary", &["bin", "dat", "edid"])
            .set_title("Import EDID")
            .pick_file()
        else {
            return;
        };
        match std::fs::read(&path) {
            Ok(bytes) => {
                if !self.admin {
                    // Importing only changes the working copy, no elevation needed.
                }
                match Edid::parse(&bytes) {
                    Ok(edid) => {
                        let Some(doc) = &mut self.doc else { return };
                        doc.edid = edid;
                        doc.original = bytes;
                        doc.source = path.display().to_string();
                        self.selected_timing = None;
                        self.notify(format!("Imported {}", path.file_name().unwrap_or_default().to_string_lossy()), false);
                    }
                    Err(err) => self.notify(format!("Invalid EDID file: {err}"), true),
                }
            }
            Err(err) => self.notify(format!("Cannot read file: {err}"), true),
        }
    }

    fn export(&mut self) {
        let Some(doc) = &self.doc else { return };
        let default_name = format!("{}.bin", doc.display.hardware_id);
        let Some(path) = rfd::FileDialog::new()
            .add_filter("EDID binary", &["bin"])
            .set_file_name(default_name)
            .set_title("Export EDID")
            .save_file()
        else {
            return;
        };
        match std::fs::write(&path, doc.bytes()) {
            Ok(()) => self.notify(format!("Saved {}", path.display()), false),
            Err(err) => self.notify(format!("Cannot write file: {err}"), true),
        }
    }

    fn open_editor(&mut self, index: Option<usize>) {
        let Some(doc) = &self.doc else { return };
        let timing = match index {
            Some(i) => match &doc.edid.base.descriptors[i] {
                Descriptor::Detailed(t) => *t,
                _ => return,
            },
            None => {
                let current = doc
                    .detailed()
                    .first()
                    .map(|(_, t)| *t)
                    .unwrap_or_default();
                timing::cvt_rb(current.hactive.max(1920), current.vactive.max(1080), 60.0)
            }
        };
        self.editor = Some(Editor::new(index, timing));
    }

    fn save_editor(&mut self) {
        let Some(editor) = &self.editor else { return };
        let Some(doc) = &mut self.doc else { return };

        let timing = editor.timing;
        if let Some(err) = editor.error.clone() {
            self.notify(err, true);
            return;
        }

        match editor.index {
            Some(index) => doc.edid.base.descriptors[index] = Descriptor::Detailed(timing),
            None => match doc.edid.free_descriptor_slot() {
                Some(slot) => doc.edid.base.descriptors[slot] = Descriptor::Detailed(timing),
                None => {
                    self.notify(
                        "No free descriptor slot. Delete a timing or a text descriptor first.",
                        true,
                    );
                    return;
                }
            },
        }
        self.editor = None;
        self.notify(format!("{} added to the working copy", timing.label()), false);
    }

    fn delete_timing(&mut self, index: usize) {
        let Some(doc) = &mut self.doc else { return };
        if doc.edid.base.descriptors[index].is_removable() {
            doc.edid.base.descriptors[index] = Descriptor::Dummy;
            self.selected_timing = None;
        }
    }

    fn move_timing(&mut self, index: usize, delta: i32) {
        let Some(doc) = &mut self.doc else { return };
        let target = index as i32 + delta;
        if target < 0 || target >= doc.edid.base.descriptors.len() as i32 {
            return;
        }
        let target = target as usize;
        doc.edid.base.descriptors.swap(index, target);
        self.selected_timing = Some(target);
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.first_frame {
            self.first_frame = false;
            win::set_dark_titlebar(APP_TITLE, self.mode == ThemeMode::Dark);
            win::set_rounded_corners(APP_TITLE);
        }

        if let Some(deadline) = self.exit_at {
            if Instant::now() >= deadline {
                debug_log("closing GUI for driver restart");
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            ctx.request_repaint_after(Duration::from_millis(50));
        }

        let p = self.palette();
        let mut actions: Vec<Action> = Vec::new();

        self.top_bar(ctx, &p, &mut actions);
        self.status_bar(ctx, &p, &mut actions);
        self.sidebar(ctx, &p, &mut actions);
        self.central(ctx, &p, &mut actions);

        self.dialogs(ctx, &p, &mut actions);

        if let Some(toast) = &self.toast {
            if toast.at.elapsed() > Duration::from_secs(4) {
                self.toast = None;
            } else {
                ctx.request_repaint_after(Duration::from_millis(16));
            }
        }

        for action in actions {
            self.handle(ctx, action);
        }

        #[cfg(debug_assertions)]
        if std::env::var_os("CRU_ICON_SHEET").is_some() {
            paint_icon_sheet(ctx, &p);
        }
    }
}

/// Dev-only sheet used to review the icon set at real size.
#[cfg(debug_assertions)]
fn paint_icon_sheet(ctx: &egui::Context, p: &Palette) {
    use super::icons;
    use egui::{FontId, LayerId, Order};

    let painter = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("icon_sheet")));
    let card = egui::Rect::from_min_size(
        ctx.screen_rect().min + egui::vec2(28.0, 92.0),
        egui::vec2(icons::ALL.len() as f32 * 54.0 + 36.0, 104.0),
    );
    painter.rect_filled(card, CornerRadius::same(16), p.card);
    painter.rect_stroke(
        card,
        CornerRadius::same(16),
        Stroke::new(1.0_f32, p.border),
        egui::StrokeKind::Inside,
    );
    for (index, icon) in icons::ALL.iter().enumerate() {
        let x = card.left() + 18.0 + index as f32 * 54.0;
        let rect =
            egui::Rect::from_min_size(egui::pos2(x, card.top() + 20.0), Vec2::splat(26.0));
        icons::draw(*icon, &painter, rect, p.text);
        painter.text(
            egui::pos2(rect.center().x, card.bottom() - 20.0),
            Align2::CENTER_CENTER,
            format!("{icon:?}"),
            FontId::proportional(9.0),
            p.muted,
        );
    }
}

#[derive(Debug)]
enum Action {
    Select(usize),
    Apply,
    Revert,
    ClearOverride,
    RestartDriver,
    RelaunchAdmin,
    Refresh,
    Import,
    Export,
    OpenEditor(Option<usize>),
    DeleteTiming(usize),
    MoveTiming(usize, i32),
    SaveEditor,
    CloseEditor,
    ToggleEstablished(u8, bool),
    ToggleTheme,
    ConfirmSelect,
    CancelSelect,
    OpenClearConfirm,
    ConfirmRestart,
}

impl App {
    fn handle(&mut self, ctx: &egui::Context, action: Action) {
        debug_log(format!("action: {action:?}"));
        match action {
            Action::Select(index) => self.request_select(index),
            Action::Apply => self.apply(),
            Action::Revert => self.revert(),
            Action::ClearOverride => self.clear_override(),
            Action::RestartDriver => {
                if let Some(name) = self.pending_restart.take() {
                    self.start_restart(name);
                } else {
                    self.start_restart(String::new());
                }
            }
            Action::RelaunchAdmin => {
                if let Err(err) = system::relaunch_elevated() {
                    self.notify(err, true);
                } else {
                    std::process::exit(0);
                }
            }
            Action::Refresh => self.refresh_displays(),
            Action::Import => self.import(),
            Action::Export => self.export(),
            Action::OpenEditor(index) => self.open_editor(index),
            Action::DeleteTiming(index) => self.delete_timing(index),
            Action::MoveTiming(index, delta) => self.move_timing(index, delta),
            Action::SaveEditor => self.save_editor(),
            Action::CloseEditor => self.editor = None,
            Action::ToggleEstablished(bit, value) => {
                if let Some(doc) = &mut self.doc {
                    doc.edid.base.set_established(bit, value);
                }
            }
            Action::ToggleTheme => {
                self.mode = match self.mode {
                    ThemeMode::Dark => ThemeMode::Light,
                    ThemeMode::Light => ThemeMode::Dark,
                };
                theme::apply_style(ctx, self.mode);
                win::set_dark_titlebar(APP_TITLE, self.mode == ThemeMode::Dark);
            }
            Action::ConfirmSelect => {
                if let Some(index) = self.pending_select.take() {
                    self.selected = Some(index);
                    self.load_document();
                }
            }
            Action::CancelSelect => self.pending_select = None,
            Action::OpenClearConfirm => self.confirm_clear = true,
            Action::ConfirmRestart => {
                if let Some(name) = self.pending_restart.take() {
                    self.start_restart(name);
                } else {
                    self.start_restart(String::new());
                }
            }
        }
    }

    // The theme needs the egui context; it is passed into `handle`.

    fn top_bar(&mut self, ctx: &egui::Context, p: &Palette, actions: &mut Vec<Action>) {
        egui::TopBottomPanel::top("topbar")
            .exact_height(58.0)
            .frame(
                Frame::NONE
                    .fill(p.bg_top)
                    .inner_margin(Margin::symmetric(16, 0)),
            )
            .show(ctx, |ui| {
                let panel_rect = ui.max_rect();
                ui.painter().line_segment(
                    [panel_rect.left_bottom(), panel_rect.right_bottom()],
                    Stroke::new(HAIRLINE, p.border.gamma_multiply(0.75)),
                );
                ui.horizontal_centered(|ui| {
                    let (badge, _) = ui.allocate_exact_size(Vec2::new(30.0, 30.0), Sense::hover());
                    ui.painter()
                        .rect_filled(badge, CornerRadius::same(9), p.accent);
                    ui.painter().rect_filled(
                        Rect::from_min_max(badge.min, Pos2::new(badge.max.x, badge.center().y)),
                        CornerRadius::same(9),
                        p.accent_top,
                    );
                    ui.painter().line_segment(
                        [
                            Pos2::new(badge.left() + 6.0, badge.top() + 1.0),
                            Pos2::new(badge.right() - 6.0, badge.top() + 1.0),
                        ],
                        Stroke::new(HAIRLINE, Color32::from_white_alpha(65)),
                    );
                    icons::draw(Icon::Logo, ui.painter(), badge.shrink(5.0), p.accent_text);

                    ui.add_space(10.0);
                    ui.vertical(|ui| {
                        ui.add_space(11.0);
                        ui.spacing_mut().item_spacing.y = 1.0;
                        ui.label(
                            RichText::new("CRU Modern")
                                .size(15.0)
                                .family(FontFamily::Name("semibold".into()))
                                .color(p.text),
                        );
                        ui.label(
                            RichText::new("Custom Resolution Utility")
                                .size(11.0)
                                .color(p.faint),
                        );
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let restarting = self.exit_at.is_some();
                        let apply_enabled = !restarting && self.doc.as_ref().is_some_and(|d| d.dirty());
                        if theme::primary_button(ui, p, "Apply", apply_enabled).clicked() {
                            actions.push(Action::Apply);
                        }
                        if theme::ghost_button(
                            ui,
                            p,
                            "Revert",
                            self.doc.as_ref().is_some_and(|d| d.dirty()),
                        )
                        .clicked()
                        {
                            actions.push(Action::Revert);
                        }
                        let label = if restarting {
                            "Restarting…"
                        } else {
                            "Restart driver"
                        };
                        if theme::ghost_button(ui, p, label, !restarting).clicked() {
                            actions.push(Action::RestartDriver);
                        }
                        ui.add_space(4.0);
                        let icon = if self.mode == ThemeMode::Dark {
                            Icon::Sun
                        } else {
                            Icon::Moon
                        };
                        if theme::icon_button(ui, p, icon, "Toggle theme").clicked() {
                            actions.push(Action::ToggleTheme);
                        }
                    });
                });
            });
    }

    fn status_bar(&mut self, ctx: &egui::Context, p: &Palette, actions: &mut Vec<Action>) {
        egui::TopBottomPanel::bottom("status")
            .exact_height(32.0)
            .frame(
                Frame::NONE
                    .fill(p.bg_bottom)
                    .inner_margin(Margin::symmetric(16, 0)),
            )
            .show(ctx, |ui| {
                let panel_rect = ui.max_rect();
                ui.painter().line_segment(
                    [panel_rect.left_top(), panel_rect.right_top()],
                    Stroke::new(HAIRLINE, p.border.gamma_multiply(0.75)),
                );
                ui.horizontal_centered(|ui| {
                    if self.exit_at.is_some() {
                        theme::chip(ui, p, "SE REPORNESTE DRIVERUL", p.warning);
                    } else if self.admin {
                        let (icon_rect, _) =
                            ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                        icons::draw(Icon::Shield, ui.painter(), icon_rect, p.success);
                        ui.label(
                            RichText::new("ADMIN")
                                .size(11.0)
                                .family(FontFamily::Name("semibold".into()))
                                .color(p.success),
                        );
                    } else {
                        let (icon_rect, _) =
                            ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                        icons::draw(Icon::Alert, ui.painter(), icon_rect, p.warning);
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Run as administrator")
                                        .size(11.5)
                                        .family(FontFamily::Name("semibold".into()))
                                        .color(p.accent),
                                )
                                .fill(Color32::TRANSPARENT)
                                .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            actions.push(Action::RelaunchAdmin);
                        }
                    }

                    ui.add_space(8.0);
                    if let Some(doc) = &self.doc {
                        let text = if doc.source == "Registry" {
                            format!("HKLM\\{}", doc.display.key_path)
                        } else {
                            doc.source.clone()
                        };
                        let avail = (ui.available_width() - 130.0).max(60.0);
                        let galley = theme::ellipsize_galley(
                            ui.painter(),
                            &text,
                            theme::mono(10.5),
                            p.faint,
                            avail,
                        );
                        let (r, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
                        ui.painter().galley(r.min, galley, p.faint);
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if let Some(doc) = &self.doc {
                            let status = if doc.dirty() {
                                "Unsaved changes"
                            } else {
                                "Up to date"
                            };
                            let color = if doc.dirty() { p.warning } else { p.success };
                            ui.label(RichText::new(status).size(11.5).color(color));
                            let (dot_rect, _) =
                                ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                            ui.painter().circle_filled(dot_rect.center(), 3.2, color);
                        }
                    });
                });
            });
    }

    fn sidebar(&mut self, ctx: &egui::Context, p: &Palette, actions: &mut Vec<Action>) {
        egui::SidePanel::left("sidebar")
            .exact_width(276.0)
            .resizable(false)
            .frame(
                Frame::NONE
                    .fill(p.panel)
                    .inner_margin(Margin::symmetric(14, 14)),
            )
            .show(ctx, |ui| {
                let panel_rect = ui.max_rect();
                ui.painter().line_segment(
                    [panel_rect.right_top(), panel_rect.right_bottom()],
                    Stroke::new(HAIRLINE, p.border.gamma_multiply(0.75)),
                );
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("DISPLAYS")
                            .text_style(TextStyle::Name("caps".into()))
                            .color(p.faint),
                    );
                    ui.add_space(2.0);
                    theme::chip(ui, p, &self.displays.len().to_string(), p.muted);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if theme::icon_button(ui, p, Icon::Refresh, "Rescan displays").clicked() {
                            actions.push(Action::Refresh);
                        }
                    });
                });
                ui.add_space(10.0);

                let height = (ui.available_height() - 46.0).max(120.0);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .max_height(height)
                    .show(ui, |ui| {
                        if self.displays.is_empty() {
                            theme::muted(ui, p, "No monitors with an EDID were found.");
                        }
                        for (index, display) in self.displays.iter().enumerate() {
                            let selected = self.selected == Some(index);
                            if display_card(ui, p, index, display, selected).clicked() {
                                actions.push(Action::Select(index));
                            }
                            ui.add_space(5.0);
                        }
                    });

                ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                    ui.horizontal(|ui| {
                        let width = (ui.available_width() - 8.0) / 2.0;
                        if theme::ghost_button_icon_sized(ui, p, Icon::Import, "Import .bin", width)
                            .clicked()
                        {
                            actions.push(Action::Import);
                        }
                        if theme::ghost_button_icon_sized(ui, p, Icon::Export, "Export .bin", width)
                            .clicked()
                        {
                            actions.push(Action::Export);
                        }
                    });
                });
            });
    }

    fn central(&mut self, ctx: &egui::Context, p: &Palette, actions: &mut Vec<Action>) {
        egui::CentralPanel::default()
            .frame(Frame::NONE.inner_margin(Margin::symmetric(22, 18)))
            .show(ctx, |ui| {
                // Vertical Sequoia gradient background behind everything.
                let full = ui.max_rect().expand(24.0);
                let bg = theme::background_gradient(p, full);
                ui.painter().add(egui::epaint::Shape::Mesh(std::sync::Arc::new(bg)));

                let Some(doc) = &self.doc else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(90.0);
                        theme::h1(ui, p, "No display selected");
                        theme::muted(ui, p, "Pick a monitor from the list or import an EDID file.");
                    });
                    return;
                };

                self.hero(ui, p, doc, actions);
                ui.add_space(14.0);

                let mut index = self.tab;
                theme::tabs_with_icons(ui, p, &mut index, &TABS);
                self.tab = index;
                let tab = match self.tab {
                    0 => Tab::Detailed,
                    1 => Tab::Established,
                    2 => Tab::Extensions,
                    _ => Tab::Raw,
                };
                ui.add_space(12.0);

                match tab {
                    Tab::Detailed => self.detailed_tab(ui, p, actions),
                    Tab::Established => self.established_tab(ui, p, actions),
                    Tab::Extensions => self.extensions_tab(ui, p),
                    Tab::Raw => self.raw_tab(ui, p),
                }
            });
    }

    fn hero(&self, ui: &mut Ui, p: &Palette, doc: &Doc, actions: &mut Vec<Action>) {
        let first_timing = doc.detailed().first().map(|(_, t)| *t);
        let aspect = doc
            .display
            .current_mode
            .filter(|(w, h, _)| *w > 0 && *h > 0)
            .map(|(w, h, _)| w as f32 / h as f32)
            .or_else(|| first_timing.map(|t| t.hactive as f32 / t.vactive as f32))
            .unwrap_or(16.0 / 9.0);

        let (res_text, hz_text) = if let Some((w, h, r)) = doc.display.current_mode {
            (format!("{w} × {h}"), format!("{r} Hz"))
        } else if let Some(t) = first_timing {
            (
                format!("{} × {}", t.hactive, t.vactive),
                format!("{} Hz", t.frame_hz()),
            )
        } else {
            ("No timing".to_string(), String::new())
        };

        let multi_display = self.displays.iter().filter(|d| d.active).count() > 1;
        let card_h = 172.0;
        let avail_w = ui.available_width();
        let left_w = (avail_w * 0.34).clamp(198.0, 268.0);
        let right_w = (avail_w - left_w - 14.0).max(240.0);
        let dark = p.bg_top.r() < 128;

        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 14.0;

            // Left: hero vector monitor card.
            let (left_rect, _) = ui.allocate_exact_size(Vec2::new(left_w, card_h), Sense::hover());
            ui.painter().rect_filled(
                left_rect.translate(Vec2::new(0.0, 5.0)),
                CornerRadius::same(RADIUS_CARD),
                p.shadow.gamma_multiply(if dark { 0.45 } else { 0.35 }),
            );
            theme::paint_card_surface(ui.painter(), left_rect, RADIUS_CARD, p);
            illustration::monitor(
                ui,
                p,
                left_rect.shrink2(Vec2::new(12.0, 10.0)),
                &res_text,
                &hz_text,
                aspect,
                multi_display,
            );

            // Right: display details bento card.
            let (right_rect, _) = ui.allocate_exact_size(Vec2::new(right_w, card_h), Sense::hover());
            ui.painter().rect_filled(
                right_rect.translate(Vec2::new(0.0, 5.0)),
                CornerRadius::same(RADIUS_CARD),
                p.shadow.gamma_multiply(if dark { 0.45 } else { 0.35 }),
            );
            theme::paint_card_surface(ui.painter(), right_rect, RADIUS_CARD, p);

            let inner_rect = right_rect.shrink(18.0);
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(inner_rect)
                    .layout(Layout::top_down(Align::LEFT)),
            );
            child.horizontal(|ui| {
                let button_reserve = if doc.display.has_override() { 140.0 } else { 0.0 };
                let title_max_w = (ui.available_width() - button_reserve).max(120.0);
                let title_galley = theme::ellipsize_galley(
                    ui.painter(),
                    &doc.display.monitor_name(),
                    theme::semibold(21.0),
                    p.text,
                    title_max_w,
                );
                let (tr, _) = ui.allocate_exact_size(title_galley.size(), Sense::hover());
                ui.painter().galley(tr.min, title_galley, p.text);

                if doc.display.has_override() {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if theme::ghost_button(ui, p, "Remove override", true).clicked() {
                            actions.push(Action::OpenClearConfirm);
                        }
                    });
                }
            });

            child.add_space(3.0);
            let base = &doc.edid.base;
            let serial = if base.serial != 0 {
                format!("  ·  S/N {}", base.serial)
            } else {
                String::new()
            };
            let meta = format!(
                "{}  ·  {}  ·  0x{:04X}{}{}",
                base.manufacturer(),
                doc.display.hardware_id,
                base.product_code,
                serial,
                if doc.display.active { "" } else { "  ·  inactive" }
            );
            let meta_galley = theme::ellipsize_galley(
                child.painter(),
                &meta,
                theme::font(11.5),
                p.muted,
                inner_rect.width(),
            );
            let (mr, _) = child.allocate_exact_size(meta_galley.size(), Sense::hover());
            child.painter().galley(mr.min, meta_galley, p.muted);

            child.add_space(12.0);
            child.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
                theme::chip(
                    ui,
                    p,
                    &format!("EDID {}.{}", base.version.0, base.version.1),
                    p.muted,
                );
                theme::chip(
                    ui,
                    p,
                    &format!("{} block(s)", doc.edid.block_count()),
                    p.muted,
                );
                theme::chip(
                    ui,
                    p,
                    &format!(
                        "{}  ·  {}",
                        if base.digital() { "Digital" } else { "Analog" },
                        base.interface()
                    ),
                    p.muted,
                );
                if doc.dirty() {
                    theme::chip(ui, p, "MODIFIED", p.warning);
                }
                if doc.display.has_override() {
                    theme::chip(ui, p, "OVERRIDE ACTIVE", p.accent);
                }
            });

            child.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                let summary = if let Some((w, h, r)) = doc.display.current_mode {
                    format!("Active signal  {w} × {h} @ {r} Hz  ·  {}", illustration::aspect_label(aspect))
                } else {
                    format!("Preferred timing  {res_text}  ·  {}", illustration::aspect_label(aspect))
                };
                ui.label(RichText::new(summary).size(11.5).color(p.faint));
            });
        });
    }

    fn detailed_tab(&mut self, ui: &mut Ui, p: &Palette, actions: &mut Vec<Action>) {
        let Some(doc) = &self.doc else { return };
        let rows = doc.detailed();
        let max_clock = doc.edid.max_pixel_clock_khz();
        let max_hfreq = doc.edid.max_hfreq_khz();

        theme::card_panel(ui, *p, |ui| {
            ui.horizontal(|ui| {
                theme::section_title(ui, *p, "Detailed resolutions");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let preview = FormatPreview {
                        max_clock,
                        max_hfreq,
                    };
                    preview.show(ui, *p);
                });
            });

            let w = ui.available_width();
            let c0 = (w * 0.30).clamp(155.0, 210.0);
            let c1 = (w * 0.16).clamp(88.0, 118.0);
            let c2 = (w * 0.17).clamp(92.0, 122.0);
            let c3 = (w * 0.25).clamp(130.0, 185.0);
            let c4 = (w - c0 - c1 - c2 - c3).max(52.0);
            let columns = theme::Columns::new(&[c0, c1, c2, c3, c4]);

            ui.horizontal(|ui| {
                ui.add_space(10.0);
                for (index, cell) in ["Resolution", "Refresh", "Clock", "Sync", "Polarity"]
                    .iter()
                    .enumerate()
                {
                    let width = columns.widths.get(index).copied().unwrap_or(100.0);
                    ui.allocate_ui(Vec2::new(width, 18.0), |ui| {
                        ui.label(RichText::new(*cell).size(11.0).color(p.faint));
                    });
                }
            });
            theme::divider(ui, *p);

            if rows.is_empty() {
                ui.add_space(10.0);
                theme::muted(ui, p, "No detailed resolutions in the base block.");
                ui.add_space(10.0);
            }

            let mut clicked = None;
            for (descriptor_index, timing) in &rows {
                let selected = self.selected_timing == Some(*descriptor_index);
                let ar = if timing.vactive > 0 {
                    timing.hactive as f32 / timing.vactive as f32
                } else {
                    16.0 / 9.0
                };
                let cells = [
                    format!(
                        "{} × {}{}",
                        timing.hactive,
                        timing.vactive,
                        if timing.interlaced { "i" } else { "" }
                    ),
                    format!("{:.2} Hz", timing.frame_hz()),
                    format!("{:.2} MHz", timing.pixel_clock_khz as f64 / 1000.0),
                    timing.sync_label(),
                    timing.polarity_label().to_string(),
                ];
                let response = theme::table_row(
                    ui,
                    *p,
                    Id::new(("timing", descriptor_index)),
                    &cells,
                    &columns,
                    selected,
                    true,
                    32.0,
                    13.0,
                    Some(ar),
                );
                if response.clicked() {
                    clicked = Some(*descriptor_index);
                }
                if response.double_clicked() {
                    actions.push(Action::OpenEditor(Some(*descriptor_index)));
                }
            }
            if let Some(index) = clicked {
                self.selected_timing = Some(index);
            }

            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if theme::ghost_button_icon(ui, p, Icon::Plus, "Add…", true).clicked() {
                    actions.push(Action::OpenEditor(None));
                }
                let selected = self.selected_timing;
                if theme::ghost_button(ui, p, "Edit…", selected.is_some()).clicked() {
                    if let Some(index) = selected {
                        actions.push(Action::OpenEditor(Some(index)));
                    }
                }
                if theme::ghost_button_icon(ui, p, Icon::Minus, "Delete", selected.is_some())
                    .clicked()
                {
                    if let Some(index) = selected {
                        actions.push(Action::DeleteTiming(index));
                    }
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if theme::icon_button(ui, p, Icon::ArrowDown, "Move down").clicked() {
                        if let Some(index) = selected {
                            actions.push(Action::MoveTiming(index, 1));
                        }
                    }
                    if theme::icon_button(ui, p, Icon::ArrowUp, "Move up").clicked() {
                        if let Some(index) = selected {
                            actions.push(Action::MoveTiming(index, -1));
                        }
                    }
                });
            });
        });
    }

    fn established_tab(&mut self, ui: &mut Ui, p: &Palette, actions: &mut Vec<Action>) {
        let Some(doc) = &self.doc else { return };
        let established = doc.edid.base.established;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                theme::card_panel(ui, *p, |ui| {
                    theme::section_title(ui, *p, "Established timings");
                    ui.add_space(4.0);
                    for (group, indices) in ESTABLISHED_GROUPS {
                        ui.label(
                            RichText::new(group)
                                .size(12.0)
                                .family(FontFamily::Name("semibold".into()))
                                .color(p.muted),
                        );
                        ui.horizontal_wrapped(|ui| {
                            for bit in indices {
                                let label = ESTABLISHED_MODES[*bit as usize];
                                let mut checked =
                                    established[(*bit / 8) as usize] & (0x80 >> (bit % 8)) != 0;
                                if ui.checkbox(&mut checked, label).changed() {
                                    actions.push(Action::ToggleEstablished(*bit, checked));
                                }
                            }
                        });
                        ui.add_space(8.0);
                    }
                });
                ui.add_space(12.0);
                theme::card_panel(ui, *p, |ui| {
                    theme::section_title(ui, *p, "Standard timings");
                    ui.add_space(4.0);
                    let mut any = false;
                    for (index, st) in doc.edid.base.standard.iter().enumerate() {
                        if st.hactive == 0 {
                            continue;
                        }
                        any = true;
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("{:>2}.", index + 1))
                                    .size(12.0)
                                    .color(p.faint),
                            );
                            ui.label(RichText::new(st.label()).size(13.0).color(p.text));
                        });
                    }
                    if !any {
                        theme::muted(ui, p, "No standard timings declared.");
                    }
                });
            });
    }

    fn extensions_tab(&mut self, ui: &mut Ui, p: &Palette) {
        let Some(doc) = &self.doc else { return };
        if doc.edid.extensions.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                theme::muted(ui, p, "This EDID has no extension blocks.");
            });
            return;
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (index, block) in doc.edid.extensions.iter().enumerate() {
                    let (title, color) = match block.first().copied().unwrap_or(0) {
                        0x02 => ("CTA-861 extension".to_string(), p.accent),
                        0x10 => ("Video timing block extension".to_string(), p.muted),
                        0x40 => ("Display information extension".to_string(), p.muted),
                        0x50 => ("Localized string extension".to_string(), p.muted),
                        0x70 => ("DisplayID extension".to_string(), p.muted),
                        0xF0 => ("Block map extension".to_string(), p.muted),
                        tag => (format!("Extension 0x{tag:02X}"), p.muted),
                    };

                    theme::card_panel(ui, *p, |ui| {
                        ui.horizontal(|ui| {
                            theme::h2(ui, p, &title);
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                theme::chip(ui, p, &format!("block {}", index + 1), color);
                            });
                        });

                        if let Some(cta) = Cta::parse(block) {
                            ui.add_space(6.0);
                            ui.horizontal_wrapped(|ui| {
                                theme::chip(ui, p, &format!("CTA rev {}", cta.revision), p.muted);
                                if cta.basic_audio {
                                    theme::chip(ui, p, "basic audio", p.muted);
                                }
                                if cta.ycbcr444 {
                                    theme::chip(ui, p, "YCbCr 4:4:4", p.muted);
                                }
                                if cta.ycbcr422 {
                                    theme::chip(ui, p, "YCbCr 4:2:2", p.muted);
                                }
                                theme::chip(
                                    ui,
                                    p,
                                    &format!("{} native DTD(s)", cta.native_dtds),
                                    p.muted,
                                );
                            });
                            ui.add_space(10.0);

                            for data in &cta.data_blocks {
                                ui.horizontal_top(|ui| {
                                    ui.allocate_ui(Vec2::new(150.0, 18.0), |ui| {
                                        ui.label(
                                            RichText::new(&data.name)
                                                .size(12.5)
                                                .family(FontFamily::Name("semibold".into()))
                                                .color(p.text),
                                        );
                                    });
                                    ui.label(
                                        RichText::new(&data.detail).size(12.5).color(p.muted),
                                    );
                                });
                            }

                            if !cta.dtds.is_empty() {
                                ui.add_space(8.0);
                                theme::divider(ui, *p);
                                ui.add_space(4.0);
                                theme::muted(ui, p, "Detailed timings in this block (read-only)");
                                for t in &cta.dtds {
                                    ui.label(
                                        RichText::new(format!("• {}", t.label()))
                                            .size(13.0)
                                            .color(p.text),
                                    );
                                }
                            }
                        } else {
                            ui.add_space(6.0);
                            theme::muted(ui, p, "Binary block, not decoded.");
                        }
                    });
                    ui.add_space(10.0);
                }
            });
    }

    fn raw_tab(&mut self, ui: &mut Ui, p: &Palette) {
        let Some(doc) = &self.doc else { return };
        let bytes = doc.bytes();
        let checksums = doc.edid.checksum_ok();

        theme::card_panel(ui, *p, |ui| {
            ui.horizontal(|ui| {
                theme::section_title(ui, *p, "Raw EDID");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let ok = checksums.iter().all(|c| *c);
                    theme::chip(
                        ui,
                        p,
                        if ok { "CHECKSUMS OK" } else { "BAD CHECKSUM" },
                        if ok { p.success } else { p.danger },
                    );
                    theme::chip(ui, p, &format!("{} bytes", bytes.len()), p.muted);
                });
            });

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(ui.available_height() - 10.0)
                .show(ui, |ui| {
                    for (block_index, block) in bytes.chunks(128).enumerate() {
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new(format!(
                                "Block {}  ·  checksum {}",
                                block_index,
                                if *checksums.get(block_index).unwrap_or(&false) {
                                    "valid"
                                } else {
                                    "invalid"
                                }
                            ))
                            .size(11.5)
                            .color(p.muted),
                        );
                        for (line_index, line) in block.chunks(16).enumerate() {
                            let hex: Vec<String> =
                                line.iter().map(|b| format!("{b:02X}")).collect();
                            ui.label(
                                RichText::new(format!(
                                    "{:04X}  {}",
                                    0x80 * block_index + line_index * 16,
                                    hex.join(" ")
                                ))
                                .text_style(TextStyle::Name("mono-small".into()))
                                .color(p.text),
                            );
                        }
                    }
                });
        });
    }

    fn dialogs(&mut self, ctx: &egui::Context, p: &Palette, actions: &mut Vec<Action>) {
        if let Some(index) = self.pending_select {
            let name = self
                .displays
                .get(index)
                .map(|d| d.monitor_name())
                .unwrap_or_default();
            let mut open = true;
            egui::Window::new("Discard changes?")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .frame(dialog_frame(*p))
                .show(ctx, |ui| {
                    ui.set_width(360.0);
                    ui.label(
                        RichText::new(format!("You have unsaved changes for the current monitor. Switch to {name}?"))
                            .size(13.0)
                            .color(p.text),
                    );
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if theme::ghost_button(ui, p, "Stay here", true).clicked() {
                            actions.push(Action::CancelSelect);
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if theme::primary_button(ui, p, "Discard and switch", true).clicked() {
                                actions.push(Action::ConfirmSelect);
                            }
                        });
                    });
                });
            if !open {
                actions.push(Action::CancelSelect);
            }
        }

        if self.confirm_clear {
            let mut open = true;
            egui::Window::new("Remove override?")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .frame(dialog_frame(*p))
                .show(ctx, |ui| {
                    ui.set_width(360.0);
                    ui.label(
                        RichText::new("The EDID_OVERRIDE registry value will be deleted and the monitor's own EDID used again.")
                            .size(13.0)
                            .color(p.text),
                    );
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if theme::ghost_button(ui, p, "Cancel", true).clicked() {
                            self.confirm_clear = false;
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if theme::primary_button(ui, p, "Remove override", true).clicked() {
                                self.confirm_clear = false;
                                actions.push(Action::ClearOverride);
                            }
                        });
                    });
                });
            if !open {
                self.confirm_clear = false;
            }
        }

        if self.pending_restart.is_some() {
            let name = self.pending_restart.as_ref().unwrap().clone();
            let mut open = true;
            egui::Window::new("Repornește driverul grafic?")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .frame(dialog_frame(*p))
                .show(ctx, |ui| {
                    ui.set_width(360.0);
                    ui.label(
                        RichText::new(format!("Substituirea EDID pentru {name} este scrisă în registru. Repornești driverul acum pentru a o activa?"))
                            .size(13.0)
                            .color(p.text),
                    );
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if theme::ghost_button(ui, p, "Mai târziu", true).clicked() {
                            self.pending_restart = None;
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if theme::primary_button(ui, p, "Repornește acum", true).clicked() {
                                // Keep `pending_restart` populated: `ConfirmRestart`
                                // takes the display name from it to build the status
                                // message. Clearing it here made the confirmation a
                                // no-op.
                                actions.push(Action::ConfirmRestart);
                            }
                        });
                    });
                });
            if !open {
                self.pending_restart = None;
            }
        }

        if let Some(editor) = &self.editor {
            let mut open = true;
            let mut save = false;
            let mut close = false;

            egui::Window::new(if editor.index.is_some() {
                "Edit detailed resolution"
            } else {
                "Add detailed resolution"
            })
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .frame(dialog_frame(*p))
            .show(ctx, |ui| {
                ui.set_width(520.0);
                let editor = self.editor.as_mut().unwrap();

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Preset").size(12.0).color(p.muted));
                    let mut label = if editor.preset == 0 {
                        "Custom".to_string()
                    } else if editor.preset == 1 {
                        "CVT reduced blanking".to_string()
                    } else {
                        timing::presets()
                            .get(editor.preset - 2)
                            .map(|p| p.name.to_string())
                            .unwrap_or_else(|| "Custom".to_string())
                    };

                    egui::ComboBox::from_id_salt("preset")
                        .width(300.0)
                        .selected_text(label.clone())
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(editor.preset == 0, "Custom").clicked() {
                                editor.preset = 0;
                                label = "Custom".into();
                            }
                            if ui
                                .selectable_label(editor.preset == 1, "CVT reduced blanking")
                                .clicked()
                            {
                                editor.preset = 1;
                                editor.apply_preset(1);
                            }
                            for (i, preset) in timing::presets().iter().enumerate() {
                                if ui.selectable_label(editor.preset == i + 2, preset.name).clicked() {
                                    editor.preset = i + 2;
                                    editor.apply_preset(i + 2);
                                }
                            }
                        });
                });

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Width").size(12.0).color(p.muted));
                    ui.add(egui::DragValue::new(&mut editor.timing.hactive).range(64..=8192).speed(1));
                    ui.add_space(10.0);
                    ui.label(RichText::new("Height").size(12.0).color(p.muted));
                    ui.add(egui::DragValue::new(&mut editor.timing.vactive).range(64..=8192).speed(1));
                    ui.add_space(10.0);
                    ui.label(RichText::new("Refresh").size(12.0).color(p.muted));
                    let refresh = ui.add(
                        egui::DragValue::new(&mut editor.refresh_target)
                            .range(20.0..=480.0)
                            .speed(0.1)
                            .suffix(" Hz"),
                    );
                    if refresh.changed() {
                        if editor.preset == 1 {
                            editor.apply_preset(1);
                        } else {
                            // Custom preset: recompute pixel clock from the new refresh rate.
                            let clock_hz =
                                editor.timing.htotal() as f64
                                    * editor.timing.vtotal() as f64
                                    * editor.refresh_target as f64;
                            let khz = clock_hz / 1000.0;
                            editor.timing.pixel_clock_khz =
                                ((khz / 10.0).round() as u32).max(1) * 10;
                        }
                    }
                });

                ui.add_space(8.0);
                ui.checkbox(&mut editor.advanced, "Show blanking details");
                if editor.advanced {
                    ui.add_space(6.0);
                    egui::Grid::new("advanced")
                        .num_columns(4)
                        .spacing([12.0, 6.0])
                        .show(ui, |ui| {
                            ui.label(RichText::new("Pixel clock").size(12.0).color(p.muted));
                            ui.add(egui::DragValue::new(&mut editor.timing.pixel_clock_khz).range(1000..=655_350).speed(10).suffix(" kHz"));
                            ui.label(RichText::new("H blank").size(12.0).color(p.muted));
                            ui.add(egui::DragValue::new(&mut editor.timing.hblank).range(1..=4096));
                            ui.end_row();

                            ui.label(RichText::new("H front").size(12.0).color(p.muted));
                            ui.add(egui::DragValue::new(&mut editor.timing.hfront).range(0..=4096));
                            ui.label(RichText::new("H sync").size(12.0).color(p.muted));
                            ui.add(egui::DragValue::new(&mut editor.timing.hsync).range(0..=4096));
                            ui.end_row();

                            ui.label(RichText::new("V blank").size(12.0).color(p.muted));
                            ui.add(egui::DragValue::new(&mut editor.timing.vblank).range(1..=4096));
                            ui.label(RichText::new("V front").size(12.0).color(p.muted));
                            ui.add(egui::DragValue::new(&mut editor.timing.vfront).range(0..=255));
                            ui.end_row();

                            ui.label(RichText::new("V sync").size(12.0).color(p.muted));
                            ui.add(egui::DragValue::new(&mut editor.timing.vsync).range(0..=255));
                            ui.label(RichText::new("Size (mm)").size(12.0).color(p.muted));
                            ui.horizontal(|ui| {
                                ui.add(egui::DragValue::new(&mut editor.timing.hsize_mm).range(0..=1024));
                                ui.label(RichText::new("×").size(12.0).color(p.faint));
                                ui.add(egui::DragValue::new(&mut editor.timing.vsize_mm).range(0..=1024));
                            });
                            ui.end_row();
                        });

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut editor.timing.interlaced, "Interlaced");
                        ui.add_space(8.0);
                        ui.checkbox(&mut editor.timing.hsync_positive, "H sync +");
                        ui.checkbox(&mut editor.timing.vsync_positive, "V sync +");
                    });
                }

                ui.add_space(10.0);
                let t = editor.timing;
                frame_preview(ui, p, &t);

                let (max_clock, max_hfreq) = self
                    .doc
                    .as_ref()
                    .map(|d| (d.edid.max_pixel_clock_khz(), d.edid.max_hfreq_khz()))
                    .unwrap_or((0, 0));
                let validation = t.validate(max_clock, max_hfreq);
                editor.error = validation.errors.first().cloned();
                for error in &validation.errors {
                    theme::muted(ui, p, format!("✕ {error}"));
                }
                for warning in &validation.warnings {
                    ui.label(RichText::new(format!("⚠ {warning}")).size(12.0).color(p.warning));
                }

                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    if theme::ghost_button(ui, p, "Cancel", true).clicked() {
                        close = true;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if theme::primary_button(ui, p, "Save", validation.is_valid()).clicked() {
                            save = true;
                        }
                        let opt_resp = theme::ghost_button_icon(
                            ui,
                            p,
                            Icon::Logo,
                            "Optimize timings",
                            true,
                        )
                        .on_hover_text(
                            "Auto-calculate exact pixel clock while preserving your monitor's native blanking, porches and sync polarity (falls back to CVT-RB1 only if needed)",
                        );
                        if opt_resp.clicked() {
                            editor.timing = timing::optimize(
                                &editor.timing,
                                editor.refresh_target as f64,
                                max_clock,
                                max_hfreq,
                            );
                            editor.preset = 0;
                        }
                    });
                });
            });

            if save {
                actions.push(Action::SaveEditor);
            } else if close || !open {
                actions.push(Action::CloseEditor);
            }
        }

        if let Some(toast) = &self.toast {
            let elapsed = toast.at.elapsed().as_secs_f32();
            let enter = (elapsed / 0.16).clamp(0.0, 1.0);
            let exit = ((4.0 - elapsed) / 0.25).clamp(0.0, 1.0);
            let ease_enter = 1.0 - (1.0 - enter).powi(3);
            let alpha = (ease_enter * exit).clamp(0.0, 1.0);
            let rise = (1.0 - ease_enter) * 8.0;

            let color = if toast.error { p.danger } else { p.success };
            let icon = if toast.error { Icon::Alert } else { Icon::Check };

            egui::Area::new(Id::new("toast"))
                .anchor(Align2::CENTER_BOTTOM, Vec2::new(0.0, -44.0 - (8.0 - rise)))
                .show(ctx, |ui| {
                    ui.set_max_width((ctx.screen_rect().width() - 48.0).max(220.0));
                    theme::toast_frame(*p).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (icon_rect, _) =
                                ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                            icons::draw(
                                icon,
                                ui.painter(),
                                icon_rect,
                                color.gamma_multiply(alpha),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(&toast.text)
                                    .size(12.0)
                                    .color(p.text.gamma_multiply(alpha)),
                            );
                        });
                    });
                });
        }
    }
}

struct FormatPreview {
    max_clock: u32,
    max_hfreq: u32,
}

impl FormatPreview {
    fn show(&self, ui: &mut Ui, p: Palette) {
        let text = if self.max_clock > 0 {
            format!(
                "display limit {:.2} MHz  ·  {} kHz",
                self.max_clock as f64 / 1000.0,
                self.max_hfreq
            )
        } else {
            "no range limits declared".to_string()
        };
        ui.label(RichText::new(text).size(11.5).color(p.faint));
    }
}

fn frame_preview(ui: &mut Ui, p: &Palette, t: &Timing) {
    let text = format!(
        "{}x{} @ {:.3} Hz  ·  H {}  ·  V {}  ·  {:.2} MHz  ·  {:.1} kHz  ·  {:.2} Gbps",
        t.hactive,
        t.vactive,
        t.frame_hz(),
        t.htotal(),
        t.vtotal(),
        t.pixel_clock_khz as f64 / 1000.0,
        t.hfreq_khz(),
        t.bandwidth_gbps(),
    );
    ui.label(RichText::new(text).size(11.5).color(p.muted));
}

fn dialog_frame(p: Palette) -> Frame {
    Frame::NONE
        .fill(p.card)
        .stroke(Stroke::new(HAIRLINE, p.border))
        .corner_radius(CornerRadius::same(RADIUS_CARD))
        .inner_margin(Margin::same(20))
        .shadow(egui::epaint::Shadow {
            offset: [0, 12],
            blur: 32,
            spread: 0,
            color: p.shadow,
        })
}

fn display_card(
    ui: &mut Ui,
    p: &Palette,
    index: usize,
    display: &DisplayInfo,
    selected: bool,
) -> egui::Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 60.0), Sense::click());

    let hover_t = ui.ctx().animate_bool_with_time(
        Id::new(("display_hover", index)),
        !selected && response.hovered(),
        0.14,
    );
    let select_t = ui
        .ctx()
        .animate_bool_with_time(Id::new(("display_sel", index)), selected, 0.14);

    let radius = CornerRadius::same(13);
    if select_t > 0.01 {
        let fill = theme::color_lerp(Color32::TRANSPARENT, p.card, select_t);
        ui.painter().rect_filled(rect, radius, fill);
        ui.painter().rect_stroke(
            rect,
            radius,
            Stroke::new(HAIRLINE, p.border.gamma_multiply(select_t)),
            StrokeKind::Inside,
        );
        ui.painter().line_segment(
            [
                Pos2::new(rect.left() + 10.0, rect.top() + 1.0),
                Pos2::new(rect.right() - 10.0, rect.top() + 1.0),
            ],
            Stroke::new(HAIRLINE, p.hairline.gamma_multiply(select_t)),
        );
        // Rounded Apple-blue left pill indicator inside the card edge.
        let pill = Rect::from_min_size(
            Pos2::new(rect.left() + 5.0, rect.center().y - 11.0),
            Vec2::new(3.0, 22.0),
        );
        ui.painter()
            .rect_filled(pill, CornerRadius::same(2), p.accent.gamma_multiply(select_t));
    } else if hover_t > 0.01 {
        ui.painter().rect_filled(
            rect,
            radius,
            theme::color_lerp(Color32::TRANSPARENT, p.hover, hover_t),
        );
    }

    // Drawn Monitor icon on the left.
    let icon_rect = Rect::from_center_size(
        Pos2::new(rect.left() + 24.0, rect.center().y),
        Vec2::splat(16.0),
    );
    let icon_color = theme::color_lerp(p.muted, p.accent, select_t);
    icons::draw(Icon::Monitor, ui.painter(), icon_rect, icon_color);

    // Status dot.
    let dot_color = if display.has_override() {
        p.accent
    } else if display.active {
        p.success
    } else {
        p.faint
    };
    ui.painter()
        .circle_filled(Pos2::new(rect.left() + 42.0, rect.center().y), 3.2, dot_color);

    // Ellipsized title + subtitle using pixel-width LayoutJob truncation.
    let text_x = rect.left() + 54.0;
    let text_max_w = (rect.right() - text_x - 10.0).max(40.0);
    let name = display.monitor_name();
    let title_color = if display.active { p.text } else { p.muted };
    let title_galley = theme::ellipsize_galley(
        ui.painter(),
        &name,
        theme::semibold(13.0),
        title_color,
        text_max_w,
    );
    ui.painter().galley(
        Pos2::new(text_x, rect.top() + 12.0),
        title_galley,
        title_color,
    );

    let subtitle = if display.active {
        display
            .current_mode
            .map(|(w, h, hz)| format!("{w} × {h} @ {hz} Hz"))
            .unwrap_or_else(|| display.subtitle())
    } else {
        format!("{}  ·  inactive", display.hardware_id)
    };
    let sub_galley = theme::ellipsize_galley(
        ui.painter(),
        &subtitle,
        theme::font(11.5),
        p.faint,
        text_max_w,
    );
    ui.painter()
        .galley(Pos2::new(text_x, rect.top() + 33.0), sub_galley, p.faint);

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_app(mode: ThemeMode) -> App {
        let fixture = include_bytes!("../../tests/fixtures/SAM0D46-256.hex");
        let hex_text = std::str::from_utf8(fixture).unwrap();
        let bytes: Vec<u8> = hex_text
            .split_whitespace()
            .filter_map(|s| u8::from_str_radix(s, 16).ok())
            .collect();
        let edid = Edid::parse(&bytes).unwrap();
        let display = DisplayInfo {
            hardware_id: "SAM0D46".to_string(),
            instance: "1&8713bca&0&UID256".to_string(),
            name: "Samsung Odyssey".to_string(),
            adapter: "AMD Radeon".to_string(),
            adapter_name: "AMD".to_string(),
            active: true,
            edid: bytes.clone(),
            override_edid: Some(bytes.clone()),
            current_mode: Some((2560, 1440, 144)),
            key_path: "SAM0D46\\1&8713bca\\Device Parameters".to_string(),
        };
        let doc = Doc {
            display: display.clone(),
            original: bytes,
            edid,
            source: "Registry".to_string(),
        };
        App {
            displays: vec![display.clone(), display],
            selected: Some(0),
            doc: Some(doc),
            tab: 0,
            mode,
            selected_timing: Some(0),
            editor: None,
            toast: Some(Toast {
                text: "Override scris pentru Samsung Odyssey.".to_string(),
                error: false,
                at: Instant::now(),
            }),
            pending_select: None,
            confirm_clear: false,
            pending_restart: None,
            exit_at: None,
            first_frame: false,
            admin: true,
        }
    }

    fn run_frame(app: &mut App, width: f32, height: f32) -> egui::FullOutput {
        let ctx = egui::Context::default();
        theme::load_fonts(&ctx);
        theme::apply_style(&ctx, app.mode);
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, height))),
            ..Default::default()
        };
        ctx.run(input, |ctx| {
            let p = app.palette();
            let mut actions = Vec::new();
            app.top_bar(ctx, &p, &mut actions);
            app.status_bar(ctx, &p, &mut actions);
            app.sidebar(ctx, &p, &mut actions);
            app.central(ctx, &p, &mut actions);
            app.dialogs(ctx, &p, &mut actions);
        })
    }

    #[test]
    fn renders_all_tabs_in_dark_and_light_at_min_and_default_sizes() {
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            for (w, h) in [(1120.0, 720.0), (900.0, 560.0)] {
                let mut app = sample_app(mode);
                for tab in 0..4 {
                    app.tab = tab;
                    let out = run_frame(&mut app, w, h);
                    assert!(
                        !out.shapes.is_empty(),
                        "expected rendered shapes for tab {tab} at {w}x{h}"
                    );
                }
            }
        }
    }

    #[test]
    fn load_document_prefers_override_edid() {
        let mut app = sample_app(ThemeMode::Dark);
        let mut modified = app.displays[0].edid.clone();
        // Change first detailed timing pixel clock in the override copy.
        let mut parsed = Edid::parse(&modified).unwrap();
        if let Descriptor::Detailed(ref mut t) = parsed.base.descriptors[0] {
            t.hactive = 1280;
            t.vactive = 720;
        }
        modified = parsed.serialize();
        app.displays[0].override_edid = Some(modified);
        app.load_document();

        let doc = app.doc.as_ref().unwrap();
        let (_, first) = doc.detailed()[0];
        assert_eq!((first.hactive, first.vactive), (1280, 720));
        assert!(!doc.dirty());
    }

    #[test]
    fn editor_dialog_renders_with_optimize_button() {
        let mut app = sample_app(ThemeMode::Dark);
        app.open_editor(Some(0));
        let out = run_frame(&mut app, 1120.0, 720.0);
        assert!(!out.shapes.is_empty());
    }
}
