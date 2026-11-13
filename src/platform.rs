//! Cursor position for placing the popup. GPUI coordinates use a top-left origin.

/// Cursor in local screen coordinates (top-left), plus that screen's size.
pub struct CursorAnchor {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub primary: bool,
}

#[cfg(target_os = "macos")]
pub fn cursor_anchor() -> Option<CursorAnchor> {
    use core_graphics::display::CGDisplay;
    use core_graphics::event::CGEvent;
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState).ok()?;
    let event = CGEvent::new(source).ok()?;
    let loc = event.location();
    let main = CGDisplay::main().id;
    let ids = CGDisplay::active_displays().ok()?;

    for id in ids {
        let display = CGDisplay::new(id);
        let bounds = display.bounds();
        let right = bounds.origin.x + bounds.size.width;
        let bottom = bounds.origin.y + bounds.size.height;
        if loc.x < bounds.origin.x || loc.x >= right || loc.y < bounds.origin.y || loc.y >= bottom {
            continue;
        }
        return Some(CursorAnchor {
            x: (loc.x - bounds.origin.x) as f32,
            y: (loc.y - bounds.origin.y) as f32,
            width: bounds.size.width as f32,
            height: bounds.size.height as f32,
            primary: id == main,
        });
    }
    None
}

#[cfg(not(target_os = "macos"))]
pub fn cursor_anchor() -> Option<CursorAnchor> {
    None
}

#[cfg(target_os = "macos")]
static PREVIOUS_PID: std::sync::Mutex<Option<i32>> = std::sync::Mutex::new(None);

/// Remember the app that is active before Ingut takes the keyboard.
pub fn remember_frontmost() {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::NSWorkspace;

        let Some(app) = NSWorkspace::sharedWorkspace().frontmostApplication() else {
            return;
        };
        let pid = app.processIdentifier();
        if pid as u32 == std::process::id() {
            return;
        }
        if let Ok(mut slot) = PREVIOUS_PID.lock() {
            *slot = Some(pid);
        }
    }
}

/// Give the keyboard back to the app that was active before the popup opened.
/// Returns false when that app could not be activated.
pub fn restore_frontmost() -> bool {
    #[cfg(target_os = "macos")]
    {
        use objc2::MainThreadMarker;
        use objc2_app_kit::{NSApplication, NSApplicationActivationOptions, NSRunningApplication};

        let pid = PREVIOUS_PID.lock().ok().and_then(|mut slot| slot.take());
        let Some(pid) = pid else {
            return false;
        };
        let Some(marker) = MainThreadMarker::new() else {
            return false;
        };
        let Some(previous) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid)
        else {
            return false;
        };
        let app = NSApplication::sharedApplication(marker);
        app.yieldActivationToApplication(&previous);
        previous.activateFromApplication_options(
            &NSRunningApplication::currentApplication(),
            NSApplicationActivationOptions::empty(),
        )
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

/// Popups use a utility-window animation that takes a noticeable fraction of a second.
pub fn disable_popup_animation() {
    #[cfg(target_os = "macos")]
    {
        use objc2::MainThreadMarker;
        use objc2_app_kit::{NSApplication, NSWindowAnimationBehavior};

        let Some(marker) = MainThreadMarker::new() else {
            return;
        };
        let Some(window) = NSApplication::sharedApplication(marker).keyWindow() else {
            return;
        };
        window.setAnimationBehavior(NSWindowAnimationBehavior::None);
    }
}

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "freebsd"))]
const ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

/// Dock icon on macOS. The mark already includes its card and shadow, so the
/// PNG is transparent around it.
pub fn install_app_icon() {
    #[cfg(target_os = "macos")]
    {
        use objc2::AllocAnyThread;
        use objc2::MainThreadMarker;
        use objc2_app_kit::{NSApplication, NSImage};
        use objc2_foundation::NSData;

        let Some(marker) = MainThreadMarker::new() else {
            return;
        };
        let data =
            unsafe { NSData::dataWithBytes_length(ICON_PNG.as_ptr().cast(), ICON_PNG.len()) };
        let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) else {
            return;
        };
        unsafe {
            NSApplication::sharedApplication(marker).setApplicationIconImage(Some(&image));
        }
    }
}

/// Window icon for X11. Other platforms use the process icon instead.
pub fn window_icon() -> Option<std::sync::Arc<image::RgbaImage>> {
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    {
        static ICON: std::sync::OnceLock<Option<std::sync::Arc<image::RgbaImage>>> =
            std::sync::OnceLock::new();
        ICON.get_or_init(|| {
            image::load_from_memory(ICON_PNG)
                .ok()
                .map(|image| std::sync::Arc::new(image.into_rgba8()))
        })
        .clone()
    }
    #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
    {
        None
    }
}
