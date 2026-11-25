//! Display enumeration via EnumDisplayDevices plus EDID access in the registry.

use std::borrow::Cow;
use std::mem::size_of;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use winreg::enums::*;
use winreg::{RegKey, RegValue};

use crate::edid::{Descriptor, Edid};

const ENUM_DISPLAY: &str = "SYSTEM\\CurrentControlSet\\Enum\\DISPLAY";
const ENUM_CURRENT_SETTINGS: u32 = 0xFFFF_FFFF;
const DISPLAY_DEVICE_ATTACHED_TO_DESKTOP: u32 = 0x0000_0001;
const EDD_GET_DEVICE_INTERFACE_NAME: u32 = 0x0000_0001;

#[repr(C)]
struct DisplayDeviceW {
    cb: u32,
    device_name: [u16; 32],
    device_string: [u16; 128],
    state_flags: u32,
    device_id: [u16; 128],
    device_key: [u16; 128],
}

#[repr(C)]
struct DevModeW {
    device_name: [u16; 32],
    spec_version: u16,
    driver_version: u16,
    size: u16,
    driver_extra: u16,
    fields: u32,
    position: [i32; 2],
    display_orientation: u32,
    display_fixed_output: u32,
    color: i16,
    duplex: i16,
    y_resolution: i16,
    tt_option: i16,
    collate: i16,
    form_name: [u16; 32],
    log_pixels: u16,
    bits_per_pel: u32,
    pels_width: u32,
    pels_height: u32,
    display_flags: u32,
    display_frequency: u32,
    icm_method: u32,
    icm_intent: u32,
    media_type: u32,
    dither_type: u32,
    reserved1: u32,
    reserved2: u32,
    panning_width: u32,
    panning_height: u32,
}

#[link(name = "user32")]
extern "system" {
    fn EnumDisplayDevicesW(
        device: *const u16,
        dev_num: u32,
        display_device: *mut DisplayDeviceW,
        flags: u32,
    ) -> i32;
    fn EnumDisplaySettingsW(device_name: *const u16, mode_num: u32, dev_mode: *mut DevModeW) -> i32;
}

#[link(name = "shell32")]
extern "system" {
    fn ShellExecuteW(
        hwnd: isize,
        operation: *const u16,
        file: *const u16,
        parameters: *const u16,
        directory: *const u16,
        show_cmd: i32,
    ) -> isize;
}

#[derive(Clone, Debug, Default)]
pub struct DisplayInfo {
    pub hardware_id: String,
    pub instance: String,
    pub name: String,
    pub adapter: String,
    pub adapter_name: String,
    pub active: bool,
    pub edid: Vec<u8>,
    pub override_edid: Option<Vec<u8>>,
    pub current_mode: Option<(u32, u32, u32)>,
    pub key_path: String,
}

impl DisplayInfo {
    pub fn monitor_name(&self) -> String {
        if let Ok(edid) = Edid::parse(&self.edid) {
            for d in &edid.base.descriptors {
                if let Descriptor::MonitorName(name) = d {
                    if !name.is_empty() {
                        return name.clone();
                    }
                }
            }
        }
        if !self.name.is_empty() {
            self.name.clone()
        } else {
            format!("{} ({})", self.hardware_id, self.instance)
        }
    }

    pub fn subtitle(&self) -> String {
        if let Ok(edid) = Edid::parse(&self.edid) {
            format!(
                "{}  ·  {}  ·  {}",
                edid.base.manufacturer(),
                self.hardware_id,
                edid.base.interface()
            )
        } else {
            self.hardware_id.clone()
        }
    }

    pub fn has_override(&self) -> bool {
        self.override_edid.as_ref().is_some_and(|o| !o.is_empty())
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn from_wide(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end]).trim().to_string()
}

/// Full registry path, shown to the user.
fn device_key_path(hardware_id: &str, instance: &str) -> String {
    format!("{ENUM_DISPLAY}\\{hardware_id}\\{instance}\\Device Parameters")
}

/// Path relative to the already-open `Enum\DISPLAY` key.
fn relative_key_path(hardware_id: &str, instance: &str) -> String {
    format!("{hardware_id}\\{instance}\\Device Parameters")
}

fn read_binary(key: &RegKey, name: &str) -> Option<Vec<u8>> {
    key.get_raw_value(name).ok().map(|v| v.bytes.to_vec())
}

