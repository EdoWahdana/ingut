use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use gpui_kit::assets::IconName;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::storage::{filter_clips, ClipContent, ClipItem};
use crate::ui::components::{
    classify, empty_state, keycap, no_results, relative_time, section_label, HistoryRow,
};
use crate::ui::theme::{
    self, Palette, FONT_BODY, FONT_CAPTION, HAIRLINE, HINT_BAR_HEIGHT, ICON_SEARCH, PREVIEW_INSET,
    PREVIEW_MIN_WIDTH, PREVIEW_RATIO, RADIUS_CARD, SEARCH_BAR_HEIGHT, SEARCH_FIELD_HEIGHT,
    SPACE_12, SPACE_2, SPACE_4, SPACE_8,
};

/// Close the popup and return the keyboard to the app that was active before it opened.
pub fn dismiss_popup(window: &mut Window, cx: &mut App) {
    crate::platform::disable_popup_animation();
    window.remove_window();
    #[cfg(target_os = "macos")]
    if !crate::platform::restore_frontmost() {
        cx.hide();
    }
    #[cfg(not(target_os = "macos"))]
    let _ = cx;
}

/// Clipboard popup: search, history, and a preview of the selected item.
pub struct HistoryView {
    items: Vec<ClipItem>,
    query_input: Entity<InputState>,
    query: String,
    /// Navigation is the default. Insert is the search field, like Vim.
    inserting: bool,
    focus: FocusHandle,
    selected: Option<i64>,
    copied: bool,
    scroll: ScrollHandle,
    preview_image: Option<(i64, std::sync::Arc<Image>)>,
    /// Image whose bytes have been requested and not delivered yet.
    loading_image: Option<i64>,
    copy_item: Box<dyn Fn(&ClipItem) -> bool>,
    pin_item: Box<dyn Fn(i64, bool)>,
    request_image: Box<dyn Fn(i64)>,
    _subscriptions: Vec<Subscription>,
    _blur: Task<()>,
}

impl HistoryView {
    pub fn new(
        items: Vec<ClipItem>,
        copy_item: impl Fn(&ClipItem) -> bool + 'static,
        pin_item: impl Fn(i64, bool) + 'static,
        request_image: impl Fn(i64) + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        theme::sync(window, cx);
        let appearance = window.observe_window_appearance(|window, cx| theme::sync(window, cx));

        let query_input = cx.new(|cx| InputState::new(window, cx).placeholder("Search clipboard…"));
        let query_subscription = cx.subscribe_in(
            &query_input,
            window,
            |this, input, event, window, cx| match event {
                InputEvent::Change => {
                    this.query = input.read(cx).value().to_string();
                    this.ensure_selection();
                    cx.notify();
                }
                InputEvent::Focus => {
                    this.inserting = true;
                    cx.notify();
                }
                InputEvent::Blur => {
                    this.inserting = false;
                    this.focus.focus(window, cx);
                    cx.notify();
                }
                InputEvent::PressEnter { .. } => {}
            },
        );

        let focus = cx.focus_handle();
        focus.focus(window, cx);
        cx.defer_in(window, |this, window, cx| {
            this.refresh_preview_image();
            if !this.inserting {
                this.focus.focus(window, cx);
            }
            cx.notify();
        });

        let selected = items.first().map(|item| item.id);
        let blur = watch_blur(window, cx);

        Self {
            items,
            query_input,
            query: String::new(),
            inserting: false,
            focus,
            selected,
            copied: false,
            scroll: ScrollHandle::new(),
            preview_image: None,
            loading_image: None,
            copy_item: Box::new(copy_item),
            pin_item: Box::new(pin_item),
            request_image: Box::new(request_image),
            _subscriptions: vec![appearance, query_subscription],
            _blur: blur,
        }
    }

    pub fn show_items(
        &mut self,
        items: Vec<ClipItem>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.items = items;
        self.ensure_selection();
        self.refresh_preview_image();
        cx.notify();
    }

    fn visible(&self) -> Vec<ClipItem> {
        filter_clips(&self.items, &self.query)
    }

