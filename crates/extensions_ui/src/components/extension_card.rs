use gpui::{AnyElement, prelude::*};
use smallvec::SmallVec;
use ui::prelude::*;

#[derive(IntoElement)]
pub struct ExtensionCard {
    children: SmallVec<[AnyElement; 2]>,
}

impl ExtensionCard {
    pub fn new() -> Self {
        Self {
            children: SmallVec::new(),
        }
    }
}

impl ParentElement for ExtensionCard {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements)
    }
}

impl RenderOnce for ExtensionCard {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div().w_full().child(
            v_flex()
                .mt_4()
                .w_full()
                .h(rems(7.))
                .p_3()
                .gap_2()
                .bg(cx.theme().colors().elevated_surface_background.opacity(0.5))
                .border_1()
                .border_color(cx.theme().colors().border_variant)
                .rounded_md()
                .children(self.children),
        )
    }
}
