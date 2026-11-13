//! Ingut color, type, spacing, and layout tokens from `docs/DESIGN.md`.
//! Views read these names. They do not embed hex values or one-off sizes.

use gpui_kit::component::{ActiveTheme as _, Theme, ThemeMode};
use gpui_kit::{px, rgb, App, Hsla, Window, WindowAppearance, WindowBackgroundAppearance};

pub const SPACE_2: f32 = 2.;
pub const SPACE_4: f32 = 4.;
pub const SPACE_8: f32 = 8.;
pub const SPACE_12: f32 = 12.;
/// Remaining steps of the 4pt scale. Preferences and later screens use them.
#[allow(dead_code)]
pub const SPACE_6: f32 = 6.;
#[allow(dead_code)]
pub const SPACE_16: f32 = 16.;
#[allow(dead_code)]
pub const SPACE_20: f32 = 20.;
#[allow(dead_code)]
pub const SPACE_24: f32 = 24.;
#[allow(dead_code)]
pub const SPACE_32: f32 = 32.;
#[allow(dead_code)]
pub const SPACE_48: f32 = 48.;

pub const RADIUS_PANEL: f32 = 14.;
pub const RADIUS_CARD: f32 = 12.;
pub const RADIUS_ROW: f32 = 10.;
pub const RADIUS_CHIP: f32 = 6.;

/// Title and callout are part of the type scale. This popup uses headline, body, and caption.
#[allow(dead_code)]
pub const FONT_TITLE: f32 = 20.;
pub const FONT_HEADLINE: f32 = 15.;
pub const FONT_BODY: f32 = 13.;
#[allow(dead_code)]
pub const FONT_CALLOUT: f32 = 12.;
pub const FONT_CAPTION: f32 = 11.;

pub const POPUP_WIDTH: f32 = 640.;
pub const POPUP_HEIGHT: f32 = 420.;
pub const SEARCH_BAR_HEIGHT: f32 = 44.;
pub const ROW_HEIGHT: f32 = 36.;
pub const HINT_BAR_HEIGHT: f32 = 28.;
pub const PREVIEW_INSET: f32 = 8.;
pub const ACCENT_BAR: f32 = 2.;
pub const PREVIEW_MIN_WIDTH: f32 = 520.;
pub const PREVIEW_RATIO: f32 = 0.45;
pub const HAIRLINE: f32 = 1.;
pub const ICON_ROW: f32 = 16.;
pub const ICON_SEARCH: f32 = 14.;
pub const ICON_EMPTY: f32 = 32.;
pub const SEARCH_FIELD_HEIGHT: f32 = 28.;

/// Named colors for the active appearance. Hex lives only in this module.
#[derive(Clone, Copy)]
pub struct Palette {
    pub bg_primary: Hsla,
    pub bg_secondary: Hsla,
    pub bg_elevated: Hsla,
    pub bg_glass: Hsla,
    pub text_primary: Hsla,
    pub text_secondary: Hsla,
    pub text_tertiary: Hsla,
    pub border_hairline: Hsla,
    pub border_glass: Hsla,
    pub accent: Hsla,
    pub accent_soft: Hsla,
    pub success: Hsla,
    pub warning: Hsla,
    pub danger: Hsla,
}

impl Palette {
    pub fn light() -> Self {
        Self {
            bg_primary: paint(0xFBFBFD, 1.),
            bg_secondary: paint(0xF2F2F7, 1.),
            bg_elevated: paint(0xFFFFFF, 1.),
            bg_glass: paint(0xFBFBFD, 0.72),
            text_primary: paint(0x1D1D1F, 1.),
            text_secondary: paint(0x86868B, 1.),
            text_tertiary: paint(0xAEAEB2, 1.),
            border_hairline: paint(0xE5E5EA, 1.),
            border_glass: paint(0xFFFFFF, 0.60),
            accent: paint(0x007AFF, 1.),
            accent_soft: paint(0x007AFF, 0.12),
            success: paint(0x34C759, 1.),
            warning: paint(0xFF9F0A, 1.),
            danger: paint(0xFF3B30, 1.),
        }
    }

    pub fn dark() -> Self {
        Self {
            bg_primary: paint(0x1C1C1E, 1.),
            bg_secondary: paint(0x2C2C2E, 1.),
            bg_elevated: paint(0x3A3A3C, 1.),
            bg_glass: paint(0x1C1C1E, 0.68),
            text_primary: paint(0xF5F5F7, 1.),
            text_secondary: paint(0x98989D, 1.),
            text_tertiary: paint(0x636366, 1.),
            border_hairline: paint(0x38383A, 1.),
            border_glass: paint(0xFFFFFF, 0.08),
            accent: paint(0x0A84FF, 1.),
            accent_soft: paint(0x0A84FF, 0.22),
            success: paint(0x30D158, 1.),
            warning: paint(0xFF9F0A, 1.),
            danger: paint(0xFF453A, 1.),
        }
    }

    pub fn from_appearance(appearance: WindowAppearance) -> Self {
        match appearance {
            WindowAppearance::Dark | WindowAppearance::VibrantDark => Self::dark(),
            WindowAppearance::Light | WindowAppearance::VibrantLight => Self::light(),
        }
    }
}

fn paint(hex: u32, alpha: f32) -> Hsla {
    rgb(hex).alpha(alpha).into()
}

/// Follow the system appearance, then restyle GPUI Kit with these tokens.
pub fn sync(window: &Window, cx: &mut App) {
    let palette = Palette::from_appearance(window.appearance());
    let mode = match window.appearance() {
        WindowAppearance::Dark | WindowAppearance::VibrantDark => ThemeMode::Dark,
        WindowAppearance::Light | WindowAppearance::VibrantLight => ThemeMode::Light,
    };
    if cx.theme().mode != mode {
        Theme::change(mode, None, cx);
    }
    let on_accent = paint(0xFFFFFF, 1.);
    Theme::update(cx, |theme| {
        theme.background = palette.bg_glass;
        theme.foreground = palette.text_primary;
        theme.muted = palette.bg_secondary;
        theme.muted_foreground = palette.text_secondary;
        theme.popover = palette.bg_elevated;
        theme.popover_foreground = palette.text_primary;
        theme.border = palette.border_hairline;
        theme.input = palette.bg_secondary;
        theme.accent = palette.accent;
        theme.accent_foreground = on_accent;
        theme.primary = palette.accent;
        theme.primary_foreground = on_accent;
        theme.list_active = palette.accent_soft;
        theme.list_hover = palette.bg_secondary;
        theme.list_active_border = palette.accent;
        theme.success = palette.success;
        theme.warning = palette.warning;
        theme.danger = palette.danger;
        theme.font_family = ".SystemUIFont".into();
        theme.font_size = px(FONT_BODY);
        theme.radius = px(RADIUS_ROW);
        theme.radius_lg = px(RADIUS_CARD);
        theme.window_radius = px(RADIUS_PANEL);
    });

    // macOS can blur the window. Elsewhere, keep a solid panel so the glass
    // color does not turn into a muddy translucent grey.
    #[cfg(target_os = "macos")]
    window.set_background_appearance(WindowBackgroundAppearance::Blurred);
    #[cfg(not(target_os = "macos"))]
    window.set_background_appearance(WindowBackgroundAppearance::Opaque);
}
