//! Video timing model, presets and validation.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timing {
    pub pixel_clock_khz: u32,
    pub hactive: u16,
    pub hblank: u16,
    pub vactive: u16,
    pub vblank: u16,
    pub hfront: u16,
    pub hsync: u16,
    pub vfront: u16,
    pub vsync: u16,
    pub hsize_mm: u16,
    pub vsize_mm: u16,
    pub hborder: u8,
    pub vborder: u8,
    pub interlaced: bool,
    pub hsync_positive: bool,
    pub vsync_positive: bool,
    pub stereo: u8,
    pub sync_type: SyncType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncType {
    AnalogComposite,
    BipolarAnalogComposite,
    DigitalComposite,
    DigitalSeparate,
}

impl SyncType {
    pub fn label(self) -> &'static str {
        match self {
            SyncType::AnalogComposite => "Analog composite",
            SyncType::BipolarAnalogComposite => "Bipolar analog composite",
            SyncType::DigitalComposite => "Digital composite",
            SyncType::DigitalSeparate => "Digital separate",
        }
    }

    pub fn from_raw(raw: u8) -> Self {
        match raw {
            0 => SyncType::AnalogComposite,
            1 => SyncType::BipolarAnalogComposite,
            2 => SyncType::DigitalComposite,
            _ => SyncType::DigitalSeparate,
        }
    }

    pub fn raw(self) -> u8 {
        match self {
            SyncType::AnalogComposite => 0,
            SyncType::BipolarAnalogComposite => 1,
            SyncType::DigitalComposite => 2,
            SyncType::DigitalSeparate => 3,
        }
    }
}

impl Default for Timing {
    fn default() -> Self {
        Self {
            pixel_clock_khz: 148_500,
            hactive: 1920,
            hblank: 280,
            vactive: 1080,
            vblank: 45,
            hfront: 88,
            hsync: 44,
            vfront: 4,
            vsync: 5,
            hsize_mm: 527,
            vsize_mm: 296,
            hborder: 0,
            vborder: 0,
            interlaced: false,
            hsync_positive: true,
            vsync_positive: true,
            stereo: 0,
            sync_type: SyncType::DigitalSeparate,
        }
    }
}

impl Timing {
    pub fn htotal(&self) -> u32 {
        self.hactive as u32 + self.hblank as u32
    }

    pub fn vtotal(&self) -> u32 {
        self.vactive as u32 + self.vblank as u32
    }

    /// Refresh rate in Hz (field rate for interlaced timings).
    pub fn refresh_hz(&self) -> f64 {
        let total = self.htotal() as f64 * self.vtotal() as f64;
        if total <= 0.0 {
            return 0.0;
        }
        self.pixel_clock_khz as f64 * 1000.0 / total
    }

    /// Frame rate in Hz (half the field rate when interlaced).
    pub fn frame_hz(&self) -> f64 {
        if self.interlaced {
            self.refresh_hz() / 2.0
        } else {
            self.refresh_hz()
        }
    }

    pub fn hfreq_khz(&self) -> f64 {
        if self.htotal() == 0 {
            return 0.0;
        }
        self.pixel_clock_khz as f64 / self.htotal() as f64
    }

    /// Raw RGB 8-bpc video data rate in Gbps (pixel clock × 24 bits).
    pub fn bandwidth_gbps(&self) -> f64 {
        self.pixel_clock_khz as f64 * 24.0 / 1_000_000.0
    }

    pub fn aspect_label(&self) -> String {
        let g = gcd(self.hactive as u32, self.vactive as u32);
        if g == 0 {
            return String::new();
        }
        format!("{}:{}", self.hactive as u32 / g, self.vactive as u32 / g)
    }

    /// "1920x1080 @ 60 Hz" style label.
    pub fn label(&self) -> String {
        let interlace = if self.interlaced { "i" } else { "" };
        format!(
            "{}x{}{} @ {:.0} Hz",
            self.hactive,
            self.vactive,
            interlace,
            self.frame_hz().round()
        )
    }

    /// Compact timing description used in tables: "H 88/44, V 4/5".
    pub fn sync_label(&self) -> String {
        format!(
            "H {}/{}  V {}/{}",
            self.hfront, self.hsync, self.vfront, self.vsync
        )
    }

