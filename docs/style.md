# illuc — Frontend Design Document

> **Purpose:** This document captures the complete visual design language of the illuc Tauri desktop application. It is intended to give any agent or developer a thorough understanding of the aesthetic, feel, colour system, typography, and component patterns so that new work stays visually coherent.

---

## 1. Application Identity

illuc is a desktop tool for managing agentic AI coding tasks. Conceptually it sits between a task manager and an IDE companion: structured, calm, and professional, but not sterile. The design avoids the cold greys of generic developer tooling and instead draws on warm, organic tones — like aged paper, terracotta, and weathered wood — to make long sessions feel less fatiguing.

The aesthetic can be summarised in a few words: **warm minimalism with a developer's precision.** It is never flashy, but every detail is considered.

---

## 2. Colour System

All colours are expressed as CSS custom properties on `:root`. The palette is warm-neutral — a desaturated sand/sepia family — with a single earthy terracotta accent used for interactive elements and brand moments.

### 2.1 Surface Palette (Light Theme — default)

The background is not white. It is a warm off-white verging on parchment, with layers that get slightly richer as you go deeper into the UI stack.

| Variable | Value | Description |
|---|---|---|
| `--surfaces-bg` | `#f1ece4` | Root page background (warm sand) |
| `--surfaces-surface` | `#f7f3ec` | Primary surface — cards, sidebar, panels |
| `--surfaces-surface_alt` | `#ebe4da` | Alternate surface — hover targets, tab bars |
| `--surfaces-surface_strong` | `#e6ded3` | Strong surface — inputs, selected states, code bg |
| `--surfaces-code_bg` | `#e4e4e4` | Code block background |

There is no pure white or pure black anywhere in the UI. The entire palette is hand-tinted with warmth.

### 2.2 Text

Text uses a single warm dark brown as the base, then fades via `rgba` opacity for hierarchy. This is important: muted and subtle text are **not grey** — they are the same brown at lower opacity, preserving the warm temperature throughout.

| Variable | Value | Description |
|---|---|---|
| `--text-default` | `#605a52` | Primary text — warm dark brown |
| `--text-muted` | `rgba(96, 90, 82, 0.7)` | Secondary text — labels, metadata |
| `--text-subtle` | `rgba(96, 90, 82, 0.5)` | Tertiary text — placeholders, disabled |
| `--text-link` | `rgba(96, 90, 82, 0.75)` | Link colour at rest |
| `--text-link_hover` | `rgba(96, 90, 82, 0.95)` | Link colour on hover |

### 2.3 Brand Accent

The accent is a warm terracotta — earthy, rich, not aggressive. It is used sparingly: focus rings, active states, send buttons, the brand dot, and blockquote accents. It never overwhelms.

| Variable | Value | Description |
|---|---|---|
| `--brand-accent` | `#b2714a` | Primary terracotta accent |
| `--brand-accent_strong` | `#8f5a3a` | Darker accent — for icons on selected states |
| `--brand-accent_soft` | `rgba(178, 113, 74, 0.2)` | Tinted accent fill — selection bg, focus glow |

### 2.4 Actions

The action colour (used on the primary CTA, send button) is a slightly brighter orange-amber, distinct from the brand accent so primary actions read as interactive:

| Variable | Value | Description |
|---|---|---|
| `--actions-primary` | `#f2994a` | Primary action — buttons, CTAs |
| `--actions-primary_hover` | `#f5a55f` | Hover brightened |
| `--actions-primary_active` | `#e78935` | Active/pressed deepened |
| `--actions-primary_contrast` | `#1f1300` | Text on action buttons (near-black warm) |

### 2.5 Status Colours

Status colours are all desaturated / muted — they do not shout. Each has a "soft" variant (20% opacity) for tinted backgrounds.

| Variable | Value | Description |
|---|---|---|
| `--status-success` | `#6f8f6c` | Muted sage green — working/running state |
| `--status-success_soft` | `rgba(111,143,108,0.2)` | |
| `--status-warning` | `#b18a4a` | Muted amber — awaiting approval |
| `--status-warning_soft` | `rgba(177,138,74,0.2)` | |
| `--status-danger` | `#b06052` | Muted brick red — failed/destructive |
| `--status-danger_soft` | `rgba(176,96,82,0.2)` | |
| `--status-info` | `#6e8aa1` | Muted slate blue — completed/informational |
| `--status-info_soft` | `rgba(110,138,161,0.2)` | |

