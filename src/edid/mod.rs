//! EDID 1.3/1.4 parser and serializer.
//!
//! The base block is decoded into editable fields; extension blocks are kept
//! byte-exact and parsed on demand (CTA-861 is decoded for display).

pub mod cea;
pub mod timing;

pub use timing::{Timing, Validation};

use thiserror::Error;

pub const HEADER: [u8; 8] = [0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00];

#[derive(Debug, Error)]
pub enum EdidError {
    #[error("EDID must be at least 128 bytes (got {0})")]
    TooShort(usize),
    #[error("invalid EDID length: {0} bytes is not a multiple of 128")]
    BadLength(usize),
    #[error("missing EDID header (00 FF FF FF FF FF FF 00)")]
    BadHeader,
}

#[derive(Clone, Debug)]
pub struct Edid {
    pub base: BaseBlock,
    pub extensions: Vec<Vec<u8>>,
}

impl Edid {
    pub fn parse(data: &[u8]) -> Result<Self, EdidError> {
        if data.len() < 128 {
            return Err(EdidError::TooShort(data.len()));
        }
        if data.len() % 128 != 0 {
            return Err(EdidError::BadLength(data.len()));
        }
        if data[0..8] != HEADER {
            return Err(EdidError::BadHeader);
        }

        let base = BaseBlock::parse(&data[0..128]);
        let extensions = (1..data.len() / 128)
            .map(|i| data[i * 128..(i + 1) * 128].to_vec())
            .collect();

        Ok(Self { base, extensions })
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(128 * (1 + self.extensions.len()));
        out.extend_from_slice(&self.base.serialize(self.extensions.len() as u8));
        for ext in &self.extensions {
            let mut block = ext.clone();
            block.resize(128, 0);
            fix_checksum(&mut block[..128]);
            out.extend_from_slice(&block);
        }
        out
    }

    pub fn block_count(&self) -> usize {
        1 + self.extensions.len()
    }

    pub fn checksum_ok(&self) -> Vec<bool> {
        let data = self.serialize();
        data.chunks(128).map(|b| b.iter().fold(0u8, |a, &x| a.wrapping_add(x)) == 0).collect()
    }

    pub fn is_modified(&self, original: &[u8]) -> bool {
        let mut current = self.serialize();
        let mut original = original.to_vec();
        original.resize(current.len().max(original.len()), 0);
        current.resize(original.len(), 0);
        current != original
    }

    /// Maximum pixel clock declared by the range limits descriptor, if any.
    pub fn max_pixel_clock_khz(&self) -> u32 {
        for d in &self.base.descriptors {
            if let Descriptor::RangeLimits(r) = d {
                if r.max_clock_10mhz > 0 {
                    return r.max_clock_10mhz as u32 * 10_000;
                }
            }
        }
        0
    }

    /// Maximum horizontal frequency declared by the range limits descriptor.
    pub fn max_hfreq_khz(&self) -> u32 {
        for d in &self.base.descriptors {
            if let Descriptor::RangeLimits(r) = d {
                return r.max_h as u32;
            }
        }
        0
    }

    pub fn detailed_timings(&self) -> Vec<Timing> {
        self.base
            .descriptors
            .iter()
            .filter_map(|d| match d {
                Descriptor::Detailed(t) => Some(*t),
                _ => None,
            })
            .collect()
    }

    pub fn free_descriptor_slot(&self) -> Option<usize> {
        self.base
            .descriptors
            .iter()
            .enumerate()
            .rev()
            .find(|(_, d)| matches!(d, Descriptor::Dummy))
            .map(|(i, _)| i)
    }
}

fn fix_checksum(block: &mut [u8]) {
    let sum: u8 = block[..127].iter().fold(0u8, |a, &x| a.wrapping_add(x));
    block[127] = 0u8.wrapping_sub(sum);
}

