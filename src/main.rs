//! Ingut, from the word "ingat". A light, cross-platform clipboard manager.

mod clipboard;
mod platform;
mod storage;
mod ui;

use std::thread;
use std::time::{Duration, Instant};

use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use gpui_kit::*;

use std::sync::mpsc;

use clipboard::{
    copy_image, copy_text, image_fingerprint, ClipRead, SystemClipboard, Watcher, POLL_INTERVAL,
};
use storage::{ClipContent, ClipItem, SqliteHistory};
use ui::HistoryView;

use ui::{POPUP_HEIGHT, POPUP_WIDTH};

struct Popup {
    window: AnyWindowHandle,
    view: Entity<HistoryView>,
}

enum StoreCmd {
    Pin {
        id: i64,
        pinned: bool,
    },
    Preload(i64),
    Load {
        id: i64,
        reply: mpsc::SyncSender<Option<Vec<u8>>>,
    },
}

fn main() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);

            let hotkey = HotKey::new(Some(Modifiers::META | Modifiers::SHIFT), Code::KeyV);
            let hotkey_id = hotkey.id();
            let manager = match GlobalHotKeyManager::new() {
                Ok(manager) => {
                    if let Err(err) = manager.register(hotkey) {
                        eprintln!("Ingut: failed to register the global shortcut: {err}");
                    }
                    Some(manager)
                }
                Err(err) => {
                    eprintln!("Ingut: global shortcut is unavailable: {err}");
                    None
                }
            };

            let (list_tx, list_rx) = mpsc::channel();
            let (image_tx, image_rx) = mpsc::channel();
            let (cmd_tx, cmd_rx) = mpsc::channel();
            thread::spawn(move || watch_history(list_tx, image_tx, cmd_rx));

            cx.spawn(async move |cx| {
                let _manager = manager;
                let mut items = Vec::new();
                let mut popup = None;
                let mut opened = false;

                loop {
                    let mut saw_snapshot = false;
                    while let Ok(latest) = list_rx.try_recv() {
                        items = latest;
                        saw_snapshot = true;
                        if let Some(current) = popup.as_ref() {
                            let pushed = cx.update(|cx| push_items(current, items.clone(), cx));
                            if !pushed {
                                popup = None;
                            }
                        }
                    }

                    while let Ok((id, bytes)) = image_rx.try_recv() {
                        if let Some(current) = popup.as_ref() {
                            let pushed = cx.update(|cx| supply_image(current, id, bytes, cx));
                            if !pushed {
                                popup = None;
                            }
                        }
                    }

                    if !opened && saw_snapshot {
                        while GlobalHotKeyEvent::receiver().try_recv().is_ok() {}
                        opened = true;
                        popup = cx.update(|cx| Some(open_popup(items.clone(), cmd_tx.clone(), cx)));
                    }

                    if opened {
                        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
                            if event.state == HotKeyState::Pressed && event.id == hotkey_id {
                                popup = cx.update(|cx| {
                                    toggle_popup(popup, items.clone(), cmd_tx.clone(), cx)
                                });
                            }
                        }
                    }

                    cx.background_executor()
                        .timer(Duration::from_millis(20))
                        .await;
                }
            })
            .detach();
        });
}

fn clip_fingerprint(store: &SqliteHistory, item: &ClipItem) -> Option<String> {
    match &item.content {
        ClipContent::Text(text) if !text.is_empty() => Some(text.clone()),
        ClipContent::Text(_) => None,
        ClipContent::Image { .. } => store
            .image_bytes(item.id)
            .ok()
            .flatten()
            .map(|bytes| image_fingerprint(&bytes)),
    }
}