/// Read Windows' `Device Parameters\EDID_OVERRIDE` subkey (`"0"`, `"1"`, ... 128-byte blocks).
fn read_override(device_params: &RegKey) -> Option<Vec<u8>> {
    let sub = device_params
        .open_subkey_with_flags("EDID_OVERRIDE", KEY_READ | KEY_WOW64_64KEY)
        .ok()?;
    let block0 = read_binary(&sub, "0")?;
    if block0.len() < 128 {
        return None;
    }
    let mut bytes = block0[..128].to_vec();
    for index in 1..16 {
        let name = index.to_string();
        let Some(block) = read_binary(&sub, &name) else {
            break;
        };
        if block.len() < 128 {
            break;
        }
        bytes.extend_from_slice(&block[..128]);
    }
    Some(bytes)
}

pub fn read_edid_file(path: &Path) -> std::io::Result<Vec<u8>> {
    std::fs::read(path)
}

pub fn write_edid_file(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    std::fs::write(path, bytes)
}

/// Enumerate every monitor with an EDID, active displays first.
pub fn enumerate_displays() -> Vec<DisplayInfo> {
    let mut displays: Vec<DisplayInfo> = Vec::new();

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let root = hklm.open_subkey_with_flags(ENUM_DISPLAY, KEY_READ | KEY_WOW64_64KEY);

    // Active displays, matched to their registry key via the device interface path.
    let mut adapter_index = 0u32;
    loop {
        let mut adapter: DisplayDeviceW = unsafe { std::mem::zeroed() };
        adapter.cb = size_of::<DisplayDeviceW>() as u32;
        let ok = unsafe { EnumDisplayDevicesW(std::ptr::null(), adapter_index, &mut adapter, 0) };
        if ok == 0 {
            break;
        }
        adapter_index += 1;

        if adapter.state_flags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP == 0 {
            continue;
        }

        let adapter_name = from_wide(&adapter.device_name);
        let adapter_string = from_wide(&adapter.device_string);
        let mode = current_mode(&adapter_name);

        let mut monitor_index = 0u32;
        loop {
            let mut monitor: DisplayDeviceW = unsafe { std::mem::zeroed() };
            monitor.cb = size_of::<DisplayDeviceW>() as u32;
            let ok = unsafe {
                EnumDisplayDevicesW(
                    adapter.device_name.as_ptr(),
                    monitor_index,
                    &mut monitor,
                    EDD_GET_DEVICE_INTERFACE_NAME,
                )
            };
            if ok == 0 {
                break;
            }
            monitor_index += 1;

            let device_id = from_wide(&monitor.device_id);
            let monitor_string = from_wide(&monitor.device_string);
            let Some((hardware_id, instance)) = parse_interface_path(&device_id) else {
                continue;
            };

            let path = device_key_path(&hardware_id, &instance);
            let relative = relative_key_path(&hardware_id, &instance);
            let (edid, override_edid) = match &root {
                Ok(root) => match root.open_subkey_with_flags(&relative, KEY_READ | KEY_WOW64_64KEY) {
                    Ok(key) => (
                        read_binary(&key, "EDID").unwrap_or_default(),
                        read_override(&key),
                    ),
                    Err(_) => (Vec::new(), None),
                },
                Err(_) => (Vec::new(), None),
            };

            if edid.is_empty() {
                continue;
            }

            displays.push(DisplayInfo {
                hardware_id,
                instance,
                name: monitor_string,
                adapter: adapter_name.clone(),
                adapter_name: adapter_string.clone(),
                active: true,
                edid,
                override_edid,
                current_mode: mode,
                key_path: path,
            });
        }
    }

    // Every remaining EDID entry in the registry (inactive / disconnected monitors).
    if let Ok(root) = &root {
        for hw in root.enum_keys().flatten() {
            let hardware_id = hw.clone();
            let Ok(hw_key) = root.open_subkey_with_flags(&hw, KEY_READ | KEY_WOW64_64KEY) else {
                continue;
            };
            for inst in hw_key.enum_keys().flatten() {
                let instance = inst.clone();
                let path = device_key_path(&hardware_id, &instance);
                let relative = relative_key_path(&hardware_id, &instance);
                let Ok(key) = root.open_subkey_with_flags(&relative, KEY_READ | KEY_WOW64_64KEY) else {
                    continue;
                };
                let Some(edid) = read_binary(&key, "EDID") else {
                    continue;
                };
                if edid.is_empty() {
                    continue;
                }
                if displays
                    .iter()
                    .any(|d| d.hardware_id == hardware_id && d.instance == instance)
                {
                    continue;
                }
                displays.push(DisplayInfo {
                    hardware_id: hardware_id.clone(),
                    instance,
                    name: String::new(),
                    adapter: String::new(),
                    adapter_name: String::new(),
                    active: false,
                    edid,
                    override_edid: read_override(&key),
                    current_mode: None,
                    key_path: path,
                });
            }
        }
    }

    displays.sort_by(|a, b| {
        b.active
            .cmp(&a.active)
            .then_with(|| a.monitor_name().cmp(&b.monitor_name()))
    });
    displays
}