#[derive(Clone, Debug)]
pub struct BaseBlock {
    pub manufacturer_id: u16,
    pub product_code: u16,
    pub serial: u32,
    pub manufacture_week: u8,
    pub manufacture_year: u16,
    pub version: (u8, u8),
    pub video_input: u8,
    pub width_cm: u8,
    pub height_cm: u8,
    pub gamma: u8,
    pub features: u8,
    pub chromaticity: [u8; 10],
    pub established: [u8; 3],
    pub standard: [StandardTiming; 8],
    pub descriptors: [Descriptor; 4],
}

impl BaseBlock {
    fn parse(b: &[u8]) -> Self {
        let mut standard = [StandardTiming::unused(); 8];
        for (i, slot) in standard.iter_mut().enumerate() {
            *slot = StandardTiming::parse([b[38 + i * 2], b[39 + i * 2]]);
        }

        let mut descriptors: [Descriptor; 4] = [
            Descriptor::Dummy,
            Descriptor::Dummy,
            Descriptor::Dummy,
            Descriptor::Dummy,
        ];
        for (i, slot) in descriptors.iter_mut().enumerate() {
            let start = 54 + i * 18;
            let mut bytes = [0u8; 18];
            bytes.copy_from_slice(&b[start..start + 18]);
            *slot = Descriptor::parse(&bytes);
        }

        Self {
            manufacturer_id: u16::from_be_bytes([b[8], b[9]]),
            product_code: u16::from_le_bytes([b[10], b[11]]),
            serial: u32::from_le_bytes([b[12], b[13], b[14], b[15]]),
            manufacture_week: b[16],
            manufacture_year: 1990 + b[17] as u16,
            version: (b[18], b[19]),
            video_input: b[20],
            width_cm: b[21],
            height_cm: b[22],
            gamma: b[23],
            features: b[24],
            chromaticity: [b[25], b[26], b[27], b[28], b[29], b[30], b[31], b[32], b[33], b[34]],
            established: [b[35], b[36], b[37]],
            standard,
            descriptors,
        }
    }

    pub fn serialize(&self, extension_count: u8) -> Vec<u8> {
        let mut b = vec![0u8; 128];
        b[0..8].copy_from_slice(&HEADER);
        b[8..10].copy_from_slice(&self.manufacturer_id.to_be_bytes());
        b[10..12].copy_from_slice(&self.product_code.to_le_bytes());
        b[12..16].copy_from_slice(&self.serial.to_le_bytes());
        b[16] = self.manufacture_week;
        b[17] = (self.manufacture_year.saturating_sub(1990)).min(255) as u8;
        b[18] = self.version.0;
        b[19] = self.version.1;
        b[20] = self.video_input;
        b[21] = self.width_cm;
        b[22] = self.height_cm;
        b[23] = self.gamma;
        b[24] = self.features;
        b[25..35].copy_from_slice(&self.chromaticity);
        b[35..38].copy_from_slice(&self.established);
        for (i, st) in self.standard.iter().enumerate() {
            let bytes = st.serialize();
            b[38 + i * 2] = bytes[0];
            b[39 + i * 2] = bytes[1];
        }
        for (i, d) in self.descriptors.iter().enumerate() {
            let bytes = d.serialize();
            b[54 + i * 18..54 + (i + 1) * 18].copy_from_slice(&bytes);
        }
        b[126] = extension_count;
        fix_checksum(&mut b);
        b
    }

    pub fn manufacturer(&self) -> String {
        let mut out = String::new();
        for shift in [10u32, 5, 0] {
            let v = ((self.manufacturer_id as u32 >> shift) & 0x1F) as u8;
            out.push((b'A' + v.saturating_sub(1)) as char);
        }
        out
    }

    pub fn digital(&self) -> bool {
        self.video_input & 0x80 != 0
    }

    pub fn bit_depth(&self) -> u8 {
        if !self.digital() {
            return 0;
        }
        match (self.video_input >> 4) & 0x07 {
            1 => 6,
            2 => 8,
            3 => 10,
            4 => 12,
            5 => 14,
            6 => 16,
            _ => 0,
        }
    }

