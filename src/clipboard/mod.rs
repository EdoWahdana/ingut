//! Cross-platform clipboard access. OS-specific code stays behind this trait.

use std::hash::{Hash, Hasher};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use anyhow::{Context, Result};

/// How often to check whether the clipboard changed. macOS exposes a change
/// counter, so this wait does not decode an image that is still the same.
pub const POLL_INTERVAL: Duration = Duration::from_millis(400);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageKind {
    Png,
    Jpeg,
    Gif,
    Webp,
}

impl ImageKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpeg",
            Self::Gif => "gif",
            Self::Webp => "webp",
        }
    }
}

/// One clipboard snapshot, already encoded for storage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClipRead {
    Text(String),
    Image {
        bytes: Vec<u8>,
        width: u32,
        height: u32,
        kind: ImageKind,
    },
}

impl ClipRead {
    pub fn fingerprint(&self) -> String {
        match self {
            Self::Text(text) => text.clone(),
            Self::Image { bytes, .. } => image_fingerprint(bytes),
        }
    }
}

/// Clipboard source. The default implementation uses `arboard`.
pub trait ClipboardSource {
    fn read_text(&mut self) -> Result<Option<String>>;
    fn write_text(&mut self, text: &str) -> Result<()>;
    fn read_image(&mut self) -> Result<Option<ClipRead>> {
        Ok(None)
    }
    fn write_image(&mut self, _png: &[u8], _width: u32, _height: u32) -> Result<()> {
        Ok(())
    }

    /// A token that changes only when the clipboard contents change.
    /// `None` means the platform cannot tell, so every poll reads the contents.
    fn generation(&mut self) -> Option<i64> {
        None
    }
}

pub struct SystemClipboard {
    inner: arboard::Clipboard,
}

impl SystemClipboard {
    pub fn new() -> Result<Self> {
        Ok(Self {
            inner: arboard::Clipboard::new()?,
        })
    }
}

impl ClipboardSource for SystemClipboard {
    fn read_text(&mut self) -> Result<Option<String>> {
        Ok(self.inner.get_text().ok())
    }

    fn write_text(&mut self, text: &str) -> Result<()> {
        self.inner.set_text(text.to_owned())?;
        Ok(())
    }

    fn read_image(&mut self) -> Result<Option<ClipRead>> {
        if let Some(from_file) = image_from_copied_files(&mut self.inner)? {
            return Ok(Some(from_file));
        }
        let Ok(image) = self.inner.get_image() else {
            return Ok(None);
        };
        if image.width == 0 || image.height == 0 || image.bytes.is_empty() {
            return Ok(None);
        }
        let bytes = encode_png(image.width, image.height, &image.bytes)?;
        Ok(Some(ClipRead::Image {
            bytes,
            width: image.width as u32,
            height: image.height as u32,
            kind: ImageKind::Png,
        }))
    }

    fn generation(&mut self) -> Option<i64> {
        #[cfg(target_os = "macos")]
        {
            use objc2_app_kit::NSPasteboard;
            Some(NSPasteboard::generalPasteboard().changeCount() as i64)
        }
        #[cfg(not(target_os = "macos"))]
        {
            None
        }
    }

    fn write_image(&mut self, png: &[u8], width: u32, height: u32) -> Result<()> {
        let rgba = decode_png(png)?;
        self.inner.set_image(arboard::ImageData {
            width: width as usize,
            height: height as usize,
            bytes: rgba.into(),
        })?;
        Ok(())
    }
}

/// Write an image back to the clipboard and ignore the next identical read.
pub fn copy_image(png: &[u8], width: u32, height: u32) -> Result<()> {
    let _gate = clipboard_gate();
    suppress_echo(&image_fingerprint(png));
    let mut clipboard = SystemClipboard::new()?;
    if let Err(err) = clipboard.write_image(png, width, height) {
        clear_echo();
        return Err(err);
    }
    Ok(())
}

/// Write text to the clipboard and ignore the next identical read, so our own
/// copy is not stored as a new history item.
pub fn copy_text(text: &str) -> Result<()> {
    let _gate = clipboard_gate();
    suppress_echo(text);
    let mut clipboard = SystemClipboard::new()?;
    if let Err(err) = clipboard.write_text(text) {
        clear_echo();
        return Err(err);
    }
    Ok(())
}

fn clipboard_gate() -> std::sync::MutexGuard<'static, ()> {
    static GATE: Mutex<()> = Mutex::new(());
    GATE.lock().unwrap_or_else(|err| err.into_inner())
}