/// `\\?\DISPLAY#SAM0D46#5&10052235&3&UID264#{guid}` -> (SAM0D46, 5&10052235&3&UID264)
fn parse_interface_path(device_id: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = device_id.split('#').collect();
    if parts.len() >= 3 {
        let hardware = parts[1].trim();
        let instance = parts[2].trim();
        if !hardware.is_empty() && !instance.is_empty() {
            return Some((hardware.to_string(), instance.to_string()));
        }
    }

    // Fallback for the classic MONITOR\XXX\{guid}\0004 form.
    let parts: Vec<&str> = device_id.split('\\').collect();
    if parts.len() >= 4 {
        return Some((parts[1].to_string(), parts[3].to_string()));
    }
    None
}

fn current_mode(adapter: &str) -> Option<(u32, u32, u32)> {
    let name = wide(adapter);
    let mut mode: DevModeW = unsafe { std::mem::zeroed() };
    mode.size = size_of::<DevModeW>() as u16;
    let ok = unsafe { EnumDisplaySettingsW(name.as_ptr(), ENUM_CURRENT_SETTINGS, &mut mode) };
    if ok == 0 {
        None
    } else {
        Some((mode.pels_width, mode.pels_height, mode.display_frequency))
    }
}

/// True when the process can write to the hardware enum key (i.e. elevated).
pub fn is_elevated() -> bool {
    RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(
            "SYSTEM\\CurrentControlSet\\Enum",
            KEY_READ | KEY_SET_VALUE | KEY_WOW64_64KEY,
        )
        .is_ok()
}

/// Write the EDID override for a display into `Device Parameters\EDID_OVERRIDE`
/// as 128-byte `REG_BINARY` block values `"0"`, `"1"`, ... (Windows / CRU format).
pub fn write_override(display: &DisplayInfo, edid: &[u8]) -> Result<(), String> {
    if edid.len() < 128 || edid.len() % 128 != 0 {
        return Err(format!("Invalid EDID length ({} bytes)", edid.len()));
    }

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = hklm
        .open_subkey_with_flags(
            &display.key_path,
            KEY_READ | KEY_SET_VALUE | KEY_CREATE_SUB_KEY | KEY_WOW64_64KEY,
        )
        .map_err(|e| format!("Cannot open {}: {e}", display.key_path))?;

    // Remove any legacy single-value EDID_OVERRIDE written by older builds.
    let _ = key.delete_value("EDID_OVERRIDE");

    let (sub, _) = key
        .create_subkey_with_flags("EDID_OVERRIDE", KEY_READ | KEY_SET_VALUE | KEY_WOW64_64KEY)
        .map_err(|e| format!("Cannot create EDID_OVERRIDE subkey: {e}"))?;

    let blocks = edid.chunks_exact(128).count();
    for (index, chunk) in edid.chunks_exact(128).enumerate() {
        let value = RegValue {
            bytes: Cow::Owned(chunk.to_vec()),
            vtype: REG_BINARY,
        };
        sub.set_raw_value(index.to_string(), &value)
            .map_err(|e| format!("Cannot write EDID_OVERRIDE\\{index}: {e}"))?;
    }

    // Clear stale higher-index blocks if the new EDID has fewer blocks.
    for index in blocks..16 {
        match sub.delete_value(index.to_string()) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => break,
            Err(e) => return Err(format!("Cannot clean EDID_OVERRIDE\\{index}: {e}")),
        }
    }

    Ok(())
}

/// Remove the override, restoring the EDID reported by the monitor.
pub fn clear_override(display: &DisplayInfo) -> Result<(), String> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = hklm
        .open_subkey_with_flags(
            &display.key_path,
            KEY_READ | KEY_SET_VALUE | KEY_WOW64_64KEY,
        )
        .map_err(|e| format!("Cannot open {}: {e}", display.key_path))?;

    let _ = key.delete_value("EDID_OVERRIDE");
    match key.delete_subkey_all("EDID_OVERRIDE") {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("Cannot delete EDID_OVERRIDE: {e}")),
    }
}

/// Relaunch the current executable with the "runas" verb.
pub fn relaunch_elevated() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_w = wide(&exe.to_string_lossy());
    let op = wide("runas");
    let result = unsafe {
        ShellExecuteW(
            0,
            op.as_ptr(),
            exe_w.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1,
        )
    };
    if result as isize <= 32 {
        Err(format!("ShellExecute failed ({result})"))
    } else {
        Ok(())
    }
}

