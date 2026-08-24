use crate::{Bounds, DisplayId, Pixels, PlatformDisplay, point, px, size};
use anyhow::Result;
use objc2::{MainThreadMarker, rc::Retained};
use objc2_app_kit::NSScreen;
use objc2_color_sync::CGDisplayCreateUUIDFromDisplayID;
use objc2_core_graphics::{CGDirectDisplayID, CGDisplayBounds, CGError, CGGetActiveDisplayList, kCGNullDirectDisplay};
use objc2_foundation::{NSNumber, ns_string};
use uuid::Uuid;

#[derive(Debug)]
pub(crate) struct MacDisplay(pub(crate) CGDirectDisplayID);

unsafe impl Send for MacDisplay {}

impl MacDisplay {
    /// Get the screen with the given [`DisplayId`].
    pub fn find_by_id(id: DisplayId) -> Option<Self> {
        Self::all().find(|screen| screen.id() == id)
    }

    /// Get the primary screen - the one with the menu bar, and whose bottom left
    /// corner is at the origin of the AppKit coordinate system.
    pub fn primary() -> Self {
        // Instead of iterating through all active systems displays via `all()` we use the first
        // NSScreen and gets its CGDirectDisplayID, because we can't be sure that `CGGetActiveDisplayList`
        // will always return a list of active displays (machine might be sleeping).
        //
        // The following is what Chromium does too:
        //
        // https://chromium.googlesource.com/chromium/src/+/66.0.3359.158/ui/display/mac/screen_mac.mm#56
        let mtm = MainThreadMarker::new().expect("Must be on the main thread");
        let screens = NSScreen::screens(mtm);
        let screen = screens.objectAtIndex(0);
        Self(screen_display_id(&screen))
    }

    /// Obtains an iterator over all currently active system displays.
    pub fn all() -> impl Iterator<Item = Self> {
        unsafe {
            // We're assuming there aren't more than 32 displays connected to the system.
            let mut displays = Vec::with_capacity(32);
            let mut display_count = 0;
            let result = CGGetActiveDisplayList(displays.capacity() as u32, displays.as_mut_ptr(), &mut display_count);

            match result {
                CGError::Success => {
                    displays.set_len(display_count as usize);
                    displays.into_iter().map(MacDisplay)
                }
                _ => panic!("Failed to get active display list. Result: {result:?}"),
            }
        }
    }
}

impl PlatformDisplay for MacDisplay {
    fn id(&self) -> DisplayId {
        DisplayId(self.0)
    }

    fn uuid(&self) -> Result<Uuid> {
        let cfuuid = unsafe { CGDisplayCreateUUIDFromDisplayID(self.0) };
        Ok(Uuid::from_bytes(cfuuid.uuid_bytes().into()))
    }

    fn bounds(&self) -> Bounds<Pixels> {
        // CGDisplayBounds is in "global display" coordinates, where 0 is
        // the top left of the primary display.
        let bounds = CGDisplayBounds(self.0);

        Bounds {
            origin: Default::default(),
            size: size(px(bounds.size.width as f32), px(bounds.size.height as f32)),
        }
    }

    fn visible_bounds(&self) -> Bounds<Pixels> {
        let Some(dominated_screen) = self.get_nsscreen() else {
            return self.bounds();
        };

        let screen_frame = dominated_screen.frame();
        let visible_frame = dominated_screen.visibleFrame();

        // Convert from bottom-left origin (AppKit) to top-left origin
        let origin_y =
            screen_frame.size.height - visible_frame.origin.y - visible_frame.size.height + screen_frame.origin.y;

        Bounds {
            origin: point(
                px(visible_frame.origin.x as f32 - screen_frame.origin.x as f32),
                px(origin_y as f32),
            ),
            size: size(
                px(visible_frame.size.width as f32),
                px(visible_frame.size.height as f32),
            ),
        }
    }
}

impl MacDisplay {
    /// Find the NSScreen corresponding to this display
    fn get_nsscreen(&self) -> Option<Retained<NSScreen>> {
        let mtm = MainThreadMarker::new().expect("Must be on the main thread");
        let screens = NSScreen::screens(mtm);

        for i in 0..screens.count() {
            let screen = screens.objectAtIndex(i);
            let screen_id = screen_display_id(&screen);
            if screen_id == kCGNullDirectDisplay {
                continue;
            }
            if screen_id == self.0 {
                return Some(screen);
            }
        }
        None
    }
}

pub fn screen_display_id(screen: &NSScreen) -> CGDirectDisplayID {
    let device_description = screen.deviceDescription();
    let Some(display_number) = device_description.objectForKey(ns_string!("NSScreenNumber")) else {
        return kCGNullDirectDisplay;
    };
    let Some(number) = display_number.downcast_ref::<NSNumber>() else {
        return kCGNullDirectDisplay;
    };
    number.unsignedIntValue()
}
