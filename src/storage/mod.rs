//! Clipboard history stored in SQLite under the application data folder.
//! Pinned rows are kept outside the history cap.

mod search;

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use rusqlite::{params, Connection};

pub use search::filter_clips;

/// How many unpinned items to keep. Pinned rows are not counted.
pub const HISTORY_LIMIT: usize = 200;

#[derive(Clone, Debug)]
pub enum ClipContent {
    Text(String),
    Image {
        width: u32,
        height: u32,
        kind: String,
    },
}

#[derive(Clone, Debug)]
pub struct ClipItem {
    pub id: i64,
    pub content: ClipContent,
    /// Pinned rows stay above the rest and are not removed by the history cap.
    pub pinned: bool,
    /// Unix time in milliseconds, used for relative timestamps in the list.
    pub created_at_ms: i64,
}

impl ClipItem {
    #[cfg(test)]
    pub fn text(id: i64, text: impl Into<String>) -> Self {
        Self {
            id,
            content: ClipContent::Text(text.into()),
            pinned: false,
            created_at_ms: 0,
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match &self.content {
            ClipContent::Text(text) => Some(text),
            ClipContent::Image { .. } => None,
        }
    }

    /// Text used by search. Images match words like "image" plus their size.
    pub fn search_text(&self) -> String {
        match &self.content {
            ClipContent::Text(text) => text.clone(),
            ClipContent::Image { width, height, .. } => format!("image {width} {height}"),
        }
    }

    /// One-line preview. Newlines and repeated spaces collapse so every row stays one line tall.
    pub fn preview(&self) -> String {
        match &self.content {
            ClipContent::Text(text) => flatten_preview(text),
            ClipContent::Image { width, height, .. } => format!("Image {width}×{height}"),
        }
    }
}

/// Character cap for a preview. The row still ellipsizes sooner when the window is narrower.
const PREVIEW_CHARS: usize = 120;

fn flatten_preview(text: &str) -> String {
    let mut out = String::new();
    let mut count = 0usize;
    let mut pending_space = false;
    let mut truncated = false;

    for ch in text.chars() {
        if ch.is_whitespace() {
            if count > 0 {
                pending_space = true;
            }
            continue;
        }
        if pending_space {
            if count >= PREVIEW_CHARS {
                truncated = true;
                break;
            }
            out.push(' ');
            count += 1;
            pending_space = false;
        }
        if count >= PREVIEW_CHARS {
            truncated = true;
            break;
        }
        out.push(ch);
        count += 1;
    }

    if truncated {
        out.push('…');
    }
    out
}

pub struct SqliteHistory {
    conn: Connection,
}

impl SqliteHistory {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS clips (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                text TEXT NOT NULL,
                pinned INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                image BLOB,
                width INTEGER,
                height INTEGER,
                image_kind TEXT
            );",
        )?;
        ensure_image_columns(&conn)?;
        Ok(Self { conn })
    }

    pub fn open_default() -> Result<Self> {
        let dirs = ProjectDirs::from("app", "Ingut", "Ingut")
            .context("application data folder is not available")?;
        Self::open(database_path(&dirs))
    }

    pub fn insert(&mut self, text: &str, pinned: bool) -> Result<ClipItem> {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis() as i64)
            .unwrap_or(0);
        self.conn.execute(
            "INSERT INTO clips (text, pinned, created_at) VALUES (?1, ?2, ?3)",
            params![text, if pinned { 1 } else { 0 }, created_at],
        )?;
        let id = self.conn.last_insert_rowid();
        self.trim_unpinned()?;
        Ok(ClipItem {
            id,
            content: ClipContent::Text(text.to_owned()),
            pinned,
            created_at_ms: created_at,
        })
    }

    pub fn insert_text(&mut self, text: &str) -> Result<ClipItem> {
        self.insert(text, false)
    }

    pub fn insert_image(
        &mut self,
        bytes: Vec<u8>,
        width: u32,
        height: u32,
        kind: &str,
    ) -> Result<ClipItem> {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis() as i64)
            .unwrap_or(0);
        self.conn.execute(
            "INSERT INTO clips (text, pinned, created_at, image, width, height, image_kind)
             VALUES ('', 0, ?1, ?2, ?3, ?4, ?5)",
            params![created_at, bytes, width, height, kind],
        )?;
        let id = self.conn.last_insert_rowid();
        self.trim_unpinned()?;
        Ok(ClipItem {
            id,
            content: ClipContent::Image {
                width,
                height,
                kind: kind.to_owned(),
            },
            pinned: false,
            created_at_ms: created_at,
        })
    }

    /// Full image bytes for one row. The history list does not load these.
    pub fn image_bytes(&self, id: i64) -> Result<Option<Vec<u8>>> {
        match self.conn.query_row(
            "SELECT image FROM clips WHERE id = ?1",
            params![id],
            |row| row.get::<_, Option<Vec<u8>>>(0),
        ) {
            Ok(Some(bytes)) if !bytes.is_empty() => Ok(Some(bytes)),
            Ok(_) => Ok(None),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(err).context("failed to read a stored image"),
        }
    }

    pub fn set_pinned(&mut self, id: i64, pinned: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE clips SET pinned = ?1 WHERE id = ?2",
            params![if pinned { 1 } else { 0 }, id],
        )?;
        Ok(())
    }

    /// Pinned rows first, then newest.
    pub fn list(&self) -> Result<Vec<ClipItem>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, text, pinned, created_at, width, height, image_kind,
                    (image IS NOT NULL AND length(image) > 0)
             FROM clips
             ORDER BY pinned DESC, id DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let text: String = row.get(1)?;
            let pinned: i64 = row.get(2)?;
            let created_at_ms: i64 = row.get(3)?;
            let width: Option<i64> = row.get(4)?;
            let height: Option<i64> = row.get(5)?;
            let kind: Option<String> = row.get(6)?;
            let has_image: i64 = row.get(7)?;
            let content = if has_image != 0 {
                ClipContent::Image {
                    width: width.unwrap_or(0) as u32,
                    height: height.unwrap_or(0) as u32,
                    kind: kind.unwrap_or_else(|| "png".to_owned()),
                }
            } else {
                ClipContent::Text(text)
            };
            Ok(ClipItem {
                id,
                content,
                pinned: pinned != 0,
                created_at_ms,
            })
        })?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    fn trim_unpinned(&mut self) -> Result<()> {
        self.conn.execute(
            "DELETE FROM clips
             WHERE pinned = 0
               AND id NOT IN (
                   SELECT id FROM clips WHERE pinned = 0 ORDER BY id DESC LIMIT ?1
               )",
            params![HISTORY_LIMIT as i64],
        )?;
        Ok(())
    }
}