Note that none of these are pure vivid primaries. Even "danger" is a brick red, not fire-engine red. This keeps the UI calm during stressful moments.

### 2.6 Borders

Borders are extremely subtle — they hint at structure without creating visual noise.

| Variable | Value | Description |
|---|---|---|
| `--borders-default` | `rgba(96, 90, 82, 0.25)` | Standard border — panels, inputs, dividers |
| `--borders-strong` | `rgba(96, 90, 82, 0.4)` | Emphasis border — hover states |

### 2.7 Interaction States

| Variable | Value | Description |
|---|---|---|
| `--interaction-hover` | `rgba(96, 90, 82, 0.08)` | Hover fill — very light wash |
| `--interaction-focus` | `rgba(178, 113, 74, 0.35)` | Focus ring / active tab indicator (accent-tinted) |
| `--interaction-selection_bg` | `rgba(178, 113, 74, 0.2)` | Selected item background (accent-tinted) |

### 2.8 Effects

| Variable | Value | Description |
|---|---|---|
| `--effects-overlay_backdrop` | `rgba(96, 90, 82, 0.25)` | Modal backdrop |
| `--effects-shadow_soft` | `rgba(96, 90, 82, 0.18)` | Standard shadow |
| `--effects-shadow_menu` | `rgba(40, 35, 28, 0.14)` | Menu shadow (slightly deeper warm) |

### 2.9 Scrollbars

Scrollbars are thin (6px), almost invisible at rest, warming on hover:

| Variable | Value |
|---|---|
| `--scrollbar-size` | `6px` |
| `--scrollbar-track` | `rgba(96, 90, 82, 0.08)` |
| `--scrollbar-thumb` | `rgba(96, 90, 82, 0.2)` |
| `--scrollbar-thumb_hover` | `rgba(96, 90, 82, 0.42)` |
| `--scrollbar-thumb_active` | `rgba(96, 90, 82, 0.55)` |

---

## 3. Typography

### 3.1 Font Stack

The UI font is **Inter** with a graceful system-font fallback chain:

```css
font-family: "Inter", "Segoe UI", system-ui, -apple-system, sans-serif;
```

Monospace (terminal output, code, tool invocations, diffs) uses:

```
"JetBrains Mono", "Fira Code", monospace
```

### 3.2 Type Scale

All sizes are relative to the root `--font-size-md` (1rem), making the whole scale themeable:

| Variable | Scale | Typical use |
|---|---|---|
| `--font-size-xs` | `× 0.75` | Labels, badges, metadata, code, tool output |
| `--font-size-sm` | `× 0.85` | Secondary UI text, input labels, status |
| `--font-size-md` | `1rem` | Body text, message content |
| `--font-size-lg` | `× 1.25` | Section headings |
| `--font-size-xl` | `× 1.6` | Page-level headings (dashboard header) |

### 3.3 Weight Usage

- **750** — section headings, count badges headings (very bold, structural)
- **650** — task titles, nav labels, row titles, repo branch names (semi-bold, not heavy)
- **600** — diff file headers
- **normal** — body copy
- No italic except blockquotes and placeholder empty states

### 3.4 Letterspacing Conventions

Certain label types use explicit letter-spacing to distinguish their semantic role:
- **Section eyebrows / category labels**: `letter-spacing: 0.14–0.2rem` + `text-transform: uppercase`
- **Status badges**: `letter-spacing: 0.06rem` + `text-transform: uppercase`
- **Task pills**: `letter-spacing: 0.04rem`

---

## 4. Layout Architecture

### 4.1 Shell

The app is a full-viewport flex column:

```
┌─────────────────────────────────────────────┐
│  .content  (flex-row, flex: 1)              │
│  ┌──────────┬──────────────────────────────┐│
│  │ Sidebar  │ Task Host (flex: 1)          ││
│  │ 360px    │  (stacked task-view layers)  ││
│  └──────────┴──────────────────────────────┘│
│  .status-bar  (footer, fixed height)        │
└─────────────────────────────────────────────┘
```

