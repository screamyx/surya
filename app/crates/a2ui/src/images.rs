//! Where a card's Image may load from. A card is model-authored, and gpui's
//! `img()` fetches a URI on render with no click, so an unfiltered URL is an
//! exfiltration channel (query string) and `file://` reads any local path.
//! v1 policy: `data:` URIs (capped) and files under the workspace or fixture
//! dir only; remote images stay a placeholder unless the host allows them.

use std::path::{Path, PathBuf};

use base64::Engine as _;
use gpui::ImageFormat;

/// Largest inline `data:` image accepted.
pub const MAX_DATA_URI_BYTES: usize = 2 * 1024 * 1024;
/// Largest local file handed to gpui's decoder.
pub const MAX_FILE_BYTES: u64 = 20 * 1024 * 1024;

#[derive(Debug, Clone, Default)]
pub struct ImagePolicy {
    /// Directories a `file://` (or absolute path) image may live under.
    /// Empty means no local files at all.
    pub allowed_dirs: Vec<PathBuf>,
    /// `http(s)` images fetch on render only when the host says so (a
    /// setting, default off). Otherwise they render as a placeholder that
    /// names the host.
    pub remote_allowed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImageDecision {
    /// Load this local file (already checked against the policy).
    File(PathBuf),
    /// Inline bytes from a `data:` URI.
    Data { format: ImageFormat, bytes: Vec<u8> },
    /// Let gpui fetch this remote URL (policy allowed it).
    Remote(String),
    /// Draw a placeholder naming the remote host instead of fetching.
    Placeholder(String),
    /// Draw the fallback box with this reason.
    Denied(String),
}

impl ImagePolicy {
    pub fn decide(&self, url: &str) -> ImageDecision {
        let url = url.trim();
        if url.is_empty() {
            return ImageDecision::Denied("Image without a url".into());
        }
        if let Some(rest) = url.strip_prefix("data:") {
            return decode_data_uri(rest);
        }
        if let Some(rest) = url.strip_prefix("file://") {
            return self.local(Path::new(rest));
        }
        if url.starts_with('/') {
            return self.local(Path::new(url));
        }
        if url.starts_with("https://") || url.starts_with("http://") {
            let host = url
                .split("://")
                .nth(1)
                .and_then(|r| r.split(['/', '?', '#']).next())
                .unwrap_or("remote");
            return if self.remote_allowed {
                ImageDecision::Remote(url.to_owned())
            } else {
                ImageDecision::Placeholder(host.to_owned())
            };
        }
        match url.split_once(':') {
            Some((scheme, _)) if !scheme.is_empty() => {
                ImageDecision::Denied(format!("Image scheme \"{scheme}:\" is not allowed"))
            }
            _ => ImageDecision::Denied("Image path must be absolute or file://".into()),
        }
    }

