//! Prints the displays found in the registry, for diagnostics.

use winreg::enums::*;
use winreg::RegKey;

fn main() {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    for flags in [
        KEY_READ,
        KEY_READ | KEY_WOW64_64KEY,
        KEY_READ | KEY_WOW64_32KEY,
    ] {
        let res = hklm.open_subkey_with_flags(
            "SYSTEM\\CurrentControlSet\\Enum\\DISPLAY",
            flags,
        );
        println!("flags {flags:#X}: {:?}", res.as_ref().err().map(|e| e.to_string()));
        if let Ok(open) = &res {
            let keys: Vec<String> = open.enum_keys().flatten().collect();
            println!("  hw ids: {keys:?}");
            for hw in keys {
                if let Ok(k) = open.open_subkey_with_flags(&hw, flags) {
                    let inst: Vec<String> = k.enum_keys().flatten().collect();
                    println!("    {hw}: {inst:?}");
                    for i in inst {
                        let p = format!("{hw}\\{i}\\Device Parameters");
                        match open.open_subkey_with_flags(&p, flags) {
                            Ok(dp) => {
                                let edid: Option<Vec<u8>> = dp
                                    .get_raw_value("EDID")
                                    .ok()
                                    .map(|v| v.bytes.to_vec());
                                println!(
                                    "      {p}: edid={:?}",
                                    edid.map(|e| e.len())
                                );
                            }
                            Err(e) => println!("      {p}: ERR {e}"),
                        }
                    }
                }
            }
        }
    }

    let displays = cru_modern::system::enumerate_displays();
    println!("enumerate_displays: {}", displays.len());
    println!("elevated: {}", cru_modern::system::is_elevated());
}