- Background: `--surfaces-bg`
- `overflow: hidden` everywhere — no page-level scroll
- All scroll happens inside specific scrollable child regions

### 4.2 Sidebar

Fixed width **360px**, non-resizable. It sits left-flush, separated by a single 1px `--borders-default` right border. Background is `--surfaces-surface` (one step lighter than the bg).

The sidebar has a structured top section (repo info + nav items) and a scrollable task list below. Task rows are full-bleed (no horizontal margin), separated by `rgba(96, 90, 82, 0.12)` hairlines.

### 4.3 Task Panels

The main content area uses split panels — chat on one side, terminal/diff/review on the other. Panels are separated by 1px borders. Panel headers have a fixed height of `36px`.

### 4.4 Modals

Modals are centered with a blurred backdrop (`backdrop-filter: blur(3px)`). They are compact (max 420px wide), with `border-radius: 12px`, padding of `1.25rem`, and a soft `--borders-default` border. No heavy drop shadows on modals — they rely on the backdrop.

---

## 5. Component Patterns

### 5.1 Buttons

There are four button archetypes:

**Icon Button (`.icon-btn`)** — circular, 34px, transparent background with a subtle border. Used for toolbar actions. Hover reveals `--surfaces-surface_strong` fill.

**Action Button (`.action-btn`)** — border-only, transparent background, small font. Comes in a default variant and a `warn` variant (brick red border + `--status-danger_soft` hover fill).

**Action Text Button (`.action-text-btn`)** — pill-shaped (999px radius), 34px tall, inline icon + label. Used for composite actions like "Open in editor."

**Send Button (`.send-btn`)** — filled circle, 30px, solid `--brand-accent` fill with white icon. The one filled action button in the UI; very deliberate placement.

All interactive elements use `transition: 0.18s ease` for border/background/color. Hover states are always visible but never dramatic. Disabled state uses `opacity: 0.4–0.55`.

### 5.2 Input Composer

The message input box is a rounded rectangle with `border-radius: 14px`. At rest it has a `--borders-default` border on a `--surfaces-surface_strong` background. On focus, it gets a `--brand-accent`-tinted border and a `box-shadow: 0 0 0 3px --brand-accent_soft` glow. This is the most prominent interactive element in the app.

### 5.3 Chat Messages

- **User messages**: Right-aligned bubble with `border-radius: 16px 16px 4px 16px` — the bottom-right corner is cut to indicate the author side. Background is `--surfaces-surface_strong`.
- **Assistant messages**: Left-aligned, no bubble — text flows freely at full width (up to `75ch`). `line-height: 1.75`.
- **Tool invocations**: Rendered inline in monospace, with colour-coded tokens: binary names in `--text-default`, flags in `--text-subtle`, arguments in `--brand-accent` at 86% opacity, strings in `--status-success`.
- **Code inline**: `--surfaces-surface_alt` background, `--brand-accent` coloured text, `border-radius: 4px`.
- **Code blocks**: `--surfaces-surface_alt` background, `border-radius: 10px`, `14px 16px` padding.
- **Blockquotes**: Left border in `--brand-accent`, `--brand-accent_soft` tinted background, italic text.

### 5.4 Status Indicators

Task status is communicated in two parallel ways:
- A small 8px dot in the sidebar row (coloured by status)
- An uppercase small-text label (`letter-spacing: 0.06rem`) in the same colour

States and their colours:
- **IDLE**: `rgba(96,90,82,0.3)` — subdued, blends with the surface
- **WORKING**: `--status-success` (sage green)
- **AWAITING_APPROVAL**: `--status-warning` (amber)
- **FAILED**: `--status-danger` (brick)
- **COMPLETED**: `--status-info` (slate blue)

### 5.5 Throbber / Loading Animation

Active tool calls use a small pulsing dot (`--brand-accent` at 35% opacity, `border-radius: 999px`, `animation: pulse-single 1.8s ease-in-out infinite`). It is small and unobtrusive.

