# Ingut Design System

Ingut should feel like a small piece of macOS that Apple forgot to ship: quiet, precise, tactile, and fast.
This document is the **source of truth for every pixel**. Vague taste words are not enough; every rule below is concrete so the result is never generic.

> Mood: *whisper-quiet luxury*. Calm, tactile, functional, direct, distraction-free.
> Reference feel: macOS Sequoia Spotlight, Raycast, visionOS glass panels, Things 3, Linear.

---

## 1. Why UIs come out bland (and how we avoid it)

| Bland default | Ingut rule |
|---|---|
| Flat grey boxes, every item looks the same | Clear layering: glass panel → content surface → selected row, each with its own depth |
| Uniform 16px padding everywhere | Deliberate spacing scale (below) with rhythm: tight inside a row, airy between groups |
| One font size, one weight | 5-step type scale with weight *and* color doing the hierarchy |
| No states, or only hover | Every interactive element has rest, hover, pressed, focused, selected, disabled |
| No motion, or random motion | A small set of spring presets, used consistently |
| Placeholder "Lorem ipsum" lists | Real-feeling content previews: type glyph, relative time, smart truncation |
| Empty screen = blank space | Designed empty, loading, and error states with an icon and one friendly line |
| Borders everywhere | Prefer spacing and tone shifts; use 1px hairlines only as structure |

---

## 2. Design tokens

All colors live in **one theme module** (`src/ui/theme.rs`) as named tokens. **Never hardcode hex values in views.**

### Color
| Token | Light | Dark | Use |
|---|---|---|---|
| `bg.primary` | `#FBFBFD` | `#1C1C1E` | Main content surface |
| `bg.secondary` | `#F2F2F7` | `#2C2C2E` | Sidebar, cards, search field |
| `bg.elevated` | `#FFFFFF` | `#3A3A3C` | Popovers, preview pane, menus |
| `bg.glass` | `#FBFBFD` @ 72% + blur | `#1C1C1E` @ 68% + blur | Main popup panel, sidebar |
| `text.primary` | `#1D1D1F` | `#F5F5F7` | Titles, item text |
| `text.secondary` | `#86868B` | `#98989D` | Metadata, timestamps, hints |
| `text.tertiary` | `#AEAEB2` | `#636366` | Placeholder, disabled |
| `border.hairline` | `#E5E5EA` | `#38383A` | 1px structural lines |
| `border.glass` | `#FFFFFF` @ 60% | `#FFFFFF` @ 8% | Inner top highlight on glass panels |
| `accent` | `#007AFF` | `#0A84FF` | Selection, focus ring, primary actions |
| `accent.soft` | `#007AFF` @ 12% | `#0A84FF` @ 22% | Selected row background |
| `success` | `#34C759` | `#30D158` | "Copied" confirmation |
| `warning` | `#FF9F0A` | `#FF9F0A` | Pinned indicator, warnings |
| `danger` | `#FF3B30` | `#FF453A` | Delete / clear history |

Accent gradient (only for the one primary button and the app icon): `#0A84FF → #5E5CE6` at 135°, subtle.
Follow the system light/dark appearance automatically.

### Spacing scale (4pt grid)
`2, 4, 6, 8, 12, 16, 20, 24, 32, 48`. Nothing else.
- Inside a row: 8–12. Between rows: 2. Between groups: 16–24. Panel padding: 12.
- Keep at least ~30% of the canvas as open space. Never fill every gap.

### Radius
- Panel / window: 14
- Cards, search field, preview pane: 12
- Rows, buttons: 10
- Badges, chips, keycaps: 6
- Nested elements use a smaller radius than their container (container radius − padding).

### Shadow and depth
- Glass panel: `0 20px 50px rgba(0,0,0,0.18)` + `0 2px 6px rgba(0,0,0,0.08)` (dark: double opacity).
- Elevated (preview, menu): `0 8px 24px rgba(0,0,0,0.12)`.
- Rows have **no** shadow, ever.
- Glass panels get a 1px `border.glass` highlight on the top edge so they read as physical glass.

### Typography
System font: **SF Pro** on macOS (the system UI font, e.g. `.SystemUIFont` in GPUI; verify), **Segoe UI Variable** on Windows, **Inter** fallback on Linux. Monospace for code/URL previews: **SF Mono** / **Cascadia Code** / **JetBrains Mono**.

| Style | Size / line height | Weight | Tracking | Use |
|---|---|---|---|---|
| Title | 20 / 24 | Semibold | −0.4 | Window/section titles (Preferences) |
| Headline | 15 / 20 | Semibold | −0.2 | Empty-state titles, card titles |
| Body | 13 / 18 | Regular | 0 | Item text |
| Callout | 12 / 16 | Medium | 0 | Buttons, search text |
| Caption | 11 / 14 | Regular | +0.1 | Timestamps, shortcut hints |

Hierarchy comes from **weight + color**, not from making things bigger. Group headers ("PINNED", "TODAY") use Caption, uppercase, Semibold, `text.secondary`, tracking +0.6.

### Iconography
SF Symbols style: outline, **1.5px stroke**, rounded caps, 16px in rows, 14px in captions. Use the Lucide icons bundled with GPUI Kit (closest match to SF Symbols). One icon style only. Filled icons only for selected/pinned state.