    fn ensure_selection(&mut self) {
        let visible = self.visible();
        if visible.iter().any(|item| Some(item.id) == self.selected) {
            return;
        }
        self.selected = visible.first().map(|item| item.id);
        self.refresh_preview_image();
    }

    fn refresh_preview_image(&mut self) {
        let selected = self.items.iter().find_map(|item| {
            if Some(item.id) != self.selected {
                return None;
            }
            match &item.content {
                ClipContent::Image { .. } => Some(Some(item.id)),
                ClipContent::Text(_) => Some(None),
            }
        });
        let Some(image_id) = selected else {
            self.loading_image = None;
            self.preview_image = None;
            return;
        };
        let Some(id) = image_id else {
            self.loading_image = None;
            self.preview_image = None;
            return;
        };
        if self
            .preview_image
            .as_ref()
            .is_some_and(|(shown, _)| *shown == id)
            || self.loading_image == Some(id)
        {
            return;
        }
        self.loading_image = Some(id);
        self.preview_image = None;
        (self.request_image)(id);
    }

    /// The history list stores image size only. Bytes arrive when this row is previewed.
    pub fn supply_image(&mut self, id: i64, bytes: Vec<u8>, cx: &mut Context<Self>) {
        if self.loading_image == Some(id) {
            self.loading_image = None;
        }
        if self.selected != Some(id) {
            return;
        }
        let kind = self.items.iter().find_map(|item| match &item.content {
            ClipContent::Image { kind, .. } if item.id == id => Some(kind.clone()),
            _ => None,
        });
        let Some(kind) = kind else {
            return;
        };
        self.preview_image = Some((
            id,
            std::sync::Arc::new(Image {
                format: image_format(&kind),
                bytes,
                id: id as u64,
            }),
        ));
        cx.notify();
    }

    fn toggle_pin(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.selected else {
            return;
        };
        let Some(item) = self.items.iter_mut().find(|item| item.id == id) else {
            return;
        };
        item.pinned = !item.pinned;
        let pinned = item.pinned;
        self.items
            .sort_by(|a, b| b.pinned.cmp(&a.pinned).then(b.id.cmp(&a.id)));
        (self.pin_item)(id, pinned);
        cx.notify();
    }

    fn selected_item(&self) -> Option<&ClipItem> {
        self.visible()
            .iter()
            .find(|item| Some(item.id) == self.selected)
            .map(|item| {
                // The borrow below re-finds the item in `self.items` so the
                // reference lives as long as `self`.
                item.id
            })
            .and_then(|id| self.items.iter().find(|item| item.id == id))
    }

    fn move_selection(&mut self, up: bool, cx: &mut Context<Self>) {
        let visible = self.visible();
        if visible.is_empty() {
            return;
        }
        let current = visible
            .iter()
            .position(|item| Some(item.id) == self.selected)
            .unwrap_or(0);
        let next = if up {
            current.saturating_sub(1)
        } else {
            (current + 1).min(visible.len() - 1)
        };
        self.selected = Some(visible[next].id);
        self.refresh_preview_image();
        // The first row of a section keeps its header on screen. Resetting the
        // first child avoids leaving the list a header short of the top, which
        // clips both the section title and the last row.
        let child = scroll_child_index(&visible, visible[next].id, now_ms());
        if child == 0 {
            self.scroll.set_offset(point(px(0.), px(0.)));
        } else if next + 1 == visible.len() {
            self.scroll.scroll_to_bottom();
        } else {
            self.scroll.scroll_to_item(child);
        }
        cx.notify();
    }

