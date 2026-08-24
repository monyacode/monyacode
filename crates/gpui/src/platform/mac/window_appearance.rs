use objc2_app_kit::{
    NSAppearance, NSAppearanceNameAqua, NSAppearanceNameDarkAqua, NSAppearanceNameVibrantDark,
    NSAppearanceNameVibrantLight,
};

use crate::WindowAppearance;

impl WindowAppearance {
    pub(crate) fn from_native(appearance: &NSAppearance) -> Self {
        unsafe {
            let name = appearance.name();
            if &*name == NSAppearanceNameVibrantLight {
                Self::VibrantLight
            } else if &*name == NSAppearanceNameVibrantDark {
                Self::VibrantDark
            } else if &*name == NSAppearanceNameAqua {
                Self::Light
            } else if &*name == NSAppearanceNameDarkAqua {
                Self::Dark
            } else {
                println!("unknown appearance");
                Self::Light
            }
        }
    }
}
