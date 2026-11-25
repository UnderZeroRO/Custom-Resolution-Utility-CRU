//! Procedural Apple-style squircle app icon renderer and `.ico` encoder.
//!
//! Kept free of crate dependencies so both `build.rs` (for embedding the PE
//! `.exe` icon) and `src/main.rs` (for `ViewportBuilder::with_icon`) can share
//! the exact same artwork via `include!`.

use std::io::{self, Write};
use std::path::Path;

/// Renders a `size × size` straight-alpha RGBA8 icon matching the in-app logo:
/// Apple-blue vertical-gradient squircle with top specular rim, crisp white
/// monitor bezel, stand foot and glowing calibration wave.
pub fn render_rgba(size: u32) -> Vec<u8> {
    let mut out = vec![0u8; (size * size * 4) as usize];
    let sub = 4u32; // 4×4 supersampling for smooth edges at 16..128 px.
    let inv_total = 1.0 / (sub * sub) as f32;

    for py in 0..size {
        for px in 0..size {
            let mut r_sum = 0.0_f32;
            let mut g_sum = 0.0_f32;
            let mut b_sum = 0.0_f32;
            let mut a_sum = 0.0_f32;

            for sy in 0..sub {
                for sx in 0..sub {
                    let u = (px as f32 + (sx as f32 + 0.5) / sub as f32) / size as f32;
                    let v = (py as f32 + (sy as f32 + 0.5) / sub as f32) / size as f32;
                    let (r, g, b, a) = sample_icon(u, v);
                    // Premultiplied accumulation for proper alpha antialiasing.
                    r_sum += r * a;
                    g_sum += g * a;
                    b_sum += b * a;
                    a_sum += a;
                }
            }

            let a = (a_sum * inv_total).clamp(0.0, 1.0);
            let idx = ((py * size + px) * 4) as usize;
            if a > 1e-4 {
                let inv_a = 1.0 / (a * (sub * sub) as f32);
                out[idx] = (r_sum * inv_a * 255.0).round().clamp(0.0, 255.0) as u8;
                out[idx + 1] = (g_sum * inv_a * 255.0).round().clamp(0.0, 255.0) as u8;
                out[idx + 2] = (b_sum * inv_a * 255.0).round().clamp(0.0, 255.0) as u8;
                out[idx + 3] = (a * 255.0).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    out
}

fn sample_icon(u: f32, v: f32) -> (f32, f32, f32, f32) {
    // Normalized centered coordinates in [-1, 1], inset 7% so squircle breathes.
    let x = (u - 0.5) / 0.43;
    let y = (v - 0.5) / 0.43;

    // Superellipse squircle (n = 4.6) gives Apple's signature smooth corners.
    let n = 4.6_f32;
    let super_r = (x.abs().powf(n) + y.abs().powf(n)).powf(1.0 / n);
    if super_r > 1.02 {
        return (0.0, 0.0, 0.0, 0.0);
    }
    let bg_alpha = ((1.0 - super_r) * 32.0 + 0.5).clamp(0.0, 1.0);

    // Background Apple-blue gradient (#2A98FF top -> #005FD6 bottom) + subtle radial lift.
    let t = v.clamp(0.0, 1.0);
    let glow = (1.0 - ((u - 0.5).powi(2) + (v - 0.25).powi(2)).sqrt() * 1.4).max(0.0);
    let mut r = lerp(0x28 as f32 / 255.0, 0x00 as f32 / 255.0, t) + glow * 0.08;
    let mut g = lerp(0x96 as f32 / 255.0, 0x5E as f32 / 255.0, t) + glow * 0.06;
    let mut b = lerp(0xFF as f32 / 255.0, 0xD8 as f32 / 255.0, t) + glow * 0.03;

    // Crisp luminous top edge highlight inside the squircle.
    if super_r > 0.91 && v < 0.35 {
        let rim = ((super_r - 0.91) / 0.09).clamp(0.0, 1.0) * (1.0 - (v / 0.35)).powf(1.2);
        r = lerp(r, 1.0, rim * 0.35);
        g = lerp(g, 1.0, rim * 0.35);
        b = lerp(b, 1.0, rim * 0.35);
    }

    // Subtle dark inner border at the very outer edge for depth.
    if super_r > 0.95 {
        let edge = ((super_r - 0.95) / 0.05).clamp(0.0, 1.0);
        r *= 1.0 - edge * 0.20;
        g *= 1.0 - edge * 0.20;
        b *= 1.0 - edge * 0.20;
    }

    // Foreground: white display bezel, stand foot, and calibration wave.
    let stroke_r = 0.030_f32;

    // 1. Monitor bezel rounded rectangle: u in [0.22, 0.78], v in [0.25, 0.66], corner 0.06.
    let bezel_d = sd_rounded_box(u - 0.50, v - 0.455, 0.275, 0.205, 0.055);
    // Subtle dark OLED screen backdrop inside the bezel.
    if bezel_d < 0.0 {
        let fill_cov = (-bezel_d * 40.0).clamp(0.0, 1.0) * 0.22;
        r = lerp(r, 0.02, fill_cov);
        g = lerp(g, 0.05, fill_cov);
        b = lerp(b, 0.14, fill_cov);
    }
    let bezel_stroke = (1.0 - (bezel_d.abs() - stroke_r * 0.65) * 45.0).clamp(0.0, 1.0);

    // 2. Stand stem & base bar: stem at u=0.50, v in [0.66, 0.75]; base at v=0.75, u in [0.37, 0.63].
    let stem_d = sd_segment(u, v, (0.50, 0.66), (0.50, 0.75));
    let base_d = sd_segment(u, v, (0.37, 0.76), (0.63, 0.76));
    let stand_stroke = (1.0 - (stem_d.min(base_d) - stroke_r * 0.60) * 45.0).clamp(0.0, 1.0);

    // 3. Calibration timing wave inside screen: u in [0.29, 0.71].
    let wave_d = sd_wave(u, v);
    let wave_cov = (1.0 - (wave_d - stroke_r * 0.70) * 45.0).clamp(0.0, 1.0);

    let fg = bezel_stroke.max(stand_stroke).max(wave_cov);
    r = lerp(r, 0.98, fg);
    g = lerp(g, 0.99, fg);
    b = lerp(b, 1.00, fg);

    (r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0), bg_alpha)
}

fn sd_wave(u: f32, v: f32) -> f32 {
    // Smooth sine-like timing pulse across the center of the screen.
    let u0 = 0.295_f32;
    let u1 = 0.705_f32;
    let steps = 20;
    let mut best = f32::MAX;
    let mut prev = (u0, wave_y(0.0));
    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        let cur = (lerp(u0, u1, t), wave_y(t));
        best = best.min(sd_segment(u, v, prev, cur));
        prev = cur;
    }
    best
}

fn wave_y(t: f32) -> f32 {
    // Windowed full sine cycle centered at v = 0.455.
    let env = (t * std::f32::consts::PI).sin().powf(0.65);
    0.455 - 0.105 * env * (t * std::f32::consts::TAU).sin()
}

fn sd_rounded_box(px: f32, py: f32, bx: f32, by: f32, r: f32) -> f32 {
    let qx = px.abs() - bx + r;
    let qy = py.abs() - by + r;
    let out = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
    let inside = qx.max(qy).min(0.0);
    out + inside - r
}

fn sd_segment(px: f32, py: f32, a: (f32, f32), b: (f32, f32)) -> f32 {
    let pax = px - a.0;
    let pay = py - a.1;
    let bax = b.0 - a.0;
    let bay = b.1 - a.1;
    let denom = bax * bax + bay * bay;
    let h = if denom > 1e-8 {
        ((pax * bax + pay * bay) / denom).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let dx = pax - bax * h;
    let dy = pay - bay * h;
    (dx * dx + dy * dy).sqrt()
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Writes a multi-size Windows `.ico` (16, 32, 48, 64, 128 px, 32-bit BGRA DIB).
#[allow(dead_code)]
pub fn write_ico(path: &Path) -> io::Result<()> {
    let sizes: [u32; 5] = [16, 32, 48, 64, 128];
    let mut frames: Vec<(u32, Vec<u8>)> = Vec::with_capacity(sizes.len());

    for &size in &sizes {
        let rgba = render_rgba(size);
        let xor_bytes = size * size * 4;
        let and_row_bytes = size.div_ceil(32) * 4;
        let and_bytes = and_row_bytes * size;
        let mut dib = Vec::with_capacity((40 + xor_bytes + and_bytes) as usize);

        // BITMAPINFOHEADER (40 bytes)
        dib.extend_from_slice(&40u32.to_le_bytes());
        dib.extend_from_slice(&(size as i32).to_le_bytes());
        dib.extend_from_slice(&((size * 2) as i32).to_le_bytes());
        dib.extend_from_slice(&1u16.to_le_bytes());
        dib.extend_from_slice(&32u16.to_le_bytes());
        dib.extend_from_slice(&0u32.to_le_bytes());
        dib.extend_from_slice(&(xor_bytes + and_bytes).to_le_bytes());
        dib.extend_from_slice(&0u32.to_le_bytes());
        dib.extend_from_slice(&0u32.to_le_bytes());
        dib.extend_from_slice(&0u32.to_le_bytes());
        dib.extend_from_slice(&0u32.to_le_bytes());

        // Bottom-up BGRA pixels.
        for y in (0..size).rev() {
            for x in 0..size {
                let i = ((y * size + x) * 4) as usize;
                dib.push(rgba[i + 2]);
                dib.push(rgba[i + 1]);
                dib.push(rgba[i]);
                dib.push(rgba[i + 3]);
            }
        }

        // Bottom-up 1-bpp AND mask.
        for y in (0..size).rev() {
            let mut row = vec![0u8; and_row_bytes as usize];
            for x in 0..size {
                let i = ((y * size + x) * 4) as usize;
                if rgba[i + 3] < 128 {
                    row[(x / 8) as usize] |= 0x80 >> (x % 8);
                }
            }
            dib.extend_from_slice(&row);
        }

        frames.push((size, dib));
    }

    let mut file = std::fs::File::create(path)?;
    // ICONDIR
    file.write_all(&0u16.to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?;
    file.write_all(&(frames.len() as u16).to_le_bytes())?;

    let mut offset = 6u32 + (frames.len() as u32) * 16;
    for (size, dib) in &frames {
        let dim = if *size >= 256 { 0u8 } else { *size as u8 };
        file.write_all(&[dim, dim, 0, 0])?;
        file.write_all(&1u16.to_le_bytes())?;
        file.write_all(&32u16.to_le_bytes())?;
        file.write_all(&(dib.len() as u32).to_le_bytes())?;
        file.write_all(&offset.to_le_bytes())?;
        offset += dib.len() as u32;
    }

    for (_, dib) in &frames {
        file.write_all(dib)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_has_transparent_corners_and_opaque_center() {
        let rgba = render_rgba(64);
        assert_eq!(rgba.len(), 64 * 64 * 4);
        // Top-left corner (0,0) is outside the squircle -> transparent.
        assert_eq!(rgba[3], 0);
        // Center pixel (32,32) is opaque.
        let center_idx = (32 * 64 + 32) * 4;
        assert_eq!(rgba[center_idx + 3], 255);
    }

    #[test]
    fn ico_encodes_valid_header() {
        let dir = std::env::temp_dir();
        let path = dir.join("cru_modern_icon_test.ico");
        write_ico(&path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert!(bytes.len() > 1000);
        assert_eq!(&bytes[0..6], &[0, 0, 1, 0, 5, 0]);
    }
}
