//! Generates the Windows `.ico` and embeds it into `cru-modern.exe` via `windres`.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

#[allow(dead_code)]
#[path = "src/app_icon.rs"]
mod app_icon;

fn find_windres() -> Option<PathBuf> {
    if let Ok(path) = env::var("PATH") {
        for dir in env::split_paths(&path) {
            for name in ["windres.exe", "x86_64-w64-mingw32-windres.exe"] {
                let candidate = dir.join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    // Fallback: WinLibs WinGet package location if not on current PATH.
    if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
        let packages = Path::new(&local_app_data).join("Microsoft\\WinGet\\Packages");
        if let Ok(entries) = std::fs::read_dir(&packages) {
            for entry in entries.flatten() {
                let candidate = entry.path().join("mingw64\\bin\\windres.exe");
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/app_icon.rs");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR missing"));
    let ico_path = out_dir.join("cru-modern.ico");
    app_icon::write_ico(&ico_path).expect("failed to generate app icon .ico");

    let rc_path = out_dir.join("cru-modern.rc");
    let ico_escaped = ico_path.to_string_lossy().replace('\\', "/");
    let version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.1.0".to_string());
    let mut parts = version.split('.').map(|s| s.parse::<u16>().unwrap_or(0));
    let (maj, min, pat) = (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(1),
        parts.next().unwrap_or(0),
    );
    let rc_content = format!(
        "1 ICON \"{ico_escaped}\"\n\
         \n\
         1 VERSIONINFO\n\
         FILEVERSION {maj},{min},{pat},0\n\
         PRODUCTVERSION {maj},{min},{pat},0\n\
         FILEOS 0x40004\n\
         FILETYPE 0x1\n\
         BEGIN\n\
           BLOCK \"StringFileInfo\"\n\
           BEGIN\n\
             BLOCK \"040904B0\"\n\
             BEGIN\n\
               VALUE \"CompanyName\", \"UnderZeroRO\"\n\
               VALUE \"FileDescription\", \"CRU Modern - Custom Resolution Utility (EDID Editor)\"\n\
               VALUE \"FileVersion\", \"{version}.0\"\n\
               VALUE \"InternalName\", \"cru-modern\"\n\
               VALUE \"OriginalFilename\", \"cru-modern.exe\"\n\
               VALUE \"ProductName\", \"CRU Modern\"\n\
               VALUE \"ProductVersion\", \"{version}.0\"\n\
             END\n\
           END\n\
           BLOCK \"VarFileInfo\"\n\
           BEGIN\n\
             VALUE \"Translation\", 0x0409, 1200\n\
           END\n\
         END\n"
    );
    std::fs::write(&rc_path, rc_content).expect("failed to write .rc file");

    let Some(windres) = find_windres() else {
        println!("cargo:warning=windres.exe not found; skipping .exe icon resource");
        return;
    };

    let obj_path = out_dir.join("cru-modern-icon.o");
    let status = Command::new(&windres)
        .args([
            rc_path.to_str().unwrap(),
            "-O",
            "coff",
            "-F",
            "pe-x86-64",
            "-o",
            obj_path.to_str().unwrap(),
        ])
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("cargo:rustc-link-arg-bin=cru-modern={}", obj_path.display());
        }
        Ok(s) => {
            println!("cargo:warning=windres exited with {s}; skipping .exe icon resource");
        }
        Err(e) => {
            println!("cargo:warning=failed to run windres: {e}");
        }
    }
}