    pub fn interface(&self) -> &'static str {
        if !self.digital() {
            return "Analog";
        }
        if self.version < (1, 4) {
            return if self.video_input & 0x01 != 0 {
                "DVI"
            } else {
                "Digital"
            };
        }
        match self.video_input & 0x0F {
            1 => "DVI",
            2 => "HDMI-A",
            3 => "HDMI-B",
            4 => "MDDI",
            5 => "DisplayPort",
            _ => "Digital",
        }
    }

    pub fn gamma_value(&self) -> f32 {
        if self.gamma == 0xFF {
            0.0
        } else {
            (self.gamma as f32 + 100.0) / 100.0
        }
    }

    pub fn model_year(&self) -> Option<u16> {
        if self.manufacture_week == 0xFF {
            None
        } else {
            Some(self.manufacture_year)
        }
    }

    pub fn screen_size_cm(&self) -> Option<(u8, u8)> {
        if self.width_cm > 0 && self.height_cm > 0 {
            Some((self.width_cm, self.height_cm))
        } else {
            None
        }
    }

    /// Diagonal in inches, derived from the physical size.
    pub fn diagonal_inches(&self) -> Option<f32> {
        self.screen_size_cm().map(|(w, h)| {
            ((w as f32).powi(2) + (h as f32).powi(2)).sqrt() / 2.54
        })
    }

    pub fn established(&self, index: u8) -> bool {
        if index >= 24 {
            return false;
        }
        self.established[(index / 8) as usize] & (0x80 >> (index % 8)) != 0
    }

    pub fn set_established(&mut self, index: u8, value: bool) {
        if index >= 24 {
            return;
        }
        let byte = (index / 8) as usize;
        let mask = 0x80 >> (index % 8);
        if value {
            self.established[byte] |= mask;
        } else {
            self.established[byte] &= !mask;
        }
    }

    pub fn preferred_timing(&self) -> bool {
        self.features & 0x02 != 0
    }
}

/// The established timings bitmap, in the same order CRU exposes it.
pub const ESTABLISHED_MODES: [&str; 17] = [
    "720x400 @ 70 Hz",
    "720x400 @ 88 Hz",
    "640x480 @ 60 Hz",
    "640x480 @ 67 Hz",
    "640x480 @ 72 Hz",
    "640x480 @ 75 Hz",
    "800x600 @ 56 Hz",
    "800x600 @ 60 Hz",
    "800x600 @ 72 Hz",
    "800x600 @ 75 Hz",
    "832x624 @ 75 Hz",
    "1024x768i @ 87 Hz",
    "1024x768 @ 60 Hz",
    "1024x768 @ 70 Hz",
    "1024x768 @ 75 Hz",
    "1280x1024 @ 75 Hz",
    "1152x870 @ 75 Hz",
];

