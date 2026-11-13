//! Reusable popup pieces. They take a [`Palette`] and do not invent colors.

mod empty;
mod keycap;
mod row;

pub use empty::{empty_state, no_results};
pub use keycap::keycap;
pub use row::{classify, relative_time, section_label, HistoryRow};