    fn copy_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self.selected_item().cloned() else {
            return;
        };
        if (self.copy_item)(&item) {
            self.copied = true;
            cx.notify();
            cx.spawn_in(window, async move |this, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(250))
                    .await;
                let _ = this.update_in(cx, |_view, window, cx| dismiss_popup(window, cx));
            })
            .detach();
        }
    }

    fn enter_insert(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.inserting = true;
        self.query_input.update(cx, |input, cx| {
            input.focus_handle(cx).focus(window, cx);
        });
        cx.notify();
    }

    fn leave_insert(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.inserting = false;
        self.focus.focus(window, cx);
        cx.notify();
    }

    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let mods = event.keystroke.modifiers;
        if mods.platform || mods.control || mods.alt {
            return;
        }
        let key = event.keystroke.key.to_ascii_lowercase();
        if self.inserting {
            if key == "escape" {
                cx.stop_propagation();
                self.leave_insert(window, cx);
            }
            return;
        }
        match key.as_str() {
            "j" | "down" => {
                cx.stop_propagation();
                self.move_selection(false, cx);
            }
            "k" | "up" => {
                cx.stop_propagation();
                self.move_selection(true, cx);
            }
            "i" => {
                cx.stop_propagation();
                self.enter_insert(window, cx);
            }
            "p" => {
                cx.stop_propagation();
                self.toggle_pin(cx);
            }
            "enter" => {
                cx.stop_propagation();
                self.copy_selected(window, cx);
            }
            "escape" => {
                cx.stop_propagation();
                dismiss_popup(window, cx);
            }
            _ => {}
        }
    }
}

impl Render for HistoryView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = Palette::from_appearance(window.appearance());
        let visible = self.visible();
        let show_preview = window.bounds().size.width.as_f32() >= PREVIEW_MIN_WIDTH;
        let inserting = self.inserting;
        let now = now_ms();
        let selected = self.selected_item().cloned();
        let copied = self.copied;
        let count = visible.len();

        div()
            .id("history")
            .size_full()
            .min_w_0()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(panel_fill(&colors))
            .text_color(colors.text_primary)
            .font_family(".SystemUIFont")
            .track_focus(&self.focus)
            .border_t_1()
            .border_color(colors.border_glass)
            .child(search_bar(&self.query_input, inserting, &colors))
            .child(hairline(&colors))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_row()
                    .items_stretch()
                    .child(history_list(
                        &visible,
                        self.selected,
                        now,
                        &colors,
                        &self.scroll,
                        &self.query,
                    ))
                    .when(show_preview, |row| {
                        row.child(div().w(px(HAIRLINE)).h_full().bg(colors.border_hairline))
                            .child(preview_pane(
                                selected.as_ref(),
                                self.preview_image.as_ref().map(|(_, image)| image.clone()),
                                now,
                                copied,
                                &colors,
                            ))
                    }),
            )
            .child(hairline(&colors))
            .child(hint_bar(
                count,
                copied,
                inserting,
                selected.as_ref().is_some_and(|item| item.pinned),
                &colors,
            ))
            .capture_key_down(cx.listener(HistoryView::on_key))
    }
}

fn search_bar(input: &Entity<InputState>, focused: bool, colors: &Palette) -> impl IntoElement {
    let accent_ring = colors.accent.alpha(0.3);
    div()
        .h(px(SEARCH_BAR_HEIGHT))
        .w_full()
        .px(px(SPACE_12))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(SPACE_8))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .h(px(SEARCH_FIELD_HEIGHT))
                .px(px(SPACE_8))
                .rounded(px(RADIUS_CARD))
                .bg(colors.bg_secondary)
                .flex()
                .flex_row()
                .items_center()
                .when(focused, move |field| {
                    field.border_2().border_color(accent_ring)
                })
                .child(
                    Input::new(input)
                        .appearance(false)
                        .with_size(gpui_kit::component::Size::Small)
                        .prefix(
                            Icon::new(IconName::Search)
                                .text_color(colors.text_tertiary)
                                .text_size(px(ICON_SEARCH)),
                        ),
                ),
        )
}