/// Groups used by the established timings panel.
pub const ESTABLISHED_GROUPS: [(&str, &[u8]); 4] = [
    ("640x480", &[2, 3, 4, 5]),
    ("800x600", &[6, 7, 8, 9]),
    ("1024x768", &[11, 12, 13, 14]),
    ("1152x870 / 1280x1024", &[15, 16]),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StandardTiming {
    pub hactive: u16,
    pub aspect: u8,
    pub refresh: u8,
}

impl StandardTiming {
    pub const fn unused() -> Self {
        Self { hactive: 0, aspect: 0, refresh: 0 }
    }

    pub fn parse(bytes: [u8; 2]) -> Self {
        if bytes[0] == 0x01 && bytes[1] == 0x01 || bytes[0] == 0x00 || bytes[0] == 0x20 {
            return Self::unused();
        }
        Self {
            hactive: (bytes[0] as u16 + 31) * 8,
            aspect: bytes[1] >> 6,
            refresh: (bytes[1] & 0x3F) + 60,
        }
    }

    pub fn serialize(&self) -> [u8; 2] {
        if self.hactive == 0 {
            return [0x01, 0x01];
        }
        let h = (self.hactive / 8).saturating_sub(31).min(255) as u8;
        let r = self.refresh.saturating_sub(60).min(63);
        [h, (self.aspect << 6) | r]
    }

    pub fn vactive(&self) -> u16 {
        match self.aspect {
            0 => self.hactive * 10 / 16,
            1 => self.hactive * 3 / 4,
            2 => self.hactive * 4 / 5,
            _ => self.hactive * 9 / 16,
        }
    }

    pub fn aspect_label(&self) -> &'static str {
        match self.aspect {
            0 => "16:10",
            1 => "4:3",
            2 => "5:4",
            _ => "16:9",
        }
    }

    pub fn label(&self) -> String {
        if self.hactive == 0 {
            "Unused".to_string()
        } else {
            format!("{}x{} @ {} Hz ({})", self.hactive, self.vactive(), self.refresh, self.aspect_label())
        }
    }
}

#[derive(Clone, Debug)]
pub struct RangeLimits {
    pub min_v: u8,
    pub max_v: u8,
    pub min_h: u8,
    pub max_h: u8,
    pub max_clock_10mhz: u8,
    pub flags: u8,
    /// Bytes 11..18 of the descriptor: the 0x0A marker plus padding.
    pub padding: [u8; 7],
}

impl RangeLimits {
    pub fn label(&self) -> String {
        format!(
            "{}â€“{} Hz vertical, {}â€“{} kHz horizontal, {:.2} MHz max clock",
            self.min_v,
            self.max_v,
            self.min_h,
            self.max_h,
            self.max_clock_10mhz as f32 * 10.0
        )
    }
}

#[derive(Clone, Debug)]
pub enum Descriptor {
    Detailed(Timing),
    MonitorName(String),
    SerialNumber(String),
    Text(String),
    RangeLimits(RangeLimits),
    StandardTimings(Vec<StandardTiming>),
    ColorPoint,
    EstablishedThree([u8; 7]),
    Dummy,
    Unknown { tag: u8, data: [u8; 13] },
}

impl Descriptor {
    pub fn parse(b: &[u8; 18]) -> Self {
        if b[0] != 0 || b[1] != 0 {
            return Descriptor::Detailed(Timing::from_raw(b));
        }
        let tag = b[3];
        let data = &b[5..18];
        match tag {
            0xFC => Descriptor::MonitorName(clean_text(data)),
            0xFF => Descriptor::SerialNumber(clean_text(data)),
            0xFE => Descriptor::Text(clean_text(data)),
            0xFD => Descriptor::RangeLimits(RangeLimits {
                min_v: data[0],
                max_v: data[1],
                min_h: data[2],
                max_h: data[3],
                max_clock_10mhz: data[4],
                flags: data[5],
                padding: {
                    let mut p = [0u8; 7];
                    p.copy_from_slice(&data[6..13]);
                    p
                },
            }),
            0xFA => {
                let mut items = Vec::new();
                for i in 0..6 {
                    let st = StandardTiming::parse([data[i * 2], data[i * 2 + 1]]);
                    if st.hactive != 0 {
                        items.push(st);
                    }
                }
                Descriptor::StandardTimings(items)
            }
            0xFB => Descriptor::ColorPoint,
            0xF7 => {
                let mut d = [0u8; 7];
                d.copy_from_slice(&data[0..7]);
                Descriptor::EstablishedThree(d)
            }
            0x10 => Descriptor::Dummy,
            other => {
                let mut d = [0u8; 13];
                d.copy_from_slice(data);
                Descriptor::Unknown { tag: other, data: d }
            }
        }
    }

