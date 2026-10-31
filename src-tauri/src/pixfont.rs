//! Tiny 3x5 pixel font used to draw a number into the tray icon.

fn glyph(c: char) -> Option<[u8; 5]> {
    Some(match c {
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b111, 0b001, 0b111, 0b100, 0b111],
        '3' => [0b111, 0b001, 0b011, 0b001, 0b111],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b111, 0b001, 0b111],
        '6' => [0b111, 0b100, 0b111, 0b101, 0b111],
        '7' => [0b111, 0b001, 0b010, 0b010, 0b010],
        '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
        '9' => [0b111, 0b101, 0b111, 0b001, 0b111],
        'K' => [0b101, 0b110, 0b100, 0b110, 0b101],
        'M' => [0b101, 0b111, 0b111, 0b101, 0b101],
        'B' => [0b110, 0b101, 0b110, 0b101, 0b110],
        '-' => [0b000, 0b000, 0b111, 0b000, 0b000],
        '.' => [0b000, 0b000, 0b000, 0b000, 0b010],
        _ => return None,
    })
}

fn width(c: char) -> u32 {
    if c == '.' { 1 } else { 3 }
}

/// Renders up to 4 characters, centred, into a `size`×`size` RGBA image.
pub fn render(text: &str, size: u32, rgba: [u8; 4], bg: [u8; 4]) -> Vec<u8> {
    let chars: Vec<char> = text.chars().filter(|c| glyph(*c).is_some()).take(4).collect();
    let mut img = vec![0u8; (size * size * 4) as usize];
    let radius = size as f32 * 0.22;
    for y in 0..size {
        for x in 0..size {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let dx = (radius - fx).max(fx - (size as f32 - radius)).max(0.0);
            let dy = (radius - fy).max(fy - (size as f32 - radius)).max(0.0);
            if dx * dx + dy * dy <= radius * radius {
                let i = ((y * size + x) * 4) as usize;
                img[i..i + 4].copy_from_slice(&bg);
            }
        }
    }
    if chars.is_empty() {
        return img;
    }
    let raw_w: u32 = chars.iter().map(|c| width(*c)).sum::<u32>() + chars.len() as u32 - 1;
    let scale = ((size - 4) / raw_w).min((size - 4) / 5).max(1);
    let (w, h) = (raw_w * scale, 5 * scale);
    let (ox, oy) = ((size.saturating_sub(w)) / 2, (size.saturating_sub(h)) / 2);
    let mut cx = 0;
    for c in chars {
        let g = glyph(c).unwrap();
        let cw = width(c);
        for (row, bits) in g.iter().enumerate() {
            for col in 0..cw {
                let bit = if cw == 1 { (bits >> 1) & 1 } else { (bits >> (2 - col)) & 1 };
                if bit == 1 {
                    for sy in 0..scale {
                        for sx in 0..scale {
                            let x = ox + (cx + col) * scale + sx;
                            let y = oy + row as u32 * scale + sy;
                            if x < size && y < size {
                                let i = ((y * size + x) * 4) as usize;
                                img[i..i + 4].copy_from_slice(&rgba);
                            }
                        }
                    }
                }
            }
        }
        cx += cw + 1;
    }
    img
}

/// Short form for tiny spaces: 37, 999, 1.2K, 48K, 1.3M.
pub fn compact(v: f64) -> String {
    let a = v.abs();
    let sign = if v < 0.0 { "-" } else { "" };
    let s = if a < 1000.0 {
        format!("{}", a.round() as i64)
    } else {
        let (n, u) = if a < 1e6 { (a / 1e3, "K") } else if a < 1e9 { (a / 1e6, "M") } else { (a / 1e9, "B") };
        if n < 10.0 {
            let t = format!("{:.1}", (n * 10.0).floor() / 10.0);
            format!("{}{u}", t.trim_end_matches(".0"))
        } else {
            format!("{}{u}", n.floor() as i64)
        }
    };
    format!("{sign}{s}")
}

/// Full number with thousands separators for the tray label; very large values fall back to `compact`.
pub fn full(v: f64) -> String {
    if !v.is_finite() || v.abs() >= 1e7 {
        return compact(v);
    }
    let neg = v < 0.0;
    let a = v.abs();
    let int = a.trunc() as i64;
    let frac = ((a - a.trunc()) * 100.0).round() as i64;
    let (int, frac) = if frac == 100 { (int + 1, 0) } else { (int, frac) };
    let digits = int.to_string();
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    if frac > 0 {
        out.push_str(&format!(".{frac:02}"));
    }
    format!("{}{out}", if neg { "-" } else { "" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_numbers() {
        assert_eq!(full(1754.0), "1,754");
        assert_eq!(full(-1284.5), "-1,284.50");
        assert_eq!(full(12.0), "12");
        assert_eq!(full(999_999.999), "1,000,000");
        assert_eq!(full(2e9), "2B");
    }

    #[test]
    fn compact_numbers() {
        assert_eq!(compact(37.0), "37");
        assert_eq!(compact(999.4), "999");
        assert_eq!(compact(1284.0), "1.2K");
        assert_eq!(compact(48213.0), "48K");
        assert_eq!(compact(1_300_000.0), "1.3M");
        assert_eq!(compact(2_000.0), "2K");
        assert_eq!(compact(-12.0), "-12");
        assert_eq!(compact(9_990.0), "9.9K");
    }

    #[test]
    fn renders_pixels() {
        let img = render("48K", 32, [255, 255, 255, 255], [0, 0, 0, 255]);
        assert_eq!(img.len(), 32 * 32 * 4);
        let white = img.chunks(4).filter(|p| p == &[255, 255, 255, 255]).count();
        assert!(white > 20);
    }
}
