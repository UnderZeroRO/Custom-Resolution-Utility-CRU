//! CTA-861 extension block decoding (read-only).

use super::timing::Timing;

#[derive(Clone, Debug)]
pub struct CtaBlock {
    pub tag: u8,
    pub name: String,
    pub detail: String,
}

#[derive(Clone, Debug)]
pub struct Cta {
    pub revision: u8,
    pub underscan: bool,
    pub basic_audio: bool,
    pub ycbcr444: bool,
    pub ycbcr422: bool,
    pub native_dtds: u8,
    pub data_blocks: Vec<CtaBlock>,
    pub dtds: Vec<Timing>,
}

impl Cta {
    pub fn parse(block: &[u8]) -> Option<Self> {
        if block.len() < 128 || block[0] != 0x02 {
            return None;
        }

        let revision = block[1];
        let dtd_offset = block[2] as usize;
        let flags = block[3];

        let mut data_blocks = Vec::new();
        let mut pos = 4usize;
        let end = if dtd_offset >= 4 { dtd_offset } else { 4 };

        while pos + 1 <= end && pos < 127 {
            let header = block[pos];
            let tag = header >> 5;
            let len = (header & 0x1F) as usize;
            if pos + 1 + len > 128 {
                break;
            }
            let payload = &block[pos + 1..pos + 1 + len];
            data_blocks.push(decode_block(tag, payload));
            pos += 1 + len;
        }

        let mut dtds = Vec::new();
        let mut dtd_pos = if dtd_offset >= 4 { dtd_offset } else { 0 };
        while dtd_pos + 18 <= 127 {
            let chunk = &block[dtd_pos..dtd_pos + 18];
            if chunk[0] == 0 && chunk[1] == 0 {
                break;
            }
            dtds.push(Timing::from_raw(chunk));
            dtd_pos += 18;
        }

        Some(Self {
            revision,
            underscan: flags & 0x80 != 0,
            basic_audio: flags & 0x40 != 0,
            ycbcr444: flags & 0x20 != 0,
            ycbcr422: flags & 0x10 != 0,
            native_dtds: flags & 0x0F,
            data_blocks,
            dtds,
        })
    }
}

fn decode_block(tag: u8, payload: &[u8]) -> CtaBlock {
    match tag {
        1 => CtaBlock {
            tag,
            name: "Audio".into(),
            detail: audio_detail(payload),
        },
        2 => CtaBlock {
            tag,
            name: "Video".into(),
            detail: video_detail(payload),
        },
        3 => CtaBlock {
            tag,
            name: "Vendor specific".into(),
            detail: vendor_detail(payload),
        },
        4 => CtaBlock {
            tag,
            name: "Speaker allocation".into(),
            detail: format!("{} byte(s)", payload.len()),
        },
        5 => CtaBlock {
            tag,
            name: "VESA DTC".into(),
            detail: format!("{} byte(s)", payload.len()),
        },
        7 => {
            let ext = payload.first().copied().unwrap_or(0);
            CtaBlock {
                tag,
                name: extended_name(ext).into(),
                detail: extended_detail(ext, payload),
            }
        }
        other => CtaBlock {
            tag: other,
            name: format!("Block {other}"),
            detail: format!("{} byte(s)", payload.len()),
        },
    }
}

fn audio_detail(payload: &[u8]) -> String {
    let mut parts = Vec::new();
    for sad in payload.chunks(3) {
        if sad.len() < 3 {
            break;
        }
        let format = (sad[0] >> 3) & 0x0F;
        let channels = (sad[0] & 0x07) + 1;
        let rates = sad[1];
        let mut rate_list = Vec::new();
        for (bit, label) in [
            (0x40, "192k"),
            (0x20, "176k"),
            (0x10, "96k"),
            (0x08, "88k"),
            (0x04, "48k"),
            (0x02, "44k"),
            (0x01, "32k"),
        ] {
            if rates & bit != 0 {
                rate_list.push(label);
            }
        }
        let name = match format {
            1 => "LPCM",
            2 => "AC-3",
            3 => "MPEG-1",
            4 => "MP3",
            5 => "MPEG-2",
            6 => "AAC",
            7 => "DTS",
            8 => "ATRAC",
            9 => "DSD",
            10 => "E-AC-3",
            11 => "DTS-HD",
            12 => "MAT",
            13 => "DST",
            14 => "WMA Pro",
            _ => "Reserved",
        };
        let extra = if format == 1 {
            let depth = sad[2];
            let mut depths = Vec::new();
            if depth & 0x04 != 0 {
                depths.push("24-bit");
            }
            if depth & 0x02 != 0 {
                depths.push("20-bit");
            }
            if depth & 0x01 != 0 {
                depths.push("16-bit");
            }
            format!(" {}", depths.join("/"))
        } else {
            String::new()
        };
        parts.push(format!(
            "{name} {channels}ch [{}]{extra}",
            rate_list.join(" ")
        ));
    }
    if parts.is_empty() {
        "No audio descriptors".into()
    } else {
        parts.join(", ")
    }
}

