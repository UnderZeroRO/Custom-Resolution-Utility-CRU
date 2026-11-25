# CRU Modern

**A clean, native Rust reimplementation of Custom Resolution Utility (CRU) for Windows.**

CRU Modern reads and writes Windows `EDID_OVERRIDE` entries directly in the registry to add, edit, reorder, or remove custom monitor resolutions and timings — wrapped in a hardware-accelerated, single-file GUI with a restrained Sequoia-inspired design.

---

## Highlights

- **Single-file Windows executable** — Pure Rust (`eframe` / `egui` over OpenGL) with no WebView2, Electron, or external runtime DLLs.
- **Sequoia-inspired Dark & Light UI** — Subtle vertical background gradients, two-layer card shadows with luminous top hairlines, Segoe UI Variable typography, animated pill controls, and Windows 11 rounded corners via DWM.
- **Vector Hero & Custom Iconography** — Live vector monitor illustration rendering the active resolution, refresh rate, and aspect ratio inside the screen, plus resolution aspect badges (`26×16`) and hand-drawn `egui::Painter` icons at any DPI.
- **Native Graphics Driver Restart** — Restarts display adapters natively via Windows `SetupAPI` (`DIF_PROPERTYCHANGE` disable/enable) using a detached helper process so the OpenGL UI cleanly exits and relaunches with the new EDID active.
- **Byte-Exact EDID Engine** — Full 128-byte base block parsing & serialization, checksum validation, CVT-RB timing calculator, built-in DMT/CTA presets, and CTA-861 extension decoding.

---

## Features

| Tab / Area | Capabilities |
|---|---|
| **Display Sidebar** | Enumerates active displays (`EnumDisplayDevicesW`) and inactive registry entries (`HKLM\SYSTEM\CurrentControlSet\Enum\DISPLAY`), showing active mode, override badge, and quick `.bin` import/export. |
| **Hero Summary** | Displays scaled monitor illustration, manufacturer/hardware ID/product code/serial metadata, EDID version, block count, signal type, and one-click **Remove override**. |
| **Detailed Resolutions** | Add, edit, delete, and reorder Detailed Timing Descriptors (DTDs) with aspect-ratio previews, CVT-RB / **Optimize timings** (CVT-RB2 exact-Hz auto-tuner), custom blanking/sync/polarity controls, and real-time pixel clock / bandwidth validation. |
| **Established & Standard** | Toggle legacy Established Timings (grouped by standard) and inspect Standard Timings. |
| **Extension Blocks** | Decode CTA-861 revision, audio/YCbCr flags, data blocks, and read-only extension DTDs. |
| **Raw EDID** | Per-128-byte block hex dump with live checksum verification (`CHECKSUMS OK` / `BAD CHECKSUM`). |

---

## Getting Started

### Requirements

- **Windows 10 or Windows 11** (x64)
- **Administrator privileges** to write `EDID_OVERRIDE` under `HKLM\SYSTEM\CurrentControlSet\Enum\DISPLAY` and restart the graphics driver.
  - If launched without elevation, the status bar shows **Run as administrator** so you can elevate in one click. Importing/exporting `.bin` files works without elevation.

### Usage

1. Download `cru-modern.exe` from the [Releases](https://github.com/UnderZeroRO/cru-modern/releases) page (or build from source below) and run it anywhere — it is completely portable.
2. Select a display from the left sidebar.
3. Add or edit timings in **Detailed resolutions** (or toggle modes under **Established**).
4. Click **Apply** to write `Device Parameters\EDID_OVERRIDE` (`0`, `1`, … 128-byte `REG_BINARY` blocks).
5. Confirm **Repornește acum** (Restart driver) when prompted — the app exits briefly while its helper cycles the display adapter (`SetupAPI`), then relaunches automatically with the updated EDID active.
6. To revert back to your monitor's hardware EDID, click **Remove override** in the hero card and restart the driver.

---

## Building from Source

### Prerequisites

- **Rust** (`stable-x86_64-pc-windows-gnu` or `msvc`)
- Optional: `windres.exe` (MinGW-w64 / WinLibs) on `PATH` so `build.rs` embeds the multi-resolution `.ico` resource into `cru-modern.exe`

### Commands

```powershell
# Run core EDID & system unit tests
cargo test --lib

# Run UI, icon, and illustration tests
cargo test --bin cru-modern

# Build optimized single-file release executable
cargo build --release
```

The compiled binary is output to `target\release\cru-modern.exe` (`opt-level = 2`, thin LTO, stripped).

---

## Project Structure

```text
cru-modern/
├── Cargo.toml
├── build.rs                 # Generates multi-size .ico and embeds PE icon via windres
├── src/
│   ├── app_icon.rs          # Procedural 4x4-supersampled squircle icon + .ico encoder
│   ├── lib.rs               # Exposes edid and system modules for library tests
│   ├── main.rs              # eframe bootstrap, viewport icon, --restart-driver helper mode
│   ├── edid/
│   │   ├── mod.rs           # EDID base block parser/serializer & checksums
│   │   ├── timing.rs        # Timing model, CVT-RB calculation, presets, validation
│   │   └── cea.rs           # CTA-861 extension block decoder
│   ├── system/
│   │   ├── mod.rs           # System module exports
│   │   └── displays.rs      # Display enumeration, EDID_OVERRIDE registry I/O, SetupAPI restart
│   └── ui/
│       ├── mod.rs           # UI module exports
│       ├── app.rs           # Application state, top bar, sidebar, hero, tabs, dialogs, toast
│       ├── theme.rs         # Dark/Light palettes, gradients, two-layer shadows, pill controls
│       ├── icons.rs         # Vector icons drawn via egui::Painter
│       ├── illustration.rs  # Vector monitor illustration & aspect-ratio preview chips
│       └── win.rs           # Windows DWM dark title bar & Win11 rounded corners
├── examples/                # Small CLI utilities (display enumeration & SetupAPI checks)
└── tests/fixtures/          # Captured real-world EDID fixtures (128 / 256 bytes)
```

---

## Technical Notes

- **Windows Registry Format**: Windows reads display overrides from the subkey `HKLM\SYSTEM\CurrentControlSet\Enum\DISPLAY\<HWID>\<Instance>\Device Parameters\EDID_OVERRIDE`, stored as 128-byte `REG_BINARY` values named `"0"`, `"1"`, etc.
- **Why Driver Restart Uses a Helper Process**: Disabling the display adapter (`DICS_DISABLE`) invalidates live OpenGL contexts, which would freeze a running GUI window. `cru-modern` spawns itself with `--restart-driver --relaunch-after-restart --wait-pid <pid>`, exits immediately so the GL context is released, cycles the adapter via `SetupDiCallClassInstaller(DIF_PROPERTYCHANGE)`, and relaunches the GUI.

---

## Acknowledgments

Inspired by ToastyX's original **Custom Resolution Utility (CRU)**. This project is an independent Rust reimplementation created for modern Windows desktops.
