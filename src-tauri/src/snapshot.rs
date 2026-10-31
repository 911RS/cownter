use base64::Engine;
use std::path::{Path, PathBuf};

const PNG_MAGIC: &[u8] = b"\x89PNG\r\n\x1a\n";
const MAX_BYTES: usize = 16 * 1024 * 1024;

/// Lowercase ASCII words joined by dashes, at most 40 characters.
fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars().flat_map(|c| c.to_lowercase()) {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
        if out.len() >= 40 {
            break;
        }
    }
    let out = out.trim_end_matches('-');
    if out.is_empty() { "number".into() } else { out.into() }
}

/// `cownter-<name>-<date>.<ext>`, with `-<n>` before the extension when `n > 1`.
pub fn file_name(name: &str, date: &str, n: u32, ext: &str) -> String {
    let base = format!("cownter-{}-{date}", slug(name));
    if n > 1 { format!("{base}-{n}.{ext}") } else { format!("{base}.{ext}") }
}

/// Decodes a base64 PNG, with or without a `data:` prefix.
pub fn decode(png: &str) -> Result<Vec<u8>, String> {
    let b64 = png.split_once("base64,").map(|(_, b)| b).unwrap_or(png);
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64.trim()).map_err(|_| "the image could not be read".to_string())?;
    if !bytes.starts_with(PNG_MAGIC) {
        return Err("the image is not a PNG".into());
    }
    if bytes.len() > MAX_BYTES {
        return Err("the image is too large".into());
    }
    Ok(bytes)
}

/// Writes the file into `dir` under a name that does not exist yet and returns its path.
pub fn write(dir: &Path, name: &str, date: &str, ext: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    for n in 1..1000 {
        let path = dir.join(file_name(name, date, n, ext));
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut f) => {
                use std::io::Write;
                f.write_all(bytes).map_err(|e| format!("could not write {}: {e}", path.display()))?;
                return Ok(path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("could not write {}: {e}", path.display())),
        }
    }
    Err("too many files with this name today".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_safe() {
        assert_eq!(file_name("GitHub stars", "2026-10-31", 1, "png"), "cownter-github-stars-2026-10-31.png");
        assert_eq!(file_name("../../etc/passwd", "2026-10-31", 1, "png"), "cownter-etc-passwd-2026-10-31.png");
        assert_eq!(file_name("🐄", "2026-10-31", 3, "png"), "cownter-number-2026-10-31-3.png");
        assert_eq!(file_name(&"x".repeat(90), "d", 1, "png").len(), "cownter--d.png".len() + 40);
        assert_eq!(file_name("all", "2026-10-31", 2, "csv"), "cownter-all-2026-10-31-2.csv");
    }

    #[test]
    fn decodes_png_only() {
        let png = base64::engine::general_purpose::STANDARD.encode(b"\x89PNG\r\n\x1a\nrest");
        assert_eq!(decode(&format!("data:image/png;base64,{png}")).unwrap().len(), 12);
        assert!(decode(&png).is_ok());
        assert!(decode("bm90IGEgcG5n").is_err());
        assert!(decode("%%%").is_err());
    }

    #[test]
    fn never_overwrites() {
        let dir = std::env::temp_dir().join(format!("cownter-snap-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let a = write(&dir, "Stars", "2026-10-31", "png", b"one").unwrap();
        let b = write(&dir, "Stars", "2026-10-31", "png", b"two").unwrap();
        assert_eq!(a.file_name().unwrap(), "cownter-stars-2026-10-31.png");
        assert_eq!(b.file_name().unwrap(), "cownter-stars-2026-10-31-2.png");
        assert_eq!(std::fs::read(&a).unwrap(), b"one");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