/// Restart the graphics driver natively via SetupAPI (disable/enable display
/// adapters), mirroring CRU restart64 `SetDriverState`. Requires elevation.
pub fn restart_driver() -> Result<String, String> {
    if !is_elevated() {
        return Err(
            "Driver restart needs administrator rights. Rulați aplicația ca administrator."
                .to_string(),
        );
    }

    restart_driver_native()
}

/// Command-line flag that turns the executable into the restart helper.
pub const RESTART_FLAG: &str = "--restart-driver";

/// Helper flag: start the GUI again once the driver is back.
pub const RELAUNCH_FLAG: &str = "--relaunch-after-restart";

/// Helper flag: PID of the GUI that must exit before disabling the adapter.
pub const WAIT_PID_FLAG: &str = "--wait-pid";

/// File where the helper stores the restart outcome for the relaunched GUI.
const LAST_RESTART_FILE: &str = "cru-modern-last-restart.txt";

fn last_restart_path() -> PathBuf {
    std::env::temp_dir().join(LAST_RESTART_FILE)
}

/// Spawn the helper that restarts the display driver and brings the GUI back.
///
/// The GUI has to exit before the adapters are disabled. This app renders with
/// OpenGL; `DICS_DISABLE` tears the GL context down and the window never
/// recovers (it stays "Not Responding" forever). The original CRU only survives
/// this because its UI is pure GDI. So the helper waits for our PID to vanish,
/// restarts the driver, and relaunches the app.
pub fn spawn_restart_helper() -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    if !is_elevated() {
        return Err(
            "Driver restart needs administrator rights. Rulați aplicația ca administrator."
                .to_string(),
        );
    }

    let exe = std::env::current_exe()
        .map_err(|e| format!("Cannot locate the application executable: {e}"))?;

    std::process::Command::new(&exe)
        .arg(RESTART_FLAG)
        .arg(RELAUNCH_FLAG)
        .arg(WAIT_PID_FLAG)
        .arg(std::process::id().to_string())
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Cannot start the restart helper: {e}"))
}

/// Helper side: persist the outcome so the relaunched GUI can report it.
pub fn write_last_restart(result: &Result<String, String>) {
    let text = match result {
        Ok(message) => format!("ok:{message}"),
        Err(err) => format!("err:{err}"),
    };
    let _ = std::fs::write(last_restart_path(), text);
}

/// GUI side: consume the outcome of a restart performed by the helper.
/// Results older than `max_age` are discarded (leftovers from older runs).
pub fn take_last_restart(max_age: Duration) -> Option<Result<String, String>> {
    let path = last_restart_path();
    let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok()?;
    let fresh = modified.elapsed().is_ok_and(|age| age <= max_age);
    let text = std::fs::read_to_string(&path).ok()?;
    let _ = std::fs::remove_file(&path);
    if !fresh {
        return None;
    }
    let (kind, message) = text.split_once(':')?;
    Some(if kind == "ok" {
        Ok(message.to_string())
    } else {
        Err(message.to_string())
    })
}

/// Helper side: start the GUI again, without a console window.
pub fn relaunch_gui() -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let exe = std::env::current_exe()
        .map_err(|e| format!("Cannot locate the application executable: {e}"))?;

    std::process::Command::new(exe)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Cannot relaunch the application: {e}"))
}

#[link(name = "kernel32", kind = "raw-dylib")]
extern "system" {
    fn OpenProcess(desired_access: u32, inherit_handle: i32, process_id: u32) -> isize;
    fn WaitForSingleObject(handle: isize, milliseconds: u32) -> u32;
    fn CloseHandle(handle: isize) -> i32;
}

const SYNCHRONIZE: u32 = 0x0010_0000;

/// Block until `pid` exits or the timeout elapses.
pub fn wait_for_process_exit(pid: u32, timeout_ms: u32) {
    if pid == 0 {
        return;
    }
    unsafe {
        let handle = OpenProcess(SYNCHRONIZE, 0, pid);
        if handle != 0 {
            WaitForSingleObject(handle, timeout_ms);
            CloseHandle(handle);
        }
    }
}

