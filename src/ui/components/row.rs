use gpui_kit::assets::IconName;
use gpui_kit::component::{h_flex, Icon};
use gpui_kit::prelude::*;
use gpui_kit::{div, px, rgb, AnyElement, FontWeight, Hsla, IntoElement, SharedString};

use crate::storage::ClipItem;
use crate::ui::theme::{
    Palette, ACCENT_BAR, FONT_BODY, FONT_CAPTION, ICON_ROW, RADIUS_CHIP, RADIUS_ROW, ROW_HEIGHT,
    SPACE_12, SPACE_2, SPACE_8,
};

pub enum ClipKind {
    Link { domain: String, path: String },
    Color { hex: String },
    Image,
    Text,
}

pub struct HistoryRow {
    pub id: u64,
    pub label: SharedString,
    pub time: SharedString,
    pub kind: ClipKind,
    pub selected: bool,
}

pub fn section_label(pinned: bool, created_at_ms: i64, now_ms: i64) -> &'static str {
    if pinned {
        return "PINNED";
    }
    let age = now_ms.saturating_sub(created_at_ms);
    const DAY: i64 = 24 * 60 * 60 * 1000;
    if age < DAY {
        "TODAY"
    } else if age < DAY * 2 {
        "YESTERDAY"
    } else {
        "EARLIER"
    }
}

pub fn relative_time(created_at_ms: i64, now_ms: i64) -> String {
    let secs = now_ms.saturating_sub(created_at_ms).max(0) / 1000;
    if secs < 60 {
        "now".to_owned()
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86_400 {
        format!("{}h", secs / 3600)
    } else {
        format!("{}d", secs / 86_400)
    }
}

pub fn classify(item: &ClipItem) -> ClipKind {
    if matches!(item.content, crate::storage::ClipContent::Image { .. }) {
        return ClipKind::Image;
    }
    let text = item.preview();
    if let Some((domain, path)) = split_url(&text) {
        return ClipKind::Link { domain, path };
    }
    if is_hex_color(&text) {
        return ClipKind::Color { hex: text };
    }
    ClipKind::Text
}

fn split_url(text: &str) -> Option<(String, String)> {
    let rest = text
        .strip_prefix("https://")
        .or_else(|| text.strip_prefix("http://"))?;
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    if host.is_empty() {
        return None;
    }
    let path = if path.is_empty() {
        String::new()
    } else {
        format!("/{path}")
    };
    Some((host.to_owned(), path))
}

fn is_hex_color(text: &str) -> bool {
    let Some(hex) = text.strip_prefix('#') else {
        return false;
    };
    matches!(hex.len(), 3 | 6 | 8) && hex.chars().all(|ch| ch.is_ascii_hexdigit())
}

impl HistoryRow {
    pub fn render(self, colors: &Palette) -> impl IntoElement {
        let icon = match &self.kind {
            ClipKind::Link { .. } => IconName::Link,
            ClipKind::Color { .. } => IconName::Hash,
            ClipKind::Image => IconName::Image,
            ClipKind::Text => IconName::Type,
        };
        let bar = if self.selected {
            colors.accent
        } else {
            colors.accent.alpha(0.)
        };
        let background = if self.selected {
            Some(colors.accent_soft)
        } else {
            None
        };
        let hover = colors.bg_secondary;
        let selected = self.selected;

        div()
            .id(("clip", self.id))
            .h(px(ROW_HEIGHT))
            .w_full()
            .min_w_0()
            .px(px(SPACE_8))
            .rounded(px(RADIUS_ROW))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(SPACE_8))
            .when_some(background, |row, color| row.bg(color))
            .hover(move |style| if selected { style } else { style.bg(hover) })
            .active(|style| style.opacity(0.92))
            .child(
                div()
                    .w(px(ACCENT_BAR))
                    .h(px(SPACE_12))
                    .rounded(px(SPACE_2))
                    .bg(bar),
            )
            .child(match &self.kind {
                ClipKind::Color { hex } => div()
                    .w(px(SPACE_12))
                    .h(px(SPACE_12))
                    .flex_shrink_0()
                    .rounded(px(RADIUS_CHIP))
                    .border_1()
                    .border_color(colors.border_hairline)
                    .bg(swatch(hex))
                    .into_any_element(),
                _ => Icon::new(icon)
                    .text_color(colors.text_secondary)
                    .text_size(px(ICON_ROW))
                    .into_any_element(),
            })
            .child(label_row(&self, colors))
            .child(
                div()
                    .flex_shrink_0()
                    .text_size(px(FONT_CAPTION))
                    .text_color(colors.text_secondary)
                    .child(self.time),
            )
    }
}

fn label_row(row: &HistoryRow, colors: &Palette) -> AnyElement {
    match &row.kind {
        ClipKind::Link { domain, path } => h_flex()
            .flex_1()
            .min_w_0()
            .overflow_hidden()
            .gap(px(SPACE_2))
            .text_size(px(FONT_BODY))
            .child(
                div()
                    .flex_shrink_0()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(colors.text_primary)
                    .child(domain.clone()),
            )
            .child(
                div()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_color(colors.text_secondary)
                    .child(path.clone()),
            )
            .into_any_element(),
        _ => div()
            .flex_1()
            .min_w_0()
            .overflow_hidden()
            .whitespace_nowrap()
            .text_ellipsis()
            .text_size(px(FONT_BODY))
            .text_color(colors.text_primary)
            .child(row.label.clone())
            .into_any_element(),
    }
}

fn swatch(hex: &str) -> Hsla {
    let digits = hex.trim_start_matches('#');
    let expanded = if digits.len() == 3 {
        digits.chars().flat_map(|ch| [ch, ch]).collect::<String>()
    } else {
        digits.to_owned()
    };
    let value = u32::from_str_radix(&expanded[..expanded.len().min(6)], 16).unwrap_or(0);
    rgb(value).into()
}