fn history_list(
    visible: &[ClipItem],
    selected: Option<i64>,
    now: i64,
    colors: &Palette,
    scroll: &ScrollHandle,
    query: &str,
) -> AnyElement {
    if visible.is_empty() && query.trim().is_empty() {
        return empty_state(colors).into_any_element();
    }
    if visible.is_empty() {
        return no_results(query.trim(), colors).into_any_element();
    }

    let mut rows: Vec<AnyElement> = Vec::new();
    let mut current_section: Option<&'static str> = None;
    for item in visible {
        let section = section_label(item.pinned, item.created_at_ms, now);
        if current_section != Some(section) {
            current_section = Some(section);
            rows.push(section_header(section, colors).into_any_element());
        }
        let kind = classify(item);
        rows.push(
            HistoryRow {
                id: item.id as u64,
                label: item.preview().into(),
                time: relative_time(item.created_at_ms, now).into(),
                kind,
                selected: Some(item.id) == selected,
            }
            .render(colors)
            .into_any_element(),
        );
    }

    div()
        .id("history-list")
        .flex_1()
        .min_w_0()
        .h_full()
        .px(px(SPACE_8))
        .py(px(SPACE_8))
        .gap(px(SPACE_2))
        .overflow_y_scroll()
        .track_scroll(scroll)
        .children(rows)
        .into_any_element()
}

fn section_header(label: &'static str, colors: &Palette) -> impl IntoElement {
    div()
        .px(px(SPACE_8))
        .pt(px(SPACE_8))
        .pb(px(SPACE_2))
        .text_size(px(FONT_CAPTION))
        .font_weight(gpui_kit::FontWeight::SEMIBOLD)
        .text_color(colors.text_secondary)
        .child(label)
}

fn preview_pane(
    item: Option<&ClipItem>,
    image: Option<std::sync::Arc<Image>>,
    now: i64,
    copied: bool,
    colors: &Palette,
) -> impl IntoElement {
    div()
        .w(relative(PREVIEW_RATIO))
        .h_full()
        .min_w_0()
        .p(px(PREVIEW_INSET))
        .child(
            div()
                .size_full()
                .rounded(px(RADIUS_CARD))
                .bg(colors.bg_elevated)
                .p(px(SPACE_12))
                .flex()
                .flex_col()
                .gap(px(SPACE_8))
                .overflow_hidden()
                .child(preview_body(item, image, now, copied, colors)),
        )
}

fn preview_body(
    item: Option<&ClipItem>,
    image: Option<std::sync::Arc<Image>>,
    now: i64,
    copied: bool,
    colors: &Palette,
) -> AnyElement {
    let Some(item) = item else {
        return div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(FONT_CAPTION))
            .text_color(colors.text_tertiary)
            .child("Nothing selected")
            .into_any_element();
    };
    let when = relative_time(item.created_at_ms, now);
    let (body, count_label) = match &item.content {
        ClipContent::Text(text) => {
            let count = text.chars().count();
            let label = if count == 1 {
                "1 character".to_owned()
            } else {
                format!("{count} characters")
            };
            (
                div()
                    .id("preview-body")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .text_size(px(FONT_BODY))
                    .text_color(colors.text_primary)
                    .child(text.clone())
                    .into_any_element(),
                label,
            )
        }
        ClipContent::Image { width, height, .. } => {
            let picture = image.map(|image| {
                img(image)
                    .id("preview-image")
                    .size_full()
                    .object_fit(ObjectFit::Contain)
                    .into_any_element()
            });
            (
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .children(picture)
                    .into_any_element(),
                format!("Image {width}×{height}"),
            )
        }
    };
    div()
        .size_full()
        .flex()
        .flex_col()
        .gap(px(SPACE_8))
        .child(body)
        .child(
            div()
                .flex_shrink_0()
                .text_size(px(FONT_CAPTION))
                .text_color(if copied {
                    colors.success
                } else {
                    colors.text_secondary
                })
                .child(if copied {
                    "Copied".to_owned()
                } else if when == "now" {
                    "Copied just now".to_owned()
                } else {
                    format!("Copied {when} ago")
                }),
        )
        .child(
            div()
                .flex_shrink_0()
                .text_size(px(FONT_CAPTION))
                .text_color(colors.text_tertiary)
                .child(count_label),
        )
        .into_any_element()
}