    pub fn serialize(&self) -> [u8; 18] {
        let mut b = [0u8; 18];
        match self {
            Descriptor::Detailed(t) => return t.to_raw(),
            Descriptor::MonitorName(s) => {
                b[3] = 0xFC;
                write_text(&mut b[5..18], s);
            }
            Descriptor::SerialNumber(s) => {
                b[3] = 0xFF;
                write_text(&mut b[5..18], s);
            }
            Descriptor::Text(s) => {
                b[3] = 0xFE;
                write_text(&mut b[5..18], s);
            }
            Descriptor::RangeLimits(r) => {
                b[3] = 0xFD;
                b[5] = r.min_v;
                b[6] = r.max_v;
                b[7] = r.min_h;
                b[8] = r.max_h;
                b[9] = r.max_clock_10mhz;
                b[10] = r.flags;
                b[11..18].copy_from_slice(&r.padding);
            }
            Descriptor::StandardTimings(items) => {
                b[3] = 0xFA;
                for (i, st) in items.iter().take(6).enumerate() {
                    let bytes = st.serialize();
                    b[5 + i * 2] = bytes[0];
                    b[6 + i * 2] = bytes[1];
                }
            }
            Descriptor::ColorPoint => b[3] = 0xFB,
            Descriptor::EstablishedThree(d) => {
                b[3] = 0xF7;
                b[5..12].copy_from_slice(d);
            }
            Descriptor::Dummy => b[3] = 0x10,
            Descriptor::Unknown { tag, data } => {
                b[3] = *tag;
                b[5..18].copy_from_slice(data);
            }
        }
        b
    }

    pub fn kind_label(&self) -> &'static str {
        match self {
            Descriptor::Detailed(_) => "Detailed timing",
            Descriptor::MonitorName(_) => "Monitor name",
            Descriptor::SerialNumber(_) => "Serial number",
            Descriptor::Text(_) => "Text",
            Descriptor::RangeLimits(_) => "Range limits",
            Descriptor::StandardTimings(_) => "Standard timings",
            Descriptor::ColorPoint => "Color point",
            Descriptor::EstablishedThree(_) => "Established timings III",
            Descriptor::Dummy => "Unused",
            Descriptor::Unknown { .. } => "Unknown",
        }
    }

    pub fn summary(&self) -> String {
        match self {
            Descriptor::Detailed(t) => t.label(),
            Descriptor::MonitorName(s) | Descriptor::SerialNumber(s) | Descriptor::Text(s) => s.clone(),
            Descriptor::RangeLimits(r) => r.label(),
            Descriptor::StandardTimings(items) => format!("{} timing(s)", items.len()),
            Descriptor::ColorPoint => "White point data".into(),
            Descriptor::EstablishedThree(_) => "Extended established timings".into(),
            Descriptor::Dummy => "â€”".into(),
            Descriptor::Unknown { tag, .. } => format!("Descriptor 0x{tag:02X}"),
        }
    }

    pub fn is_removable(&self) -> bool {
        matches!(self, Descriptor::Detailed(_))
    }
}

impl Timing {
    /// Decode a detailed timing descriptor.
    pub fn from_raw(b: &[u8]) -> Self {
        let pixel_clock_khz = ((b[1] as u32) << 8 | b[0] as u32) * 10;
        let hactive = b[2] as u16 | ((b[4] as u16 & 0xF0) << 4);
        let hblank = b[3] as u16 | ((b[4] as u16 & 0x0F) << 8);
        let vactive = b[5] as u16 | ((b[7] as u16 & 0xF0) << 4);
        let vblank = b[6] as u16 | ((b[7] as u16 & 0x0F) << 8);
        let hfront = b[8] as u16 | ((b[11] as u16 & 0xC0) << 2);
        let hsync = b[9] as u16 | ((b[11] as u16 & 0x30) << 4);
        let vfront = (b[10] as u16 >> 4) | ((b[11] as u16 & 0x0C) << 2);
        let vsync = (b[10] as u16 & 0x0F) | ((b[11] as u16 & 0x03) << 4);
        let hsize_mm = b[12] as u16 | ((b[14] as u16 & 0xF0) << 4);
        let vsize_mm = b[13] as u16 | ((b[14] as u16 & 0x0F) << 8);

        let flags = b[17];
        let sync_type = SyncType::from_raw((flags >> 3) & 0x03);
        let hsync_positive = flags & 0x02 != 0;
        let vsync_positive = flags & 0x04 != 0;

        Timing {
            pixel_clock_khz,
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
            hborder: b[15],
            vborder: b[16],
            interlaced: flags & 0x80 != 0,
            hsync_positive,
            vsync_positive,
            stereo: ((flags >> 5) & 0x03) | ((flags & 0x01) << 2),
            sync_type,
        }
    }