fn video_detail(payload: &[u8]) -> String {
    let mut parts = Vec::new();
    for &svd in payload {
        let vic = svd & 0x7F;
        let native = if svd & 0x80 != 0 { " (native)" } else { "" };
        let name = vic_name(vic);
        parts.push(format!("{vic}{name}{native}"));
    }
    if parts.is_empty() {
        "No video descriptors".into()
    } else {
        parts.join(", ")
    }
}

fn vendor_detail(payload: &[u8]) -> String {
    if payload.len() >= 3 {
        let oui = (payload[0] as u32) | (payload[1] as u32) << 8 | (payload[2] as u32) << 16;
        if oui == 0x000C03 {
            let mut text = "HDMI Licensing".to_string();
            if payload.len() >= 5 {
                let a = payload[3] as u32;
                let b = payload[4] as u32;
                text.push_str(&format!(
                    ", phys addr {}.{}.{}.{}",
                    (a >> 4) & 0x0F,
                    a & 0x0F,
                    (b >> 4) & 0x0F,
                    b & 0x0F
                ));
            }
            return text;
        }
        return format!("OUI {:06X}", oui);
    }
    "—".into()
}

fn extended_name(ext: u8) -> &'static str {
    match ext {
        0x00 => "Video capability",
        0x01 => "Vendor specific video",
        0x02 => "VESA video",
        0x03 => "HDMI video",
        0x04 => "HDMI audio",
        0x05 => "Colorimetry",
        0x06 => "HDR static metadata",
        0x07 => "HDR dynamic metadata",
        0x0E => "YCbCr 4:2:0 video",
        0x0F => "YCbCr 4:2:0 capability map",
        _ => "Extended",
    }
}

fn extended_detail(ext: u8, payload: &[u8]) -> String {
    match ext {
        0x00 => {
            let flags = payload.get(1).copied().unwrap_or(0);
            let mut parts = Vec::new();
            if flags & 0x80 != 0 {
                parts.push("underscan");
            }
            if flags & 0x40 != 0 {
                parts.push("basic audio");
            }
            if flags & 0x20 != 0 {
                parts.push("YCbCr 4:4:4");
            }
            if flags & 0x10 != 0 {
                parts.push("YCbCr 4:2:2");
            }
            if parts.is_empty() {
                "no flags".into()
            } else {
                parts.join(", ")
            }
        }
        0x05 => {
            let flags = payload.get(1).copied().unwrap_or(0);
            let mut parts = Vec::new();
            if flags & 0x02 != 0 {
                parts.push("xvYCC 601");
            }
            if flags & 0x04 != 0 {
                parts.push("xvYCC 709");
            }
            if flags & 0x08 != 0 {
                parts.push("sYCC 601");
            }
            if flags & 0x10 != 0 {
                parts.push("Adobe YCC 601");
            }
            if flags & 0x20 != 0 {
                parts.push("Adobe RGB");
            }
            if flags & 0x40 != 0 {
                parts.push("BT.2020 cYCC");
            }
            if flags & 0x80 != 0 {
                parts.push("BT.2020 YCC");
            }
            if parts.is_empty() {
                "no colorimetry flags".into()
            } else {
                parts.join(", ")
            }
        }
        0x06 => {
            let flags = payload.get(1).copied().unwrap_or(0);
            let mut parts = Vec::new();
            if flags & 0x01 != 0 {
                parts.push("SMPTE 2084 (HDR10)");
            }
            if flags & 0x02 != 0 {
                parts.push("HLG");
            }
            if flags & 0x04 != 0 {
                parts.push("ST 2086 mastering");
            }
            if parts.is_empty() {
                "no HDR flags".into()
            } else {
                parts.join(", ")
            }
        }
        0x0E => {
            let mut parts = Vec::new();
            for &v in payload.iter().skip(1) {
                let vic = v & 0x7F;
                let name = vic_name(vic);
                parts.push(format!("{vic}{name}"));
            }
            if parts.is_empty() {
                "no additional modes".into()
            } else {
                parts.join(", ")
            }
        }
        _ => format!("{} byte(s)", payload.len()),
    }
}

fn vic_name(vic: u8) -> &'static str {
    match vic {
        1 => " 640x480p60",
        2 | 3 => " 720x480p60",
        4 => " 1280x720p60",
        5 => " 1920x1080i60",
        6 | 7 => " 720x480i60",
        16 => " 1920x1080p60",
        17 | 18 => " 720x576p50",
        19 => " 1280x720p50",
        20 => " 1920x1080i50",
        31 => " 1920x1080p50",
        32 => " 1920x1080p24",
        33 => " 1920x1080p25",
        34 => " 1920x1080p30",
        60 | 61 | 62 | 63 => " 3840x2160p",
        93 | 94 | 95 | 96 | 97 | 98 => " 3840x2160p",
        _ => "",
    }
}