fn hint_bar(
    count: usize,
    copied: bool,
    inserting: bool,
    pinned: bool,
    colors: &Palette,
) -> impl IntoElement {
    let items = if count == 1 {
        "1 item".to_owned()
    } else {
        format!("{count} items")
    };
    div()
        .h(px(HINT_BAR_HEIGHT))
        .w_full()
        .px(px(SPACE_12))
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .text_size(px(FONT_CAPTION))
        .text_color(colors.text_secondary)
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(SPACE_12))
                .when(!inserting, |row| {
                    row.child(hint_pair("k", "Up", colors))
                        .child(hint_pair("j", "Down", colors))
                        .child(hint_pair("i", "Search", colors))
                        .child(hint_pair("p", if pinned { "Unpin" } else { "Pin" }, colors))
                        .child(hint_pair("enter", "Copy", colors))
                        .child(hint_pair("esc", "Close", colors))
                })
                .when(inserting, |row| row.child(hint_pair("esc", "Back", colors)))
                .when(copied, |row| {
                    row.child(
                        div()
                            .text_color(colors.success)
                            .font_weight(gpui_kit::FontWeight::MEDIUM)
                            .child("Copied"),
                    )
                }),
        )
        .child(div().child(items))
}

fn hint_pair(key: &str, label: &str, colors: &Palette) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(SPACE_4))
        .child(keycap(key, colors))
        .child(label.to_owned())
}

fn scroll_child_index(visible: &[ClipItem], selected_id: i64, now: i64) -> usize {
    let mut child = 0usize;
    let mut section: Option<&str> = None;
    for item in visible {
        let label = section_label(item.pinned, item.created_at_ms, now);
        let mut header = None;
        if section != Some(label) {
            section = Some(label);
            header = Some(child);
            child += 1;
        }
        if item.id == selected_id {
            return header.unwrap_or(child);
        }
        child += 1;
    }
    0
}

fn image_format(kind: &str) -> ImageFormat {
    match kind {
        "jpeg" => ImageFormat::Jpeg,
        "gif" => ImageFormat::Gif,
        "webp" => ImageFormat::Webp,
        _ => ImageFormat::Png,
    }
}

fn hairline(colors: &Palette) -> impl IntoElement {
    div().w_full().h(px(HAIRLINE)).bg(colors.border_hairline)
}

fn panel_fill(colors: &Palette) -> Hsla {
    if cfg!(target_os = "macos") {
        colors.bg_glass
    } else {
        colors.bg_primary
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

fn watch_blur(window: &mut Window, cx: &mut Context<HistoryView>) -> Task<()> {
    cx.spawn_in(window, async move |this, cx| {
        let opened = Instant::now();
        let mut was_active = false;
        loop {
            cx.background_executor()
                .timer(Duration::from_millis(80))
                .await;
            let close = this.update_in(cx, |_view, window, _cx| {
                if window.is_window_active() {
                    was_active = true;
                    false
                } else {
                    was_active && opened.elapsed() > Duration::from_millis(300)
                }
            });
            match close {
                Ok(true) => {
                    let _ = this.update_in(cx, |_view, window, cx| dismiss_popup(window, cx));
                    break;
                }
                Ok(false) => {}
                Err(_) => break,
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::scroll_child_index;
    use crate::storage::ClipItem;

    #[test]
    fn first_row_of_a_section_scrolls_to_its_header() {
        let mut items = vec![ClipItem::text(1, "first"), ClipItem::text(2, "second")];
        items[0].created_at_ms = 2_000;
        items[1].created_at_ms = 1_000;
        assert_eq!(scroll_child_index(&items, 1, 2_000), 0);
        assert_eq!(scroll_child_index(&items, 2, 2_000), 2);
    }

    #[test]
    fn a_later_section_scrolls_to_that_header() {
        let mut pinned = ClipItem::text(1, "pin");
        pinned.pinned = true;
        pinned.created_at_ms = 2_000;
        let mut older = ClipItem::text(2, "old");
        older.created_at_ms = 0;
        assert_eq!(scroll_child_index(&[pinned, older], 2, 2_000), 2);
    }
}