    /// Encode as a detailed timing descriptor.
    pub fn to_raw(&self) -> [u8; 18] {
        let mut b = [0u8; 18];
        let clock = (self.pixel_clock_khz / 10).min(0xFFFF) as u16;
        b[0] = clock as u8;
        b[1] = (clock >> 8) as u8;
        b[2] = self.hactive as u8;
        b[3] = self.hblank as u8;
        b[4] = (((self.hactive >> 8) & 0x0F) as u8) << 4 | ((self.hblank >> 8) & 0x0F) as u8;
        b[5] = self.vactive as u8;
        b[6] = self.vblank as u8;
        b[7] = (((self.vactive >> 8) & 0x0F) as u8) << 4 | ((self.vblank >> 8) & 0x0F) as u8;
        b[8] = self.hfront as u8;
        b[9] = self.hsync as u8;
        b[10] = ((self.vfront & 0x0F) as u8) << 4 | (self.vsync & 0x0F) as u8;
        b[11] = ((self.hfront >> 8) & 0x03) as u8
            | (((self.hsync >> 8) & 0x03) as u8) << 4
            | (((self.vfront >> 4) & 0x03) as u8) << 2
            | ((self.vsync >> 4) & 0x03) as u8;
        b[12] = self.hsize_mm as u8;
        b[13] = self.vsize_mm as u8;
        b[14] = (((self.hsize_mm >> 8) & 0x0F) as u8) << 4 | ((self.vsize_mm >> 8) & 0x0F) as u8;
        b[15] = self.hborder;
        b[16] = self.vborder;
        let stereo = (self.stereo & 0x03) << 5 | (self.stereo >> 2) & 0x01;
        b[17] = (self.interlaced as u8) << 7
            | stereo
            | self.sync_type.raw() << 3
            | (self.vsync_positive as u8) << 2
            | (self.hsync_positive as u8) << 1;
        b
    }
}

use timing::SyncType;

fn clean_text(data: &[u8]) -> String {
    let text: String = data
        .iter()
        .take_while(|&&c| c != 0x0A && c != 0x00)
        .map(|&c| if c == 0x20 { ' ' } else { c as char })
        .collect();
    text.trim().to_string()
}