    pub fn polarity_label(&self) -> &'static str {
        match (self.hsync_positive, self.vsync_positive) {
            (true, true) => "++",
            (true, false) => "+-",
            (false, true) => "-+",
            (false, false) => "--",
        }
    }

    /// Sanity check against the display limits.
    pub fn validate(&self, max_clock_khz: u32, max_hfreq_khz: u32) -> Validation {
        let mut result = Validation::default();

        if self.hactive == 0 || self.vactive == 0 {
            result.errors.push("Active area cannot be zero".into());
        }
        if self.htotal() <= self.hactive as u32 {
            result
                .errors
                .push("Horizontal total must exceed active width".into());
        }
        if self.vtotal() <= self.vactive as u32 {
            result
                .errors
                .push("Vertical total must exceed active height".into());
        }
        if self.hfront as u32 + self.hsync as u32 > self.hblank as u32 {
            result
                .errors
                .push("H front porch + sync exceed horizontal blanking".into());
        }
        if self.vfront as u32 + self.vsync as u32 > self.vblank as u32 {
            result
                .errors
                .push("V front porch + sync exceed vertical blanking".into());
        }
        if self.pixel_clock_khz > 655_350 {
            result
                .errors
                .push("Pixel clock above the EDID limit of 655.35 MHz".into());
        }
        if max_clock_khz > 0 && self.pixel_clock_khz > max_clock_khz {
            result.warnings.push(format!(
                "Pixel clock {:.2} MHz exceeds the display limit {:.2} MHz",
                self.pixel_clock_khz as f64 / 1000.0,
                max_clock_khz as f64 / 1000.0
            ));
        }
        if max_hfreq_khz > 0 && self.hfreq_khz() > max_hfreq_khz as f64 {
            result.warnings.push(format!(
                "Horizontal frequency {:.1} kHz exceeds the display limit {} kHz",
                self.hfreq_khz(),
                max_hfreq_khz
            ));
        }
        let refresh = self.refresh_hz();
        if !(20.0..=480.0).contains(&refresh) {
            result
                .warnings
                .push(format!("Unusual refresh rate: {refresh:.1} Hz"));
        }

        result
    }
}

