use crate::prelude::*;

use gpui::{IntoElement, Styled, img};

pub fn render_avatar(size: Rems, window: &mut Window, cx: &mut App) -> impl IntoElement {
    let border_width = px(0.);
    let container_size = size.to_pixels(window.rem_size()) + border_width * 2.;
    let image = img("");

    div().size(container_size).rounded_full().child(
        image
            .size(size)
            .rounded_full()
            .bg(cx.theme().colors().element_disabled)
            .with_fallback(|| {
                h_flex()
                    .size_full()
                    .justify_center()
                    .child(Icon::new(IconName::Person).color(Color::Muted).size(IconSize::Small))
                    .into_any_element()
            }),
    )
}
