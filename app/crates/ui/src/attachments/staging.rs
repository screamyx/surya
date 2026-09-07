use super::MAX_ATTACHMENT_BYTES;
use gpui::{Image, ImageFormat};
use std::{path::Path, sync::Arc};

/// A local file staged for upload, with an image preview only when applicable.
#[derive(Clone)]
pub struct StagedAttachment {
    pub id: String,
    pub name: String,
    data: StagedData,
}

#[derive(Clone)]
enum StagedData {
    Image(Arc<Image>),
    Text(Arc<Vec<u8>>),
}

impl StagedAttachment {
    pub fn bytes(&self) -> &[u8] {
        match &self.data {
            StagedData::Image(image) => &image.bytes,
            StagedData::Text(bytes) => bytes,
        }
    }

    pub fn image(&self) -> Option<&Arc<Image>> {
        match &self.data {
            StagedData::Image(image) => Some(image),
            StagedData::Text(_) => None,
        }
    }
}

/// Image formats the whole pipeline supports: intersection of gpui's decoders
/// and the engine's `mime_by_ext` read-back jail.
///
/// One table, because two things read it: the check that decides whether a
/// picked file is staged, and the notice that tells the user what he may
/// attach. Written as a second literal, those two drift and the message starts
/// lying about what the code takes.
const BY_EXTENSION: &[(&str, ImageFormat)] = &[
    ("png", ImageFormat::Png),
    ("jpg", ImageFormat::Jpeg),
    ("jpeg", ImageFormat::Jpeg),
    ("gif", ImageFormat::Gif),
    ("webp", ImageFormat::Webp),
    ("svg", ImageFormat::Svg),
    ("bmp", ImageFormat::Bmp),
    ("tif", ImageFormat::Tiff),
    ("tiff", ImageFormat::Tiff),
];

pub fn format_by_extension(path: &Path) -> Option<ImageFormat> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    BY_EXTENSION
        .iter()
        .find(|(candidate, _)| *candidate == ext)
        .map(|(_, format)| *format)
}

/// The same extensions, spelled out for a message that has to name them.
///
/// Every accepted spelling is listed, `jpeg` beside `jpg` and `tiff` beside
/// `tif`: someone holding a `.jpeg` should not have to guess whether the short
/// form in a notice includes his file.
pub fn supported_extensions() -> String {
    BY_EXTENSION
        .iter()
        .map(|(ext, _)| *ext)
        .collect::<Vec<_>>()
        .join(", ")
}

/// use-attachments.ts `ensureExtension`: pasted screenshots often arrive as a
/// bare "image" — make sure the staged name carries a type-matching extension.
pub fn ensure_extension(name: &str, format: ImageFormat) -> String {
    let has_ext = name
        .rsplit_once('.')
        .map(|(stem, ext)| {
            !stem.is_empty()
                && (2..=5).contains(&ext.len())
                && ext.chars().all(|c| c.is_ascii_alphanumeric())
        })
        .unwrap_or(false);
    if has_ext {
        name.to_string()
    } else {
        format!("{name}.{}", format.extension())
    }
}

/// Stage a file from disk (picker / drop / pasted path). `Err` carries the
/// user-facing message (mirrors the old `onError` copy).
pub fn stage_file(path: &Path) -> Result<StagedAttachment, String> {
    use std::io::Read;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".into());
    let file = std::fs::File::open(path).map_err(|_| format!("{name} could not be read."))?;
    let meta = file
        .metadata()
        .map_err(|_| format!("{name} could not be read."))?;
    if !meta.is_file() {
        return Err(format!("{name} is not a file."));
    }
    if meta.len() > MAX_ATTACHMENT_BYTES {
        return Err(format!("{name} is too large (24 MB max)."));
    }
    // Bound the read too: a file may grow after the metadata check.
    let mut bytes = Vec::new();
    file.take(MAX_ATTACHMENT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| format!("{name} could not be read."))?;
    if bytes.len() as u64 > MAX_ATTACHMENT_BYTES {
        return Err(format!("{name} is too large (24 MB max)."));
    }
    let (name, data) = if let Some(format) = format_by_extension(path) {
        (
            ensure_extension(&name, format),
            StagedData::Image(Arc::new(Image::from_bytes(format, bytes))),
        )
    } else {
        if bytes.contains(&0) || std::str::from_utf8(&bytes).is_err() {
            return Err(format!(
                "{name} is not a supported image or UTF-8 text file."
            ));
        }
        (name, StagedData::Text(Arc::new(bytes)))
    };
    Ok(StagedAttachment {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        data,
    })
}

/// Stage an image pasted from the clipboard.
pub fn stage_clipboard_image(image: Image) -> StagedAttachment {
    let format = image.format;
    StagedAttachment {
        id: uuid::Uuid::new_v4().to_string(),
        name: ensure_extension("image", format),
        data: StagedData::Image(Arc::new(image)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_file_stages_original_name_and_bytes_without_an_image() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.txt");
        let bytes = "release code: maple-42\nUnicode: café\n".as_bytes();
        std::fs::write(&path, bytes).unwrap();
        let file = stage_file(&path).unwrap();
        assert_eq!(file.name, "notes.txt");
        assert_eq!(file.bytes(), bytes);
        assert!(file.image().is_none());
    }

    #[test]
    fn extensionless_and_empty_text_files_are_supported() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["README", "empty.txt"] {
            let path = dir.path().join(name);
            std::fs::write(&path, b"").unwrap();
            assert_eq!(stage_file(&path).unwrap().name, name);
        }
    }

    #[test]
    fn binary_and_oversized_text_files_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("binary.txt");
        for bytes in [vec![0xff], vec![b'a', 0, b'b']] {
            std::fs::write(&path, bytes).unwrap();
            assert!(stage_file(&path).is_err());
        }
        let file = std::fs::File::create(&path).unwrap();
        file.set_len(MAX_ATTACHMENT_BYTES + 1).unwrap();
        assert!(stage_file(&path).err().unwrap().contains("too large"));
    }

    #[test]
    fn images_keep_their_preview_and_upload_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("image.png");
        let bytes = b"\x89PNG\r\n\x1a\n";
        std::fs::write(&path, bytes).unwrap();
        let file = stage_file(&path).unwrap();
        assert_eq!(file.name, "image.png");
        assert_eq!(file.bytes(), bytes);
        assert_eq!(file.image().unwrap().format, ImageFormat::Png);
    }
}