fn write_text(dst: &mut [u8], text: &str) {
    for (i, byte) in dst.iter_mut().enumerate() {
        if i == text.len() {
            *byte = 0x0A;
        } else if i > text.len() {
            *byte = 0x20;
        }
    }
    for (i, ch) in text.as_bytes().iter().take(dst.len()).enumerate() {
        dst[i] = *ch;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        s.split_whitespace()
            .map(|t| u8::from_str_radix(t, 16).unwrap())
            .collect()
    }

    // Real EDIDs captured from this machine (tests/fixtures).
    const SAM0D46: &str = include_str!("../../tests/fixtures/SAM0D46-256.hex");
    const ELO: &str = include_str!("../../tests/fixtures/ELO2201-256.hex");
    const SAM0D46_BASE: &str = include_str!("../../tests/fixtures/SAM0D46-128.hex");
    const RGT1352: &str = include_str!("../../tests/fixtures/RGT1352-256.hex");

    #[test]
    fn parses_samsung_edid() {
        let e = Edid::parse(&hex(SAM0D46)).unwrap();
        assert_eq!(e.base.manufacturer(), "SAM");
        assert_eq!(e.base.product_code, 0x0D46);
        assert_eq!(e.base.version, (1, 3));
        assert!(e.base.digital());
        assert_eq!(e.base.interface(), "Digital");
        assert_eq!(e.block_count(), 2);

        let name = e
            .base
            .descriptors
            .iter()
            .find_map(|d| match d {
                Descriptor::MonitorName(n) => Some(n.clone()),
                _ => None,
            })
            .unwrap();
        assert_eq!(name, "S19F350");

        let t = e.detailed_timings()[0];
        assert_eq!((t.hactive, t.vactive), (1366, 768));
        assert_eq!(t.htotal(), 1792);
        assert!((t.frame_hz() - 75.0).abs() < 0.5);
        assert!(t.hsync_positive && t.vsync_positive);
    }

    #[test]
    fn roundtrip_is_byte_exact() {
        for data in [hex(SAM0D46), hex(ELO)] {
            let e = Edid::parse(&data).unwrap();
            assert_eq!(e.serialize(), data);
        }
    }

    #[test]
    fn cta_extension_is_decoded() {
        let e = Edid::parse(&hex(SAM0D46)).unwrap();
        let cta = cea::Cta::parse(&e.extensions[0]).expect("CTA block");
        assert_eq!(cta.revision, 3);
        assert!(cta.data_blocks.iter().any(|b| b.tag == 2), "video block");
        assert!(!cta.dtds.is_empty());
    }

    #[test]
    fn all_captured_fixtures_roundtrip() {
        for (name, data) in [
            ("SAM0D46-base", hex(SAM0D46_BASE)),
            ("RGT1352", hex(RGT1352)),
            ("ELO", hex(ELO)),
            ("SAM0D46", hex(SAM0D46)),
        ] {
            let edid = Edid::parse(&data).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(edid.base.manufacturer().len(), 3, "{name}");
        }

        for (name, data) in [("RGT1352", hex(RGT1352)), ("ELO", hex(ELO)), ("SAM0D46", hex(SAM0D46))] {
            let edid = Edid::parse(&data).unwrap();
            assert_eq!(edid.serialize(), data, "{name} must be byte exact");
            assert!(edid.checksum_ok().iter().all(|ok| *ok), "{name} checksums");
        }
    }

    #[test]
    fn editing_a_timing_roundtrips() {
        let data = hex(SAM0D46);
        let mut e = Edid::parse(&data).unwrap();
        let slot = e.free_descriptor_slot().expect("free slot");
        e.base.descriptors[slot] = Descriptor::Detailed(timing::cvt_rb(1920, 1200, 60.0));
        let out = e.serialize();
        let reparsed = Edid::parse(&out).unwrap();
        let found = reparsed
            .detailed_timings()
            .into_iter()
            .find(|t| t.hactive == 1920 && t.vactive == 1200)
            .expect("new timing present");
        assert_eq!(found.htotal(), 2080);
        assert!(out.chunks(128).all(|b| b.iter().fold(0u8, |a, &x| a.wrapping_add(x)) == 0));
    }

    #[test]
    fn deleting_a_detailed_timing_works_and_roundtrips() {
        let data = hex(SAM0D46);
        let mut e = Edid::parse(&data).unwrap();
        let initial_count = e.detailed_timings().len();
        assert!(initial_count >= 1);
        let idx = e
            .base
            .descriptors
            .iter()
            .position(|d| d.is_removable())
            .expect("detailed timing must be removable");
        e.base.descriptors[idx] = Descriptor::Dummy;
        let reparsed = Edid::parse(&e.serialize()).unwrap();
        assert_eq!(reparsed.detailed_timings().len(), initial_count - 1);
    }
}
