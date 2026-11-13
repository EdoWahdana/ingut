use gpui_kit::assets::IconName;
use gpui_kit::component::Icon;
use gpui_kit::prelude::*;
use gpui_kit::{div, px, FontWeight, IntoElement, SharedString};

use crate::ui::theme::{Palette, FONT_CAPTION, FONT_HEADLINE, ICON_EMPTY, SPACE_4, SPACE_8};

pub fn empty_state(colors: &Palette) -> impl IntoElement {
    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(SPACE_8))
        .child(
            Icon::new(IconName::Clipboard)
                .text_color(colors.text_tertiary)
                .text_size(px(ICON_EMPTY)),
        )
        .child(
            div()
                .text_size(px(FONT_HEADLINE))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(colors.text_primary)
                .child("Nothing to remember yet"),
        )
        .child(
            div()
                .text_size(px(FONT_CAPTION))
                .text_color(colors.text_secondary)
                .child("Copy something and it will show up here."),
        )
}

pub fn no_results(query: impl Into<SharedString>, colors: &Palette) -> impl IntoElement {
    let query = query.into();
    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(SPACE_4))
        .child(
            div()
                .text_size(px(FONT_HEADLINE))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(colors.text_primary)
                .child(format!("No matches for \"{query}\"")),
        )
        .child(
            div()
                .text_size(px(FONT_CAPTION))
                .text_color(colors.text_secondary)
                .child("Try fewer letters"),
        )
}
