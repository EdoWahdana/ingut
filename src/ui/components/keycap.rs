use gpui_kit::prelude::*;
use gpui_kit::{div, px, FontWeight, IntoElement, SharedString};

use crate::ui::theme::{Palette, FONT_CAPTION, RADIUS_CHIP, SPACE_2, SPACE_4};

pub fn keycap(label: impl Into<SharedString>, colors: &Palette) -> impl IntoElement {
    div()
        .px(px(SPACE_4))
        .py(px(SPACE_2))
        .rounded(px(RADIUS_CHIP))
        .border_1()
        .border_color(colors.border_hairline)
        .text_size(px(FONT_CAPTION))
        .font_weight(FontWeight::MEDIUM)
        .text_color(colors.text_secondary)
        .child(label.into())
}
