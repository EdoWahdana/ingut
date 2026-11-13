//! UI layer. Views and interaction only. Clipboard and the database stay outside this module.

mod components;
mod history_view;
pub mod theme;

pub use history_view::{dismiss_popup, HistoryView};
pub use theme::{POPUP_HEIGHT, POPUP_WIDTH};