#[derive(Clone, Debug, Default)]
pub struct Validation {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl Validation {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

/// VESA CVT reduced blanking (v1) timing for the given mode.
pub fn cvt_rb(hactive: u16, vactive: u16, refresh: f64) -> Timing {
    let hblank = 160.0f64;
    let htotal = hactive as f64 + hblank;

    let mut vbi = 9.0f64;
    let mut clock_hz = 0.0f64;
    for _ in 0..6 {
        let vtotal = vactive as f64 + vbi;
        clock_hz = htotal * vtotal * refresh;
        vbi = (460.0e-6 * clock_hz / htotal).ceil().max(9.0);
    }

    let (hsize, vsize) = default_size_mm(hactive, vactive);

    Timing {
        pixel_clock_khz: round_clock(clock_hz),
        hactive,
        hblank: 160,
        vactive,
        vblank: vbi.round() as u16,
        hfront: 48,
        hsync: 32,
        vfront: 3,
        vsync: 6,
        hsize_mm: hsize,
        vsize_mm: vsize,
        hborder: 0,
        vborder: 0,
        interlaced: false,
        hsync_positive: true,
        vsync_positive: false,
        stereo: 0,
        sync_type: SyncType::DigitalSeparate,
    }
}

/// Computes optimal timings for the target resolution and refresh rate.
///
/// Strategy:
/// 1. First prefers keeping the monitor's own working blanking/sync geometry
///    (`hblank`, `vblank`, `hfront`, `hsync`, `vfront`, `vsync`, polarity) with
///    at most tiny increments to hit an exact `10 kHz` EDID pixel clock step —
///    preventing overscan/cropping and color-space shifts on strict scalers.
/// 2. Falls back to VESA CVT-RB1 (`hblank >= 160`) only if the native geometry
///    exceeds EDID/display clock limits (or had oversized 50 Hz blanking).
pub fn optimize(
    current: &Timing,
    target_hz: f64,
    max_clock_khz: u32,
    max_hfreq_khz: u32,
) -> Timing {
    let hactive = current.hactive.max(64);
    let vactive = current.vactive.max(64);
    let target_hz = target_hz.clamp(20.0, 480.0);

    let vsync = if current.vsync >= 3 && current.vsync <= 12 {
        current.vsync
    } else {
        vsync_for_aspect(hactive, vactive)
    };
    let min_vbi_460us = ((460.0e-6 * target_hz * vactive as f64)
        / (1.0 - 460.0e-6 * target_hz).max(0.1))
    .ceil() as u16;

    let (hsize_mm, vsize_mm) = if current.hsize_mm > 0 && current.vsize_mm > 0 {
        (current.hsize_mm, current.vsize_mm)
    } else {
        default_size_mm(hactive, vactive)
    };

    let native_geometry_ok = (80..=360).contains(&current.hblank)
        && current.vblank >= vsync + 6
        && current.hfront + current.hsync < current.hblank
        && current.vfront + current.vsync < current.vblank;

    let mut best_score = (u8::MAX, u8::MAX, u32::MAX, u16::MAX, u32::MAX);
    let mut best_timing: Option<Timing> = None;

    let mut eval = |hblank: u16, vblank: u16, tier: u8, geo_dist: u16| {
        let htotal = hactive as u32 + hblank as u32;
        let vtotal = vactive as u32 + vblank as u32;
        if htotal == 0 || vtotal == 0 {
            return;
        }
        let ideal_hz = htotal as f64 * vtotal as f64 * target_hz;
        let clk_khz = round_clock(ideal_hz);
        if clk_khz > 655_350 {
            return;
        }

        let actual_hz = clk_khz as f64 * 1000.0 / (htotal as f64 * vtotal as f64);
        let hz_err = (actual_hz - target_hz).abs();
        let hfreq = clk_khz as f64 / htotal as f64;

        let over_display = u8::from(
            (max_clock_khz > 0 && clk_khz > max_clock_khz)
                || (max_hfreq_khz > 0 && hfreq > max_hfreq_khz as f64),
        );
        let err_milli = (hz_err * 1000.0).round().min(u32::MAX as f64) as u32;

        let score = (over_display, tier, err_milli, geo_dist, clk_khz);
        if score < best_score {
            best_score = score;
            let (hfront, hsync) = if native_geometry_ok
                && tier == 0
                && current.hfront + current.hsync < hblank
            {
                (current.hfront, current.hsync)
            } else {
                (48, 32)
            };
            let vfront = if native_geometry_ok
                && tier == 0
                && current.vfront + vsync < vblank
            {
                current.vfront
            } else {
                3.min(vblank.saturating_sub(vsync + 6)).max(1)
            };
            best_timing = Some(Timing {
                pixel_clock_khz: clk_khz,
                hactive,
                hblank,
                vactive,
                vblank,
                hfront,
                hsync,
                vfront,
                vsync,
                hsize_mm,
                vsize_mm,
                hborder: current.hborder,
                vborder: current.vborder,
                interlaced: false,
                hsync_positive: current.hsync_positive,
                vsync_positive: current.vsync_positive,
                stereo: current.stereo,
                sync_type: current.sync_type,
            });
        }
    };

    // Tier 0: preserve native blanking/porches with at most tiny positive nudges
    // so exact 10 kHz clock multiples don't alter image framing on the monitor.
    if native_geometry_ok {
        for dh in (0..=24u16).step_by(2) {
            for dv in 0..=12u16 {
                let hb = current.hblank.saturating_add(dh);
                let vb = current.vblank.saturating_add(dv);
                eval(hb, vb, 0, dh + dv);
            }
        }
    }

    // Tier 1: safe VESA CVT-RB1 fallback (hblank >= 160) if Tier 0 exceeded clock limits.
    let rb1_v_min = min_vbi_460us.max(vsync + 12).max(20);
    for hblank in (160..=320).step_by(4) {
        for vblank in rb1_v_min..=rb1_v_min.saturating_add(28) {
            eval(hblank, vblank, 1, hblank.abs_diff(160));
        }
    }

    best_timing.unwrap_or_else(|| {
        let mut t = cvt_rb(hactive, vactive, target_hz);
        t.hsize_mm = hsize_mm;
        t.vsize_mm = vsize_mm;
        t.hsync_positive = current.hsync_positive;
        t.vsync_positive = current.vsync_positive;
        t
    })
}

fn vsync_for_aspect(hactive: u16, vactive: u16) -> u16 {
    if vactive == 0 {
        return 5;
    }
    let ratio = hactive as f64 / vactive as f64;
    if (ratio - 4.0 / 3.0).abs() < 0.05 {
        4
    } else if (ratio - 16.0 / 9.0).abs() < 0.06 {
        5
    } else if (ratio - 16.0 / 10.0).abs() < 0.05 {
        6
    } else if (ratio - 5.0 / 4.0).abs() < 0.05 || (ratio - 15.0 / 9.0).abs() < 0.05 {
        7
    } else {
        8
    }
}

fn default_size_mm(hactive: u16, vactive: u16) -> (u16, u16) {
    let hsize = (hactive as f64 / 3.6).round() as u16;
    let vsize = if hactive == 0 {
        0
    } else {
        (vactive as f64 * hsize as f64 / hactive as f64).round() as u16
    };
    (hsize, vsize)
}

pub struct Preset {
    pub name: &'static str,
    pub hactive: u16,
    pub vactive: u16,
    pub refresh: u16,
    pub timing: Timing,
}

pub fn presets() -> Vec<Preset> {
    // name, w, h, hz, clock_khz, hblank, vblank, hfront, hsync, vfront, vsync, hmm, vmm, interlaced, h+, v+
    let defs: &[(&str, u16, u16, u16, u32, u16, u16, u16, u16, u16, u16, u16, u16, bool, bool, bool)] = &[
        ("640x480 @ 60 (DMT)", 640, 480, 60, 25_175, 160, 45, 16, 96, 10, 2, 169, 127, false, false, false),
        ("800x600 @ 60 (DMT)", 800, 600, 60, 40_000, 256, 28, 40, 128, 1, 4, 211, 158, false, true, true),
        ("1024x768 @ 60 (DMT)", 1024, 768, 60, 65_000, 320, 38, 24, 136, 3, 6, 270, 203, false, false, false),
        ("1280x720 @ 60 (CTA)", 1280, 720, 60, 74_250, 370, 30, 110, 40, 5, 5, 338, 190, false, true, true),
        ("1280x1024 @ 60 (DMT)", 1280, 1024, 60, 108_000, 408, 42, 48, 112, 1, 3, 338, 270, false, true, true),
        ("1366x768 @ 60 (DMT)", 1366, 768, 60, 85_500, 426, 27, 72, 136, 3, 6, 344, 194, false, true, true),
        ("1600x900 @ 60 (DMT)", 1600, 900, 60, 108_000, 200, 100, 24, 80, 1, 3, 422, 238, false, true, true),
        ("1920x1080 @ 60 (CTA)", 1920, 1080, 60, 148_500, 280, 45, 88, 44, 4, 5, 527, 296, false, true, true),
        ("1920x1080 @ 50 (CTA)", 1920, 1080, 50, 148_500, 720, 45, 528, 44, 4, 5, 527, 296, false, true, true),
        ("1920x1080 @ 25 (CTA)", 1920, 1080, 25, 74_250, 720, 45, 528, 44, 4, 5, 527, 296, false, true, true),
        ("1920x1080 @ 100 (CTA)", 1920, 1080, 100, 297_000, 720, 45, 528, 44, 4, 5, 527, 296, false, true, true),
        ("1920x1080 @ 120 (CTA)", 1920, 1080, 120, 297_000, 280, 45, 88, 44, 4, 5, 527, 296, false, true, true),
        ("1920x1080i @ 60 (CTA)", 1920, 540, 60, 74_250, 280, 22, 88, 44, 2, 5, 527, 296, true, true, true),
        ("2560x1440 @ 60 (DMT-RB)", 2560, 1440, 60, 241_500, 160, 41, 48, 32, 3, 5, 597, 336, false, true, true),
        ("2560x1600 @ 60 (DMT-RB)", 2560, 1600, 60, 268_500, 160, 46, 48, 32, 3, 6, 641, 400, false, true, true),
        ("3440x1440 @ 60 (CVT-RB)", 3440, 1440, 60, 319_800, 160, 41, 48, 32, 3, 5, 800, 335, false, true, true),
        ("3840x2160 @ 30 (CTA)", 3840, 2160, 30, 297_000, 560, 90, 176, 88, 8, 10, 621, 341, false, true, true),
        ("3840x2160 @ 60 (CTA)", 3840, 2160, 60, 594_000, 560, 90, 176, 88, 8, 10, 621, 341, false, true, true),
    ];

    defs.iter()
        .map(|d| Preset {
            name: d.0,
            hactive: d.1,
            vactive: d.2,
            refresh: d.3,
            timing: Timing {
                pixel_clock_khz: d.4,
                hactive: d.1,
                vactive: d.2,
                hblank: d.5,
                vblank: d.6,
                hfront: d.7,
                hsync: d.8,
                vfront: d.9,
                vsync: d.10,
                hsize_mm: d.11,
                vsize_mm: d.12,
                interlaced: d.13,
                hsync_positive: d.14,
                vsync_positive: d.15,
                ..Timing::default()
            },
        })
        .collect()
}

fn round_clock(hz: f64) -> u32 {
    let khz = hz / 1000.0;
    ((khz / 10.0).round() as u32).max(1) * 10
}

fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

impl fmt::Display for Timing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cvt_rb_matches_known_1080p60() {
        let t = cvt_rb(1920, 1080, 60.0);
        assert_eq!(t.htotal(), 2080);
        assert!((t.vtotal() as i32 - 1111).abs() <= 2, "vtotal={}", t.vtotal());
        assert!(
            (t.pixel_clock_khz as i32 - 138_650).abs() <= 200,
            "clock={}",
            t.pixel_clock_khz
        );
        assert!((t.refresh_hz() - 60.0).abs() < 0.3);
    }

