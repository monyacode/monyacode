use std::ffi::c_void;

use objc2::rc::Retained;
use objc2_app_kit::{
    NSPasteboard, NSPasteboardNameFind, NSPasteboardType, NSPasteboardTypePNG, NSPasteboardTypeString,
    NSPasteboardTypeTIFF,
};
use objc2_foundation::{NSData, NSUInteger, ns_string};
use strum::IntoEnumIterator as _;

use crate::{ClipboardEntry, ClipboardItem, ClipboardString, Image, ImageFormat, asset_cache::hash};

static TEXT_HASH_TYPE: &'static str = "monyacode-text-hash";
static METADATA_TYPE: &'static str = "monyacode-metadata";

pub struct Pasteboard {
    inner: Retained<NSPasteboard>,
}

impl Pasteboard {
    pub fn general() -> Self {
        Self::new(NSPasteboard::generalPasteboard())
    }

    pub fn find() -> Self {
        Self::new(NSPasteboard::pasteboardWithName(unsafe { NSPasteboardNameFind }))
    }

    #[cfg(test)]
    pub fn unique() -> Self {
        Self::new(NSPasteboard::pasteboardWithUniqueName())
    }

    fn new(inner: Retained<NSPasteboard>) -> Self {
        Self { inner }
    }

    pub fn read(&self) -> Option<ClipboardItem> {
        let Some(pasteboard_types) = self.inner.types() else {
            return None;
        };

        // First, see if it's a string.
        let string_type = ns_string!("public.utf8-plain-text");
        if pasteboard_types.containsObject(string_type) {
            let Some(data) = self.inner.dataForType(string_type) else {
                return None;
            };
            if data.is_empty() {
                return Some(self.read_string(&[]));
            }
            let bytes = unsafe { data.as_bytes_unchecked() };
            return Some(self.read_string(bytes));
        }

        // If it wasn't a string, try the various supported image types.
        for format in ImageFormat::iter() {
            if let Some(item) = self.read_image(format) {
                return Some(item);
            }
        }

        // If it wasn't a string or a supported image type, give up.
        None
    }

    fn read_image(&self, format: ImageFormat) -> Option<ClipboardItem> {
        let mut ut_type: UTType = format.into();

        let Some(types) = self.inner.types() else {
            return None;
        };
        if !types.containsObject(ut_type.inner()) {
            return None;
        }
        self.data_for_type(ut_type.inner()).map(|bytes| {
            let bytes = bytes.to_vec();
            let id = hash(&bytes);

            ClipboardItem {
                entries: vec![ClipboardEntry::Image(Image { format, bytes, id })],
            }
        })
    }

    fn read_string(&self, text_bytes: &[u8]) -> ClipboardItem {
        let text_hash_type = ns_string!(TEXT_HASH_TYPE);
        let metadata_type = ns_string!(METADATA_TYPE);
        let text = String::from_utf8_lossy(text_bytes).to_string();
        let metadata = self.data_for_type(text_hash_type).and_then(|hash_bytes| {
            let hash_bytes = hash_bytes.try_into().ok()?;
            let hash = u64::from_be_bytes(hash_bytes);
            let metadata = self.data_for_type(metadata_type)?;

            if hash == ClipboardString::text_hash(&text) {
                String::from_utf8(metadata).ok()
            } else {
                None
            }
        });

        ClipboardItem {
            entries: vec![ClipboardEntry::String(ClipboardString { text, metadata })],
        }
    }

    fn data_for_type(&self, kind: &NSPasteboardType) -> Option<Vec<u8>> {
        let Some(data) = self.inner.dataForType(kind) else {
            return None;
        };
        Some(data.to_vec())
    }

    pub fn write(&self, item: ClipboardItem) {
        unsafe {
            match item.entries.as_slice() {
                [] => {
                    // Writing an empty list of entries just clears the clipboard.
                    self.inner.clearContents();
                }
                [ClipboardEntry::String(string)] => {
                    self.write_plaintext(string);
                }
                [ClipboardEntry::Image(image)] => {
                    self.write_image(image);
                }
                [ClipboardEntry::ExternalPaths(_)] => {}
                _ => {
                    // Agus NB: We're currently only writing string entries to the clipboard when we have more than one.
                    //
                    // This was the existing behavior before I refactored the outer clipboard code:
                    // https://github.com/zed-industries/zed/blob/65f7412a0265552b06ce122655369d6cc7381dd6/crates/gpui/src/platform/mac/platform.rs#L1060-L1110
                    //
                    // Note how `any_images` is always `false`. We should fix that, but that's orthogonal to the refactor.

                    let mut combined = ClipboardString {
                        text: String::new(),
                        metadata: None,
                    };

                    for entry in item.entries {
                        match entry {
                            ClipboardEntry::String(text) => {
                                combined.text.push_str(&text.text());
                                if combined.metadata.is_none() {
                                    combined.metadata = text.metadata;
                                }
                            }
                            _ => {}
                        }
                    }

                    self.write_plaintext(&combined);
                }
            }
        }
    }

