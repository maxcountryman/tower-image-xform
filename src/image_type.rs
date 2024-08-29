//! Image types as constants which can be used to establish a slice of supported
//! image types and their respective image formats.
use image::ImageFormat;
use mediatype::{names, MediaType};

const IMAGE_WEBP: MediaType = image_type(names::WEBP);
const IMAGE_AVIF: MediaType = image_type(names::AVIF);
const IMAGE_PNG: MediaType = image_type(names::PNG);
const IMAGE_JPEG: MediaType = image_type(names::JPEG);

const fn image_type(subtype: mediatype::Name) -> MediaType {
    MediaType::new(names::IMAGE, subtype)
}

/// WebP image type.
pub const WEBP: SupportedImageType = SupportedImageType::new(IMAGE_WEBP, ImageFormat::WebP);
/// AVIF image type.
pub const AVIF: SupportedImageType = SupportedImageType::new(IMAGE_AVIF, ImageFormat::Avif);
/// PNG image type.
pub const PNG: SupportedImageType = SupportedImageType::new(IMAGE_PNG, ImageFormat::Png);
/// JPEG image type.
pub const JPEG: SupportedImageType = SupportedImageType::new(IMAGE_JPEG, ImageFormat::Jpeg);

/// Alias for a static slice of [`SupportedImageType`].
pub type SupportedImageTypes = &'static [SupportedImageType<'static>];

/// Default of supported image types, consisting of [`WEBP`] and [`PNG`].
pub const DEFAULT_SUPPORTED_IMAGE_TYPES: SupportedImageTypes = &[WEBP, PNG];

/// Pair of [`MediaType`] and [`ImageFormat`].
///
/// This structure establishes an association between the two types and is
/// useful for conversions.
#[derive(Debug)]
pub struct SupportedImageType<'a> {
    /// Media type, such as "image/png".
    pub media_type: MediaType<'a>,

    /// Image format, such as "Png".
    pub image_format: ImageFormat,
}

impl<'a> SupportedImageType<'a> {
    const fn new(media_type: MediaType<'a>, image_format: ImageFormat) -> Self {
        Self {
            media_type,
            image_format,
        }
    }
}

impl<'a> From<&'a SupportedImageType<'a>> for &'a MediaType<'a> {
    fn from(value: &'a SupportedImageType<'a>) -> Self {
        &value.media_type
    }
}