fn watch_history(
    list_tx: mpsc::Sender<Vec<ClipItem>>,
    image_tx: mpsc::Sender<(i64, Vec<u8>)>,
    cmd_rx: mpsc::Receiver<StoreCmd>,
) {
    let Ok(mut store) = SqliteHistory::open_default() else {
        let _ = list_tx.send(Vec::new());
        return;
    };
    let mut items = store.list().unwrap_or_default();
    let mut watcher = SystemClipboard::new().ok().map(|clipboard| {
        let mut watcher = Watcher::new(clipboard);
        watcher.prime(
            items
                .first()
                .and_then(|item| clip_fingerprint(&store, item)),
        );
        watcher
    });
    if list_tx.send(items.clone()).is_err() {
        return;
    }

    let mut next_poll = Instant::now();
    loop {
        let wait = next_poll.saturating_duration_since(Instant::now());
        let mut changed = false;
        match cmd_rx.recv_timeout(wait) {
            Ok(StoreCmd::Pin { id, pinned }) => {
                if store.set_pinned(id, pinned).is_ok() {
                    changed = true;
                }
            }
            Ok(StoreCmd::Preload(id)) => {
                if let Ok(Some(bytes)) = store.image_bytes(id) {
                    let _ = image_tx.send((id, bytes));
                }
            }
            Ok(StoreCmd::Load { id, reply }) => {
                let bytes = store.image_bytes(id).ok().flatten();
                let _ = reply.send(bytes);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                next_poll = Instant::now() + POLL_INTERVAL;
                if let Some(watcher) = watcher.as_mut() {
                    if let Ok(Some(clip)) = watcher.poll() {
                        let inserted = match clip {
                            ClipRead::Text(text) => store.insert_text(&text).is_ok(),
                            ClipRead::Image {
                                bytes,
                                width,
                                height,
                                kind,
                            } => store
                                .insert_image(bytes, width, height, kind.as_str())
                                .is_ok(),
                        };
                        if inserted {
                            changed = true;
                        }
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }

        if changed {
            if let Ok(latest) = store.list() {
                items = latest;
                if list_tx.send(items.clone()).is_err() {
                    break;
                }
            }
        }
    }
}

fn load_image_bytes(cmd_tx: &mpsc::Sender<StoreCmd>, id: i64) -> Option<Vec<u8>> {
    let (reply_tx, reply_rx) = mpsc::sync_channel(1);
    cmd_tx
        .send(StoreCmd::Load {
            id,
            reply: reply_tx,
        })
        .ok()?;
    reply_rx.recv_timeout(Duration::from_secs(1)).ok().flatten()
}

fn copy_clip(item: &ClipItem, cmd_tx: &mpsc::Sender<StoreCmd>) -> bool {
    if let Some(text) = item.as_text() {
        return copy_text(text).is_ok();
    }
    let ClipContent::Image { width, height, .. } = &item.content else {
        return false;
    };
    let Some(bytes) = load_image_bytes(cmd_tx, item.id) else {
        return false;
    };
    copy_image(&bytes, *width, *height).is_ok()
}

fn open_popup(items: Vec<ClipItem>, cmd_tx: mpsc::Sender<StoreCmd>, cx: &mut App) -> Popup {
    let (bounds, display_id) = popup_bounds(cx);
    let items_for_view = items;
    let (window, view) = gpui_kit::open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: None,
            focus: true,
            show: true,
            kind: WindowKind::PopUp,
            is_resizable: false,
            is_minimizable: false,
            display_id,
            ..Default::default()
        },
        cx,
        |window, cx| {
            cx.new(|cx| {
                HistoryView::new(
                    items_for_view,
                    {
                        let cmd_tx = cmd_tx.clone();
                        move |item| copy_clip(item, &cmd_tx)
                    },
                    {
                        let cmd_tx = cmd_tx.clone();
                        move |id, pinned| {
                            let _ = cmd_tx.send(StoreCmd::Pin { id, pinned });
                        }
                    },
                    {
                        let cmd_tx = cmd_tx.clone();
                        move |id| {
                            let _ = cmd_tx.send(StoreCmd::Preload(id));
                        }
                    },
                    window,
                    cx,
                )
            })
        },
    )
    .expect("failed to open the Ingut window");
    platform::remember_frontmost();
    cx.activate(true);
    platform::disable_popup_animation();
    Popup { window, view }
}

fn toggle_popup(
    current: Option<Popup>,
    items: Vec<ClipItem>,
    cmd_tx: mpsc::Sender<StoreCmd>,
    cx: &mut App,
) -> Option<Popup> {
    if let Some(popup) = current {
        if popup
            .window
            .update(cx, |_root, window, cx| ui::dismiss_popup(window, cx))
            .is_ok()
        {
            return None;
        }
    }
    Some(open_popup(items, cmd_tx, cx))
}

fn supply_image(popup: &Popup, id: i64, bytes: Vec<u8>, cx: &mut App) -> bool {
    let view = popup.view.clone();
    popup
        .window
        .update(cx, |_root, _window, cx| {
            view.update(cx, |view, cx| view.supply_image(id, bytes, cx));
        })
        .is_ok()
}

fn push_items(popup: &Popup, items: Vec<ClipItem>, cx: &mut App) -> bool {
    let view = popup.view.clone();
    popup
        .window
        .update(cx, |_root, window, cx| {
            view.update(cx, |view, cx| view.show_items(items, window, cx));
        })
        .is_ok()
}

/// Place the popup near the cursor, clamped to the visible screen.
fn popup_bounds(cx: &App) -> (Bounds<Pixels>, Option<DisplayId>) {
    let popup = size(px(POPUP_WIDTH), px(POPUP_HEIGHT));
    let Some(anchor) = platform::cursor_anchor() else {
        return (Bounds::centered(None, popup, cx), None);
    };

    let display = if anchor.primary {
        cx.primary_display()
    } else {
        cx.displays().into_iter().find(|display| {
            let bounds = display.bounds().size;
            (bounds.width.as_f32() - anchor.width).abs() < 1.
                && (bounds.height.as_f32() - anchor.height).abs() < 1.
        })
    };
    let Some(display) = display.or_else(|| cx.primary_display()) else {
        return (Bounds::centered(None, popup, cx), None);
    };

    let area = display.visible_bounds();
    let min_x = area.origin.x.as_f32();
    let min_y = area.origin.y.as_f32();
    let max_x = (min_x + area.size.width.as_f32() - POPUP_WIDTH).max(min_x);
    let max_y = (min_y + area.size.height.as_f32() - POPUP_HEIGHT).max(min_y);
    let origin = point(
        px((anchor.x + 12.).clamp(min_x, max_x)),
        px((anchor.y + 12.).clamp(min_y, max_y)),
    );
    (
        Bounds {
            origin,
            size: popup,
        },
        Some(display.id()),
    )
}