    fn write_plaintext(&self, string: &ClipboardString) {
        let text_hash_type = ns_string!(TEXT_HASH_TYPE);
        let metadata_type = ns_string!(METADATA_TYPE);
        self.inner.clearContents();

        unsafe {
            let text_bytes =
                NSData::dataWithBytes_length(string.text.as_ptr() as *const c_void, string.text.len() as NSUInteger);
            self.inner.setData_forType(Some(&text_bytes), NSPasteboardTypeString);
        }

        if let Some(metadata) = string.metadata.as_ref() {
            let hash_bytes = ClipboardString::text_hash(&string.text).to_be_bytes();
            unsafe {
                let hash_bytes =
                    NSData::dataWithBytes_length(hash_bytes.as_ptr() as *const c_void, hash_bytes.len() as NSUInteger);
                self.inner.setData_forType(Some(&hash_bytes), text_hash_type);

                let metadata_bytes =
                    NSData::dataWithBytes_length(metadata.as_ptr() as *const c_void, metadata.len() as NSUInteger);
                self.inner.setData_forType(Some(&metadata_bytes), metadata_type);
            }
        }
    }

    unsafe fn write_image(&self, image: &Image) {
        self.inner.clearContents();

        unsafe {
            let bytes =
                NSData::dataWithBytes_length(image.bytes.as_ptr() as *const c_void, image.bytes.len() as NSUInteger);

            self.inner
                .setData_forType(Some(&bytes), Into::<UTType>::into(image.format).inner());
        }
    }
}

impl From<ImageFormat> for UTType {
    fn from(value: ImageFormat) -> Self {
        match value {
            ImageFormat::Png => Self::png(),
            ImageFormat::Jpeg => Self::jpeg(),
            ImageFormat::Tiff => Self::tiff(),
            ImageFormat::Webp => Self::webp(),
            ImageFormat::Gif => Self::gif(),
            ImageFormat::Bmp => Self::bmp(),
            ImageFormat::Svg => Self::svg(),
            ImageFormat::Ico => Self::ico(),
        }
    }
}

// See https://developer.apple.com/documentation/uniformtypeidentifiers/uttype-swift.struct/
pub struct UTType(&'static NSPasteboardType);

impl UTType {
    pub fn png() -> Self {
        // https://developer.apple.com/documentation/uniformtypeidentifiers/uttype-swift.struct/png
        Self(unsafe { NSPasteboardTypePNG }) // This is a rare case where there's a built-in NSPasteboardType
    }

    pub fn jpeg() -> Self {
        // https://developer.apple.com/documentation/uniformtypeidentifiers/uttype-swift.struct/jpeg
        Self(ns_string!("public.jpeg"))
    }

    pub fn gif() -> Self {
        // https://developer.apple.com/documentation/uniformtypeidentifiers/uttype-swift.struct/gif
        Self(ns_string!("com.compuserve.gif"))
    }

    pub fn webp() -> Self {
        // https://developer.apple.com/documentation/uniformtypeidentifiers/uttype-swift.struct/webp
        Self(ns_string!("org.webmproject.webp"))
    }

    pub fn bmp() -> Self {
        // https://developer.apple.com/documentation/uniformtypeidentifiers/uttype-swift.struct/bmp
        Self(ns_string!("com.microsoft.bmp"))
    }

    pub fn svg() -> Self {
        // https://developer.apple.com/documentation/uniformtypeidentifiers/uttype-swift.struct/svg
        Self(ns_string!("public.svg-image"))
    }

    pub fn ico() -> Self {
        // https://developer.apple.com/documentation/uniformtypeidentifiers/uttype-swift.struct/ico
        Self(ns_string!("com.microsoft.ico"))
    }

    pub fn tiff() -> Self {
        // https://developer.apple.com/documentation/uniformtypeidentifiers/uttype-swift.struct/tiff
        Self(unsafe { NSPasteboardTypeTIFF }) // This is a rare case where there's a built-in NSPasteboardType
    }

    fn inner(&self) -> &'static NSPasteboardType {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::{ClipboardEntry, ClipboardItem, ClipboardString};

    use super::*;

    #[test]
    fn test_string() {
        let pasteboard = Pasteboard::unique();
        assert_eq!(pasteboard.read(), None);

        let item = ClipboardItem::new_string("1".to_string());
        pasteboard.write(item.clone());
        assert_eq!(pasteboard.read(), Some(item));

        let item = ClipboardItem {
            entries: vec![ClipboardEntry::String(
                ClipboardString::new("2".to_string()).with_json_metadata(vec![3, 4]),
            )],
        };
        pasteboard.write(item.clone());
        assert_eq!(pasteboard.read(), Some(item));

        let text_from_other_app = "text from other app";
        unsafe {
            let bytes = NSData::dataWithBytes_length(
                text_from_other_app.as_ptr() as *const c_void,
                text_from_other_app.len() as NSUInteger,
            );
            pasteboard.inner.setData_forType(Some(&bytes), NSPasteboardTypeString);
        }
        assert_eq!(
            pasteboard.read(),
            Some(ClipboardItem::new_string(text_from_other_app.to_string()))
        );
    }
}
