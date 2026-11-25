//! Probe: resolve every setupapi symbol by name via GetProcAddress and report
//! exactly which names exist. Read-only.

use std::ffi::c_void;

#[link(name = "kernel32", kind = "raw-dylib")]
extern "system" {
    fn LoadLibraryA(name: *const u8) -> isize;
    fn GetProcAddress(module: isize, name: *const u8) -> *mut c_void;
    fn FreeLibrary(module: isize) -> i32;
}

fn exists(module: isize, name: &str) -> bool {
    let mut bytes = name.as_bytes().to_vec();
    bytes.push(0);
    let p = unsafe { GetProcAddress(module, bytes.as_ptr()) };
    !p.is_null()
}

fn main() {
    let module = unsafe { LoadLibraryA(b"setupapi.dll\0".as_ptr()) };
    println!("module = {module}");
    if module == 0 {
        println!("LoadLibraryA failed");
        return;
    }

    let names = [
        "SetupDiGetClassDevs",
        "SetupDiGetClassDevsA",
        "SetupDiGetClassDevsW",
        "SetupDiEnumDeviceInfo",
        "SetupDiEnumDeviceInfoA",
        "SetupDiEnumDeviceInfoW",
        "SetupDiSetClassInstallParams",
        "SetupDiSetClassInstallParamsA",
        "SetupDiSetClassInstallParamsW",
        "SetupDiCallClassInstaller",
        "SetupDiCallClassInstallerA",
        "SetupDiCallClassInstallerW",
        "SetupDiDestroyDeviceInfoList",
    ];

    for name in names {
        println!("{:<32} {}", name, if exists(module, name) { "FOUND" } else { "-- not found --" });
    }

    unsafe { FreeLibrary(module) };
}