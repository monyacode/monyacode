//! Macos screen have a y axis that goings up from the bottom of the screen and
//! an origin at the bottom left of the main display.
mod dispatcher;
mod display;
mod display_link;
mod events;
mod keyboard;
mod pasteboard;

mod blurred_view;
mod gpui_view;
mod platform;
mod window;
mod window_appearance;

use crate::{DevicePixels, Pixels, Size, px, size};

pub(crate) use dispatcher::*;
pub(crate) use display::*;
pub(crate) use display_link::*;
pub(crate) use keyboard::*;
use objc2_foundation::{NSRect, NSSize};
pub(crate) use platform::*;
pub(crate) use window::*;

impl From<NSSize> for Size<Pixels> {
    fn from(value: NSSize) -> Self {
        Size {
            width: px(value.width as f32),
            height: px(value.height as f32),
        }
    }
}

impl From<NSRect> for Size<Pixels> {
    fn from(rect: NSRect) -> Self {
        let NSSize { width, height } = rect.size;
        size(width.into(), height.into())
    }
}

impl From<NSRect> for Size<DevicePixels> {
    fn from(rect: NSRect) -> Self {
        let NSSize { width, height } = rect.size;
        size(DevicePixels(width as i32), DevicePixels(height as i32))
    }
}