The typing/streaming indicator uses a shimmer gradient animation — text fades between `--text-muted` and `--text-default` in a 2.4s left-to-right sweep. Elegant, not distracting.

### 5.6 Dropdown Menus

Dropdowns (model selector, editor launcher) appear above their trigger using `bottom: calc(100% + 8px)`. They have:
- `border-radius: 12px`
- `--surfaces-surface_strong` background
- `box-shadow: 0 8px 32px rgba(0,0,0,0.4), 0 2px 8px rgba(0,0,0,0.3)` — this is the one place with more dramatic shadows, ensuring menus float clearly off the warm surface
- Animated: `opacity` + `translateY(6px)` + `scale(0.97)` → open state, `transition: 0.18s ease`
- Option rows: `border-radius: 7px`, hover fills with `--surfaces-surface_alt`, selected fills with `--brand-accent_soft`

### 5.7 Tabs

Tabs are minimal: text buttons with `border-bottom: 2px solid transparent` that fills with `--interaction-focus` when active. No pill shape, no background fill. The active tab also switches from `--text-muted` to `--text-default` colour.

### 5.8 Chips / Pills

Used for task labels and model selectors. Two styles:
- **Outline pill**: `border: 1px solid rgba(96,90,82,0.14)`, transparent background, `border-radius: 999px`. Very understated.
- **Chip**: `--components-chip_bg` fill (`rgba(96,90,82,0.15)`), no border, `border-radius: 999px`. Used for status/category tags.

### 5.9 Diff Viewer

The diff panel uses the monospace font stack at `--font-size-xs`. Row height is exactly `28px`. The file list sidebar is 248px wide. Diff syntax follows the standard red/green convention using `--status-danger` and `--status-success`.

---

## 6. Motion & Animation

The UI is calm. Animations are:
- **Transitions**: almost universally `0.18s ease` — fast enough to feel responsive, slow enough to feel deliberate
- **Scale transforms on hover**: small (1.04–1.06×), used on send/stop buttons and toggle icons only
- **Open/close menus**: combined `opacity`, `translateY`, and `scale` transitions — menus feel physical
- **Throbber pulse**: `1.8s ease-in-out infinite` — gentle breathing rhythm
- **Shimmer sweep**: `2.4s linear infinite` — streaming text animation

No bounce, no spring physics, no dramatic entrances. Everything settles quietly.

---

## 7. Theming Architecture

The application supports runtime theme overrides. At startup, `ThemeService` calls a Tauri backend command (`settings_theme_get`) which returns a flat key-value map. Keys use dot notation (e.g., `surfaces.bg`) which gets converted to CSS custom properties (`--surfaces-bg`) and set on `document.documentElement.style`. This means any variable in the theme can be overridden from settings.

Syntax highlighting has a separate `data-syntax-theme` attribute on `:root` which toggles between `light` (GitHub light theme, default) and `dark` (GitHub dark theme).

The theme fires a custom `illuc-theme-applied` event on `window` after each apply, so consumers like xterm.js can reread computed variables.

---

## 8. Design Principles Summary

1. **Warmth over neutrality.** No cool greys. Every surface, text colour, and border tint inherits from the warm brown base.
2. **Hierarchy through opacity, not colour.** Muted and subtle text are the same hue as default text, just at lower opacity. This keeps the palette coherent.
3. **One accent, used sparingly.** Terracotta (`#b2714a`) is the only brand colour. It appears on focus states, the send button, selected states, and inline code. Not on headings, not on backgrounds.
4. **Status colours are soft.** Sage green, amber, brick red, and slate blue — never vivid primaries. They inform without alarming.
5. **Monospace for data, Inter for everything else.** Tool invocations, terminal output, code, and diffs use JetBrains Mono. All UI chrome uses Inter.
6. **Borders are hints, not walls.** 1px `rgba` borders at 25% opacity, sometimes as low as 12% for interior dividers.
7. **Motion is always `0.18s ease`.** Consistent, no exceptions except the breathing animations.
8. **No shadows on surfaces, only on floating elements.** Panels and cards rely on borders and background-colour layering. Dropdowns and menus use shadows. Modals rely on backdrop.