    fn local(&self, path: &Path) -> ImageDecision {
        // Canonicalize both sides so `..` and symlinks cannot escape the dir.
        let Ok(real) = path.canonicalize() else {
            return ImageDecision::Denied(format!("Image not found: {}", path.display()));
        };
        let allowed = self.allowed_dirs.iter().any(|dir| {
            dir.canonicalize()
                .map(|d| real.starts_with(&d))
                .unwrap_or(false)
        });
        if !allowed {
            return ImageDecision::Denied(format!(
                "Image outside the workspace: {}",
                path.display()
            ));
        }
        match std::fs::metadata(&real) {
            Ok(meta) if meta.is_file() && meta.len() <= MAX_FILE_BYTES => ImageDecision::File(real),
            Ok(meta) if !meta.is_file() => ImageDecision::Denied("Image path is not a file".into()),
            Ok(meta) => ImageDecision::Denied(format!(
                "Image too large ({} MB, cap {} MB)",
                meta.len() / (1024 * 1024),
                MAX_FILE_BYTES / (1024 * 1024)
            )),
            Err(e) => ImageDecision::Denied(format!("Image unreadable: {e}")),
        }
    }
}

fn decode_data_uri(rest: &str) -> ImageDecision {
    let Some((header, payload)) = rest.split_once(',') else {
        return ImageDecision::Denied("Malformed data: image".into());
    };
    let mut parts = header.split(';');
    let mime = parts.next().unwrap_or("");
    let is_base64 = parts.any(|p| p == "base64");
    let format = match mime {
        "image/png" => ImageFormat::Png,
        "image/jpeg" | "image/jpg" => ImageFormat::Jpeg,
        "image/webp" => ImageFormat::Webp,
        "image/gif" => ImageFormat::Gif,
        "image/svg+xml" => ImageFormat::Svg,
        "image/bmp" => ImageFormat::Bmp,
        other => {
            return ImageDecision::Denied(format!("data: image type \"{other}\" is not allowed"));
        }
    };
    // Base64 inflates 4:3, so the cap on the encoded text bounds the decode.
    if payload.len() > MAX_DATA_URI_BYTES * 4 / 3 + 4 {
        return ImageDecision::Denied(format!(
            "data: image too large (cap {} KB)",
            MAX_DATA_URI_BYTES / 1024
        ));
    }
    let bytes = if is_base64 {
        match base64::engine::general_purpose::STANDARD.decode(payload.trim()) {
            Ok(b) => b,
            Err(_) => return ImageDecision::Denied("data: image is not valid base64".into()),
        }
    } else {
        payload.as_bytes().to_vec()
    };
    if bytes.is_empty() {
        return ImageDecision::Denied("data: image is empty".into());
    }
    if bytes.len() > MAX_DATA_URI_BYTES {
        return ImageDecision::Denied("data: image too large".into());
    }
    ImageDecision::Data { format, bytes }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sandbox(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("surya-a2ui-images-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("inner")).unwrap();
        std::fs::write(dir.join("inner/ok.png"), b"\x89PNG\r\n\x1a\nfake").unwrap();
        dir
    }

    #[test]
    fn local_files_only_under_allowed_dirs() {
        let dir = sandbox("local");
        let policy = ImagePolicy {
            allowed_dirs: vec![dir.join("inner")],
            remote_allowed: false,
        };
        let ok = dir.join("inner/ok.png");
        assert!(matches!(policy.decide(&format!("file://{}", ok.display())), ImageDecision::File(p) if p == ok.canonicalize().unwrap()));
        assert!(matches!(policy.decide(ok.to_str().unwrap()), ImageDecision::File(_)));
        // Escapes and outsiders are denied even when the file exists.
        let escape = format!("file://{}/inner/../inner/../../{}", dir.display(), "etc/hostname");
        assert!(matches!(policy.decide(&escape), ImageDecision::Denied(_)));
        assert!(matches!(policy.decide("file:///etc/passwd"), ImageDecision::Denied(r) if r.contains("outside")));
        assert!(matches!(policy.decide("file:///nope/missing.png"), ImageDecision::Denied(r) if r.contains("not found")));
        assert!(matches!(policy.decide("inner/ok.png"), ImageDecision::Denied(_)));
        // No allowed dirs: nothing local loads.
        let none = ImagePolicy::default();
        assert!(matches!(none.decide(ok.to_str().unwrap()), ImageDecision::Denied(_)));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn remote_is_a_placeholder_unless_allowed() {
        let policy = ImagePolicy::default();
        assert_eq!(
            policy.decide("https://evil.example/x.png?secret=token"),
            ImageDecision::Placeholder("evil.example".into())
        );
        let open = ImagePolicy { remote_allowed: true, ..Default::default() };
        assert!(matches!(open.decide("http://cdn.example/a.png"), ImageDecision::Remote(_)));
        assert!(matches!(policy.decide("ftp://x/y.png"), ImageDecision::Denied(r) if r.contains("ftp:")));
        assert!(matches!(policy.decide("javascript:alert(1)"), ImageDecision::Denied(_)));
        assert!(matches!(policy.decide(""), ImageDecision::Denied(_)));
    }

    #[test]
    fn data_uris_decode_with_caps() {
        let policy = ImagePolicy::default();
        let png = base64::engine::general_purpose::STANDARD.encode(b"\x89PNG\r\n\x1a\nfake");
        assert!(matches!(
            policy.decide(&format!("data:image/png;base64,{png}")),
            ImageDecision::Data { format: ImageFormat::Png, bytes } if bytes.starts_with(b"\x89PNG")
        ));
        assert!(matches!(policy.decide("data:text/html;base64,PGI+"), ImageDecision::Denied(_)));
        assert!(matches!(policy.decide("data:image/png;base64,***"), ImageDecision::Denied(_)));
        assert!(matches!(policy.decide("data:image/png;base64,"), ImageDecision::Denied(_)));
        let huge = "A".repeat(MAX_DATA_URI_BYTES * 4 / 3 + 64);
        assert!(matches!(policy.decide(&format!("data:image/png;base64,{huge}")), ImageDecision::Denied(r) if r.contains("too large")));
    }
}