fn echo_slot() -> &'static Mutex<Option<String>> {
    static ECHO: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    ECHO.get_or_init(|| Mutex::new(None))
}

fn suppress_echo(text: &str) {
    if let Ok(mut slot) = echo_slot().lock() {
        *slot = Some(text.to_owned());
    }
}

fn clear_echo() {
    if let Ok(mut slot) = echo_slot().lock() {
        *slot = None;
    }
}

/// True once, when this text is the copy we just wrote ourselves.
fn take_echo(text: &str) -> bool {
    let Ok(mut slot) = echo_slot().lock() else {
        return false;
    };
    if slot.as_deref() == Some(text) {
        *slot = None;
        true
    } else {
        false
    }
}

/// Polls the clipboard and drops consecutive duplicates.
pub struct Watcher<S> {
    source: S,
    last: Option<String>,
    generation: Option<i64>,
}

impl<S: ClipboardSource> Watcher<S> {
    pub fn new(source: S) -> Self {
        Self {
            source,
            last: None,
            generation: None,
        }
    }

    /// Remember the current text without treating it as a new change.
    /// Used at startup so the stored clipboard is not inserted twice.
    pub fn prime(&mut self, last: Option<String>) {
        self.last = last.filter(|text| !text.is_empty());
    }

    /// `Some` only when the clipboard differs from the previous read.
    /// An image file or bitmap wins over a filename string Finder also places on the pasteboard.
    pub fn poll(&mut self) -> Result<Option<ClipRead>> {
        let _gate = clipboard_gate();
        let generation = self.source.generation();
        if generation.is_some() && generation == self.generation {
            return Ok(None);
        }
        let read = self.read_changed()?;
        self.generation = generation;
        Ok(read)
    }

    fn read_changed(&mut self) -> Result<Option<ClipRead>> {
        if let Some(image) = self.source.read_image()? {
            return self.accept(image);
        }
        if let Some(text) = self.source.read_text()? {
            if !text.is_empty() {
                return self.accept(ClipRead::Text(text));
            }
        }
        self.last = None;
        Ok(None)
    }

    fn accept(&mut self, read: ClipRead) -> Result<Option<ClipRead>> {
        let fingerprint = read.fingerprint();
        if self.last.as_deref() == Some(fingerprint.as_str()) {
            return Ok(None);
        }
        if take_echo(&fingerprint) {
            self.last = Some(fingerprint);
            return Ok(None);
        }
        self.last = Some(fingerprint);
        Ok(Some(read))
    }
}

pub fn image_fingerprint(png: &[u8]) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    png.hash(&mut hasher);
    format!("image:{}", hasher.finish())
}

fn encode_png(width: usize, height: usize, rgba: &[u8]) -> Result<Vec<u8>> {
    let image = image::RgbaImage::from_raw(width as u32, height as u32, rgba.to_vec())
        .context("clipboard image size does not match its bytes")?;
    let mut png = Vec::new();
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .context("failed to encode clipboard image")?;
    Ok(png)
}

fn decode_png(png: &[u8]) -> Result<Vec<u8>> {
    let image = image::load_from_memory(png).context("failed to decode stored image")?;
    Ok(image.to_rgba8().into_raw())
}

fn image_from_copied_files(clipboard: &mut arboard::Clipboard) -> Result<Option<ClipRead>> {
    let paths = match clipboard.get().file_list() {
        Ok(paths) => paths,
        Err(_) => return Ok(None),
    };
    for path in paths.into_iter().take(8) {
        if let Some(image) = load_image_file(&path) {
            return Ok(Some(image));
        }
    }
    Ok(None)
}

