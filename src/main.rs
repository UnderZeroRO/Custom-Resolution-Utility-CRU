//! CRU Modern — a modern front-end for Custom Resolution Utility.
//!
//! Reads and writes EDID overrides through the Windows registry, with a
//! cleaner UI than the original VCL application.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

mod app_icon;
mod ui;

use ui::APP_TITLE;

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // Helper mode: the GUI spawns itself with these flags so the display driver
    // restart happens outside the process that owns the OpenGL context.
    if args.iter().any(|arg| arg == cru_modern::system::RESTART_FLAG) {
        run_restart_helper(&args);
    }

    let icon = egui::IconData {
        rgba: app_icon::render_rgba(64),
        width: 64,
        height: 64,
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(APP_TITLE)
            .with_inner_size([1120.0, 720.0])
            .with_min_inner_size([900.0, 560.0])
            .with_icon(Arc::new(icon))
            .with_app_id("cru-modern"),
        ..Default::default()
    };

    eframe::run_native(
        APP_TITLE,
        options,
        Box::new(|cc| Ok(Box::new(ui::App::new(cc)))),
    )
}

/// Restart the display driver, then bring the GUI back.
fn run_restart_helper(args: &[String]) -> ! {
    let relaunch = args.iter().any(|arg| arg == cru_modern::system::RELAUNCH_FLAG);
    let wait_pid = args
        .iter()
        .position(|arg| arg == cru_modern::system::WAIT_PID_FLAG)
        .and_then(|index| args.get(index + 1))
        .and_then(|pid| pid.parse::<u32>().ok())
        .unwrap_or(0);

    // The GUI must be gone before the adapter is disabled, otherwise its
    // OpenGL context blocks the PnP stop.
    cru_modern::system::wait_for_process_exit(wait_pid, 20_000);

    let outcome = cru_modern::system::restart_driver();
    cru_modern::system::write_last_restart(&outcome);

    if relaunch {
        let _ = cru_modern::system::relaunch_gui();
    }

    match &outcome {
        Ok(message) => println!("{message}"),
        Err(err) => println!("{err}"),
    }

    std::process::exit(if outcome.is_ok() { 0 } else { 1 });
}