### Motion (spring presets)
| Preset | Spec | Use |
|---|---|---|
| `snappy` | ~180ms, ease-out spring, no overshoot | Hover, selection move |
| `smooth` | ~260ms, gentle spring, ≤2% overshoot | Popup open/close, panel collapse |
| `confirm` | ~350ms, scale 1 → 0.97 → 1 | "Copied" feedback |

- Popup open: fade 0 → 1 + scale 0.96 → 1 + translateY 6 → 0 (`smooth`). Close: reverse, faster (140ms).
- Selection highlight **slides** between rows rather than jumping.
- Respect "reduce motion": crossfade only.
- Filter search results instantly; never animate layout on every keystroke.

---

## 3. Ingut screens

### A. Clipboard popup (the heart of the app)
Summoned by `Cmd+Shift+V`, appears near the cursor (or screen center), **640 × 420**, glass panel, no titlebar.

```
┌──────────────────────────────────────────────────────────────┐
│  ⌕  Search clipboard…                              ⌘⇧V   ⚙  │  ← 44px search bar
├───────────────────────────────┬──────────────────────────────┤
│ PINNED                        │                              │
│ ★ [doc]  Alamat rumah…   ⌘1   │   Preview of selected item   │
│ TODAY                         │   (full text / image / URL   │
│ ▌[link] github.com/zed… 2m ⌘2 │    card)                     │
│   [text] Meeting notes… 5m ⌘3 │                              │
│   [img]  Screenshot…   1h ⌘4  │   Copied 2m ago              │
│ YESTERDAY                     │   142 characters             │
│   …                           │                              │
├───────────────────────────────┴──────────────────────────────┤
│  ↵ Paste   ⌘↵ Copy   ⌘P Pin   ⌫ Delete              24 items │  ← 28px hint bar
└──────────────────────────────────────────────────────────────┘
```
- Split pane: list 55% / preview 45%. Preview pane uses `bg.elevated`, 12 radius, inset 8 from the panel edge. Below 520px width, the preview collapses with `smooth`.
- **Row** (36px tall): type glyph (16px, `text.secondary`) · one-line preview (Body, middle-truncated for URLs and paths) · relative time (Caption) · `⌘1`…`⌘9` keycap chip on the first 9 rows.
  - Rest: transparent. Hover: `bg.secondary`. Selected: `accent.soft` background + 2px accent bar on the left, text stays `text.primary`. Pressed: scale 0.99.
- Content-aware previews: URLs show domain in Semibold + path in secondary; colors (`#hex`) show a swatch; images show a thumbnail; code uses monospace.
- Search field: `bg.secondary`, 12 radius, no border; focus = 2px `accent` ring at 30% opacity. Matched characters in results use `accent` + Semibold.
- Hint bar: Caption, `text.secondary`; keycaps are 6-radius chips with a 1px hairline.
- **Empty state:** centered clipboard icon (32px, `text.tertiary`), "Nothing to remember yet" (Headline), "Copy something and it will show up here." (Caption).
- **No results:** `No matches for "abc"` + hint "Try fewer letters".
- **Copied feedback:** row plays `confirm`, a small green ✓ "Copied" toast fades in at the bottom, popup closes after 250ms.

### B. Preferences window
Native-style window: traffic lights integrated into a 52px toolbar, **collapsible translucent sidebar** (220px, `bg.glass`), content pane on `bg.primary`.
- Sidebar: General, Shortcuts, Privacy, Appearance, About. Selected item = `accent.soft` pill, 10 radius.
- Content: max width 560, top padding 32; settings grouped in `bg.secondary` cards with hairline dividers between rows, like macOS System Settings.
- Toggles, segmented controls and dropdowns from GPUI Kit, restyled with our tokens.

### C. Menu bar / tray
Monochrome 1.5px template icon. Menu: Open Ingut, Pause recording, Clear history…, Preferences…, Quit.

---

## 4. Implementation rules (GPUI Kit)
- Build on GPUI Kit components (Input, virtual list, Button, Switch, theme) and **restyle them with our tokens**; default styling alone is not finished work.
- Tokens live in `src/ui/theme.rs`; register light/dark through GPUI Kit's theme system and follow the system appearance.
- Glass: use the window's blurred/transparent background appearance on macOS (check the current GPUI API for the exact name). Where blur isn't available, fall back to solid `bg.primary`, never a muddy semi-transparent grey.
- History uses a virtual list (it can hold thousands of items).
- Reusable pieces (Row, Keycap, SectionHeader, EmptyState, Toast) live in `src/ui/components/`.

## 5. Never do this
- Hardcoded hex colors or ad-hoc paddings in views.
- Pure black `#000` backgrounds or pure white on pure black.
- Heavy borders, borders on every row, or shadows on rows.
- More than one accent color on screen (status colors only in their own context).
- Center-aligned list text, all-caps body text, emoji as real UI icons.
- Bouncy, slow, or decorative animations.
- Modals for things that can be inline.
- Shipping GPUI Kit's default look unchanged.

## 6. Done checklist for every UI change
- [ ] Uses only tokens from `theme.rs`
- [ ] Looks right in **both** light and dark mode
- [ ] All states designed: rest, hover, pressed, focused, selected, disabled, empty, loading
- [ ] Fully usable with keyboard only, with a visible focus ring
- [ ] Spacing on the 4pt scale; at least ~30% open space
- [ ] Motion uses a named preset and respects reduce-motion
- [ ] Matches the screen spec in section 3; any deviation is explained in the commit