// Native graphics driver restart via SetupAPI, mirroring CRU restart64
// `SetDriverState()` in restart.c: DICS_DISABLE then DICS_ENABLE on every
// device of the display class. Requires elevation.
//
// Symbol resolution note (verified with examples/setupapi_names.rs): setupapi.dll
// only exports the `W` variants of SetupDiGetClassDevs and SetupDiSetClassInstallParams;
// the bare names are Win32 macros and do not exist as entry points.
#[link(name = "setupapi", kind = "raw-dylib")]
extern "system" {
    // NOTE: setupapi.dll exports only the W variant; the bare `SetupDiGetClassDevs`
    // is a Win32 macro and does not exist as an entry point.
    fn SetupDiGetClassDevsW(
        class_guid: *const Guid,
        enumerator: *const u16,
        hwnd_parent: isize,
        flags: u32,
    ) -> isize;
    fn SetupDiEnumDeviceInfo(
        device_info_set: isize,
        member_index: u32,
        device_info_data: *mut SpDevInfoData,
    ) -> i32;
    fn SetupDiSetClassInstallParamsW(
        device_info_set: isize,
        device_info_data: *const SpDevInfoData,
        class_install_params: *const SpPropChangeParams,
        class_install_params_size: u32,
    ) -> i32;
    fn SetupDiCallClassInstaller(
        install_function: u32,
        device_info_set: isize,
        device_info_data: *const SpDevInfoData,
    ) -> i32;
    fn SetupDiDestroyDeviceInfoList(device_info_set: isize);
}

const DIGCF_PRESENT: u32 = 0x0000_0002;
const DICS_FLAG_GLOBAL: u32 = 0x0000_0001;
const DICS_ENABLE: u32 = 0x0000_0001;
const DICS_DISABLE: u32 = 0x0000_0002;
// DIF_PROPERTYCHANGE
const DI_PROPERTYCHANGE: u32 = 0x0000_0012;

/// {4d36e968-e325-11ce-bfc1-08002be10318}
const GUID_DEVCLASS_DISPLAY: Guid = Guid {
    data1: 0x4d36e968,
    data2: 0xe325,
    data3: 0x11ce,
    data4: [0xbf, 0xc1, 0x08, 0x00, 0x2b, 0xe1, 0x03, 0x18],
};

#[repr(C)]
struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

#[repr(C)]
struct SpDevInfoData {
    cb_size: u32,
    class_guid: Guid,
    dev_inst: u32,
    reserved: *mut std::ffi::c_void,
}

#[repr(C)]
struct SpClassInstallHeader {
    cb_size: u32,
    install_function: u32,
}

#[repr(C)]
struct SpPropChangeParams {
    class_install_header: SpClassInstallHeader,
    state_change: u32,
    scope: u32,
    hw_profile: u32,
}

/// Disable or enable every present display adapter, returning how many changed.
fn set_display_driver_state(state: u32) -> u32 {
    unsafe {
        let devices = SetupDiGetClassDevsW(
            &GUID_DEVCLASS_DISPLAY,
            std::ptr::null(),
            0,
            DIGCF_PRESENT,
        );
        if devices == -1 {
            return 0;
        }
        let mut result = 0u32;
        let mut index = 0u32;
        loop {
            let mut device: SpDevInfoData = std::mem::zeroed();
            device.cb_size = size_of::<SpDevInfoData>() as u32;
            if SetupDiEnumDeviceInfo(devices, index, &mut device) == 0 {
                break;
            }
            index += 1;
            let mut params: SpPropChangeParams = std::mem::zeroed();
            params.class_install_header.cb_size = size_of::<SpClassInstallHeader>() as u32;
            params.class_install_header.install_function = DI_PROPERTYCHANGE;
            params.state_change = state;
            params.scope = DICS_FLAG_GLOBAL;
            params.hw_profile = 0;
            if SetupDiSetClassInstallParamsW(
                devices,
                &device,
                &params,
                size_of::<SpPropChangeParams>() as u32,
            ) != 0
                && SetupDiCallClassInstaller(DI_PROPERTYCHANGE, devices, &device) != 0
            {
                result += 1;
            }
        }
        SetupDiDestroyDeviceInfoList(devices);
        result
    }
}

fn restart_driver_native() -> Result<String, String> {
    let disabled = set_display_driver_state(DICS_DISABLE);
    if disabled == 0 {
        return Err(
            "Nu s-a putut dezactiva niciun adaptor de afișare. Verificați drepturile de administrator."
                .to_string(),
        );
    }
    thread::sleep(Duration::from_millis(100));
    let enabled = set_display_driver_state(DICS_ENABLE);
    if enabled == 0 {
        return Err(
            "Nu s-a putut reînnoi niciun adaptor de afișare. Verificați drepturile de administrator."
                .to_string(),
        );
    }
    // CRU așteaptă 3500 ms după reînnoire ca driverul să se stabilească.
    thread::sleep(Duration::from_millis(3500));
    Ok(format!("Driver repornit nativ ({enabled} adaptoare)"))
}