fn load_image_file(path: &std::path::Path) -> Option<ClipRead> {
    let len = std::fs::metadata(path).ok()?.len();
    if len == 0 || len > 40 * 1024 * 1024 {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    let format = image::guess_format(&bytes).ok()?;
    let kind = match format {
        image::ImageFormat::Png => ImageKind::Png,
        image::ImageFormat::Jpeg => ImageKind::Jpeg,
        image::ImageFormat::Gif => ImageKind::Gif,
        image::ImageFormat::WebP => ImageKind::Webp,
        _ => return None,
    };
    let (width, height) =
        image::ImageReader::with_format(std::io::Cursor::new(bytes.as_slice()), format)
            .into_dimensions()
            .ok()?;
    Some(ClipRead::Image {
        width,
        height,
        bytes,
        kind,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;

    struct Scripted {
        reads: VecDeque<Option<String>>,
    }

    impl Scripted {
        fn new(reads: impl IntoIterator<Item = Option<String>>) -> Self {
            Self {
                reads: reads.into_iter().collect(),
            }
        }
    }

    impl ClipboardSource for Scripted {
        fn read_text(&mut self) -> Result<Option<String>> {
            Ok(self.reads.pop_front().flatten())
        }

        fn write_text(&mut self, _text: &str) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn consecutive_duplicate_emits_once() {
        let mut watcher = Watcher::new(Scripted::new([
            Some("alpha".into()),
            Some("alpha".into()),
            Some("beta".into()),
        ]));

        assert_eq!(text_read(watcher.poll()), Some("alpha".into()));
        assert_eq!(text_read(watcher.poll()), None);
        assert_eq!(text_read(watcher.poll()), Some("beta".into()));
    }

    #[test]
    fn empty_clipboard_allows_the_same_text_again() {
        let mut watcher = Watcher::new(Scripted::new([
            Some("alpha".into()),
            None,
            Some("alpha".into()),
        ]));

        assert_eq!(text_read(watcher.poll()), Some("alpha".into()));
        assert_eq!(text_read(watcher.poll()), None);
        assert_eq!(text_read(watcher.poll()), Some("alpha".into()));
    }

    #[test]
    fn primed_text_is_not_emitted() {
        let mut watcher = Watcher::new(Scripted::new([Some("alpha".into()), Some("beta".into())]));
        watcher.prime(Some("alpha".into()));

        assert_eq!(text_read(watcher.poll()), None);
        assert_eq!(text_read(watcher.poll()), Some("beta".into()));
    }

    #[test]
    fn echoed_write_is_not_emitted() {
        let mut watcher = Watcher::new(Scripted::new([Some("salin".into())]));
        suppress_echo("salin");

        assert_eq!(text_read(watcher.poll()), None);
    }

    struct Counted {
        generation: std::sync::Arc<std::sync::atomic::AtomicI64>,
        reads: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    }

    impl ClipboardSource for Counted {
        fn read_text(&mut self) -> Result<Option<String>> {
            self.reads.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(Some("same".into()))
        }

        fn write_text(&mut self, _text: &str) -> Result<()> {
            Ok(())
        }

        fn generation(&mut self) -> Option<i64> {
            Some(self.generation.load(std::sync::atomic::Ordering::SeqCst))
        }
    }

    #[test]
    fn unchanged_generation_does_not_read_again() {
        use std::sync::atomic::Ordering;
        use std::sync::Arc;

        let generation = Arc::new(std::sync::atomic::AtomicI64::new(1));
        let reads = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut watcher = Watcher::new(Counted {
            generation: Arc::clone(&generation),
            reads: Arc::clone(&reads),
        });

        assert_eq!(text_read(watcher.poll()), Some("same".into()));
        assert_eq!(text_read(watcher.poll()), None);
        assert_eq!(reads.load(Ordering::SeqCst), 1);

        generation.store(2, Ordering::SeqCst);
        assert_eq!(text_read(watcher.poll()), None);
        assert_eq!(reads.load(Ordering::SeqCst), 2);
    }

    struct ImageAndName;

    impl ClipboardSource for ImageAndName {
        fn read_text(&mut self) -> Result<Option<String>> {
            Ok(Some("photo.jpg".into()))
        }

        fn write_text(&mut self, _text: &str) -> Result<()> {
            Ok(())
        }

        fn read_image(&mut self) -> Result<Option<ClipRead>> {
            Ok(Some(ClipRead::Image {
                bytes: vec![1, 2, 3],
                width: 4,
                height: 5,
                kind: ImageKind::Jpeg,
            }))
        }
    }

    #[test]
    fn image_wins_over_a_filename() {
        let mut watcher = Watcher::new(ImageAndName);
        match watcher.poll().unwrap() {
            Some(ClipRead::Image {
                kind,
                width,
                height,
                ..
            }) => {
                assert_eq!(kind, ImageKind::Jpeg);
                assert_eq!(width, 4);
                assert_eq!(height, 5);
            }
            other => panic!("expected an image, got {other:?}"),
        }
    }

    fn text_read(read: Result<Option<ClipRead>>) -> Option<String> {
        match read.unwrap() {
            Some(ClipRead::Text(text)) => Some(text),
            _ => None,
        }
    }
}
