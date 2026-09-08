//! Choose the decoder from encoded bytes, not an untrusted filename or MIME.

use gpui::{Image, ImageFormat};
use std::sync::Arc;

/// Raster signatures are authoritative. SVG has no fixed binary signature,
/// so its declared format remains the fallback, as for unknown/corrupt data.
pub(super) fn encoded_format(bytes: &[u8], fallback: ImageFormat) -> ImageFormat {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        ImageFormat::Png
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        ImageFormat::Jpeg
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        ImageFormat::Gif
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        ImageFormat::Webp
    } else if bytes.starts_with(b"BM") {
        ImageFormat::Bmp
    } else if bytes.starts_with(b"II\x2a\0") || bytes.starts_with(b"MM\0\x2a") {
        ImageFormat::Tiff
    } else {
        fallback
    }
}

pub(super) fn image_from_bytes(fallback: ImageFormat, bytes: Vec<u8>) -> Arc<Image> {
    Arc::new(Image::from_bytes(encoded_format(&bytes, fallback), bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    const JPEG: &[u8] = include_bytes!("fixtures/two-colors.jpg");

    fn assert_decodes(image: &Image) {
        let decoded = image
            .to_image_data(gpui::SvgRenderer::new(Arc::new(())))
            .unwrap();
        assert_eq!(decoded.frame_count(), 1);
        assert_eq!(decoded.size(0).width.0, 4);
        assert_eq!(decoded.size(0).height.0, 2);
    }

    #[test]
    fn jpeg_named_png_stages_with_a_working_gpui_preview() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("screenshot.png");
        std::fs::write(&path, JPEG).unwrap();
        let staged = crate::attachments::stage_file(&path).unwrap();
        let image = staged.image().expect("a JPEG stages as an image, not text");
        assert_eq!(image.format, ImageFormat::Jpeg);
        assert_eq!(staged.bytes(), JPEG);
        assert_decodes(image);
    }

    #[test]
    fn readback_with_wrong_png_mime_uses_the_same_decoder() {
        let image = image_from_bytes(ImageFormat::Png, JPEG.to_vec());
        assert_eq!(image.format, ImageFormat::Jpeg);
        assert_decodes(&image);
    }

    #[test]
    fn known_raster_signatures_override_declarations_and_svg_keeps_its_fallback() {
        for (bytes, format) in [
            (&b"\x89PNG\r\n\x1a\n"[..], ImageFormat::Png),
            (&b"\xff\xd8\xff"[..], ImageFormat::Jpeg),
            (&b"GIF89a"[..], ImageFormat::Gif),
            (&b"RIFF\0\0\0\0WEBP"[..], ImageFormat::Webp),
            (&b"BM"[..], ImageFormat::Bmp),
            (&b"II\x2a\0"[..], ImageFormat::Tiff),
            (&b"MM\0\x2a"[..], ImageFormat::Tiff),
        ] {
            assert_eq!(encoded_format(bytes, ImageFormat::Svg), format);
        }
        assert_eq!(
            encoded_format(b"<svg></svg>", ImageFormat::Svg),
            ImageFormat::Svg
        );
    }
}
