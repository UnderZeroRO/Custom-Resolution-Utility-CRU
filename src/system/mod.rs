//! Windows display enumeration and EDID storage (registry).

pub mod displays;

pub use displays::{
    clear_override, enumerate_displays, is_elevated, read_edid_file, relaunch_elevated,
    relaunch_gui, restart_driver, spawn_restart_helper, take_last_restart, wait_for_process_exit,
    write_edid_file, write_last_restart, write_override, DisplayInfo, RELAUNCH_FLAG, RESTART_FLAG,
    WAIT_PID_FLAG,
};
