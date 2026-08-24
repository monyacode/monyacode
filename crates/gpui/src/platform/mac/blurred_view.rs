use objc2::{ClassType, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained};
use objc2_app_kit::{NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView};
use objc2_foundation::NSRect;

use crate::remove_layer_background;

define_class!(
    #[unsafe(super(NSVisualEffectView))]
    #[name = "BlurredView"]
    #[thread_kind = MainThreadOnly]
    pub struct BlurredView;

    impl BlurredView {
        #[unsafe(method(updateLayer))]
        fn update_layer(&self) {
            let _: () = unsafe { msg_send![super(self, NSVisualEffectView::class()), updateLayer] };
            if let Some(layer) = self.layer() {
                remove_layer_background(&layer)
            }
        }
    }
);

impl BlurredView {
    pub fn new(mtm: MainThreadMarker, frame: NSRect) -> Retained<Self> {
        let this = Self::alloc(mtm);
        let view: Retained<Self> = unsafe { msg_send![this, initWithFrame: frame] };
        // Use a colorless semantic material. The default value `AppearanceBased`, though not
        // manually set, is deprecated.
        view.setMaterial(NSVisualEffectMaterial::Selection);
        view.setState(NSVisualEffectState::Active);
        view
    }
}
