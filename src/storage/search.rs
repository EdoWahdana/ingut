//! Fuzzy search over history text. This module does not know about the UI.

use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

use super::ClipItem;

/// An empty query keeps the original order. Otherwise the highest score comes
/// first, and equal scores keep the original order (newest first).
pub fn filter_clips(items: &[ClipItem], query: &str) -> Vec<ClipItem> {
    let query = query.trim();
    if query.is_empty() {
        return items.to_vec();
    }

    let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);
    let mut matcher = Matcher::new(Config::DEFAULT);
    let mut buf = Vec::new();
    let mut ranked = Vec::new();

    for (ix, item) in items.iter().enumerate() {
        let text = item.search_text();
        buf.clear();
        if let Some(score) = pattern.score(Utf32Str::new(&text, &mut buf), &mut matcher) {
            ranked.push((score, ix));
        }
    }

    ranked.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    ranked
        .into_iter()
        .map(|(_, ix)| items[ix].clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: i64, text: &str) -> ClipItem {
        ClipItem::text(id, text)
    }

    #[test]
    fn empty_query_keeps_order() {
        let items = vec![item(2, "kedua"), item(1, "pertama")];
        let filtered = filter_clips(&items, "  ");
        assert_eq!(filtered[0].id, 2);
        assert_eq!(filtered[1].id, 1);
    }

    #[test]
    fn query_drops_non_matches() {
        let items = vec![item(1, "hello"), item(2, "world")];
        assert!(filter_clips(&items, "zzz").is_empty());
    }

    #[test]
    fn better_match_ranks_first() {
        let items = vec![item(1, "abc"), item(2, "alphabet"), item(3, "zzz")];
        let filtered = filter_clips(&items, "alph");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, 2);
    }
}