fn ensure_image_columns(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare("PRAGMA table_info(clips)")?;
    let names = stmt.query_map([], |row| row.get::<_, String>(1))?;
    let mut has_image = false;
    let mut has_kind = false;
    for name in names {
        match name?.as_str() {
            "image" => has_image = true,
            "image_kind" => has_kind = true,
            _ => {}
        }
    }
    if !has_image {
        conn.execute_batch(
            "ALTER TABLE clips ADD COLUMN image BLOB;
             ALTER TABLE clips ADD COLUMN width INTEGER;
             ALTER TABLE clips ADD COLUMN height INTEGER;",
        )?;
    }
    if !has_kind {
        conn.execute("ALTER TABLE clips ADD COLUMN image_kind TEXT", [])?;
    }
    Ok(())
}

fn database_path(dirs: &ProjectDirs) -> PathBuf {
    dirs.data_dir().join("history.db")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store(name: &str) -> SqliteHistory {
        let path = std::env::temp_dir().join(format!("ingut-{name}-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        SqliteHistory::open(&path).unwrap()
    }

    #[test]
    fn preview_is_one_line() {
        let item = ClipItem::text(1, "baris satu\n\nbaris   dua\t tiga");
        assert_eq!(item.preview(), "baris satu baris dua tiga");
        assert!(!item.preview().contains('\n'));
    }

    #[test]
    fn preview_caps_a_long_line() {
        let item = ClipItem::text(1, "x".repeat(400));
        let preview = item.preview();
        assert!(preview.ends_with('…'));
        assert_eq!(preview.chars().count(), PREVIEW_CHARS + 1);
    }

    #[test]
    fn pinned_item_sorts_first() {
        let mut store = temp_store("pin-order");
        let older = store.insert_text("older").unwrap();
        store.insert_text("newer").unwrap();
        store.set_pinned(older.id, true).unwrap();
        let items = store.list().unwrap();
        assert_eq!(items[0].id, older.id);
        assert!(items[0].pinned);
        assert!(!items[1].pinned);
    }

    #[test]
    fn image_roundtrip() {
        let mut store = temp_store("image");
        let saved = store.insert_image(vec![1, 2, 3, 4], 2, 2, "png").unwrap();
        let items = store.list().unwrap();
        match &items[0].content {
            ClipContent::Image {
                width,
                height,
                kind,
            } => {
                assert_eq!(*width, 2);
                assert_eq!(*height, 2);
                assert_eq!(kind, "png");
            }
            ClipContent::Text(_) => panic!("expected an image row"),
        }
        assert_eq!(
            store.image_bytes(saved.id).unwrap().unwrap(),
            vec![1, 2, 3, 4]
        );
        assert_eq!(saved.preview(), "Image 2×2");
        assert_eq!(items[0].search_text(), "image 2 2");
    }

    #[test]
    fn insert_reload_and_newest_first() {
        let path = std::env::temp_dir().join(format!("ingut-order-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);

        let mut store = SqliteHistory::open(&path).unwrap();
        store.insert_text("pertama").unwrap();
        store.insert_text("kedua").unwrap();
        drop(store);

        let store = SqliteHistory::open(&path).unwrap();
        let items = store.list().unwrap();
        assert_eq!(items[0].as_text(), Some("kedua"));
        assert_eq!(items[1].as_text(), Some("pertama"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn two_hundred_first_unpinned_item_drops_the_oldest() {
        let mut store = temp_store("cap");
        let mut oldest = None;
        for n in 0..=HISTORY_LIMIT {
            let item = store.insert_text(&format!("item-{n}")).unwrap();
            if oldest.is_none() {
                oldest = Some(item.id);
            }
        }

        let items = store.list().unwrap();
        assert_eq!(items.len(), HISTORY_LIMIT);
        assert!(items.iter().all(|item| item.id != oldest.unwrap()));
        assert_eq!(
            items[0].as_text(),
            Some(format!("item-{HISTORY_LIMIT}").as_str())
        );
    }

    #[test]
    fn pinned_rows_stay_when_unpinned_history_is_capped() {
        let mut store = temp_store("pinned");
        let pinned = store.insert("simpan", true).unwrap();
        for n in 0..=HISTORY_LIMIT {
            store.insert_text(&format!("bebas-{n}")).unwrap();
        }

        let items = store.list().unwrap();
        assert_eq!(items.len(), HISTORY_LIMIT + 1);
        assert!(items.iter().any(|item| item.id == pinned.id && item.pinned));
        assert_eq!(
            items.iter().filter(|item| !item.pinned).count(),
            HISTORY_LIMIT
        );
    }

    #[test]
    fn older_database_gains_an_image_kind_column() {
        let path = std::env::temp_dir().join(format!("ingut-migrate-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE clips (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                text TEXT NOT NULL,
                pinned INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                image BLOB,
                width INTEGER,
                height INTEGER
            );",
        )
        .unwrap();
        drop(conn);

        let mut store = SqliteHistory::open(&path).unwrap();
        store.insert_image(vec![9], 1, 1, "jpeg").unwrap();
        match &store.list().unwrap()[0].content {
            ClipContent::Image { kind, .. } => assert_eq!(kind, "jpeg"),
            ClipContent::Text(_) => panic!("stored image was read back as text"),
        }
        drop(store);
        let _ = std::fs::remove_file(&path);
    }
}