    #[test]
    fn presets_are_self_consistent() {
        for p in presets() {
            let t = p.timing;
            assert!(
                t.hfront as u32 + t.hsync as u32 <= t.hblank as u32,
                "{}",
                p.name
            );
            assert!(
                t.vfront as u32 + t.vsync as u32 <= t.vblank as u32,
                "{}",
                p.name
            );
            let refresh = t.refresh_hz();
            assert!(
                (refresh - p.refresh as f64).abs() < 1.5,
                "{}: {} vs {}",
                p.name,
                refresh,
                p.refresh
            );
        }
    }

    #[test]
    fn aspect_is_reduced() {
        let t = Timing::default();
        assert_eq!(t.aspect_label(), "16:9");
    }

    #[test]
    fn optimize_hits_exact_144hz_with_valid_blanking() {
        let base = Timing::default();
        let t = optimize(&base, 144.0, 0, 0);
        assert_eq!((t.hsize_mm, t.vsize_mm), (base.hsize_mm, base.vsize_mm));
        assert_eq!(
            (t.hsync_positive, t.vsync_positive),
            (base.hsync_positive, base.vsync_positive)
        );
        assert_eq!((t.hblank, t.vblank), (base.hblank, base.vblank));
        assert_eq!((t.hfront, t.hsync, t.vfront, t.vsync), (base.hfront, base.hsync, base.vfront, base.vsync));
        assert!((t.frame_hz() - 144.0).abs() < 0.002, "hz={}", t.frame_hz());
        assert!(t.validate(0, 0).is_valid());
    }

    #[test]
    fn optimize_fits_1080p240_within_edid_limit() {
        let base = Timing::default();
        let t = optimize(&base, 240.0, 0, 0);
        assert!(t.hblank >= 160, "hblank={}", t.hblank);
        assert!(t.validate(0, 0).is_valid());
        assert!(t.pixel_clock_khz <= 655_350);
        assert!((t.frame_hz() - 240.0).abs() < 0.05, "hz={}", t.frame_hz());
    }
}
