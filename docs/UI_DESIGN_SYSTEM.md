# OpenWand UI Design System

Wave 52A — Design System Foundation.

## Token Naming Convention

All tokens live in `crates/app/src/ui/design_tokens.rs` as `pub const &str`.

| Module | Prefix | Examples |
|--------|--------|---------|
| `colors` | Semantic name | `PRIMARY`, `SUCCESS_BG`, `TEXT_MUTED` |
| `typo` | `FONT_` or `TEXT_` | `FONT_BODY`, `TEXT_SM`, `TEXT_BASE` |
| `spacing` | `SPACE_` + size | `SPACE_SM`, `SPACE_LG`, `SPACE_2XL` |
| `radius` | Size | `SM`, `MD`, `LG`, `PILL` |
| `layout_dims` | Dimension name | `SIDEBAR_LEFT_WIDTH`, `DOT_SIZE` |

When adding new tokens, follow the existing naming pattern and add a corresponding test.

## Semantic Tones

Use `UiTone` instead of raw hex colors:

```rust
use crate::ui::design_tokens::UiTone;

let bg = UiTone::Success.bg();       // "#e8f4e8"
let border = UiTone::Warning.border(); // "#ffc107"
let text = UiTone::Error.text();       // "#721c24"
```

| Tone | Use case |
|------|----------|
| `Neutral` | Default cards, inactive states |
| `Info` | Informational highlights |
| `Success` | Completed actions, positive status |
| `Warning` | Pending approvals, caution states |
| `Error` | Failures, errors, rejections |
| `Primary` | Primary actions, active elements |
| `Danger` | Destructive actions, cancel buttons |

## Size Variants

```rust
use crate::ui::design_tokens::{UiSize, UiDensity};
```

| `UiSize` | Font | Dot |
|----------|------|-----|
| `Xs` | 10px | 6px |
| `Sm` | 11px | 8px |
| `Md` | 12px | 10px |
| `Lg` | 14px | 12px |

| `UiDensity` | Block padding | Inline padding |
|-------------|--------------|----------------|
| `Compact` | 4px | 8px |
| `Comfortable` | 12px | 16px |

## Layout Primitives

Style builders in `layout.rs` produce CSS strings for structural layout:

| Function | Purpose |
|----------|---------|
| `sidebar_style(side)` | Left/right sidebar panel |
| `main_column_style()` | Central flex column |
| `header_bar_style()` | Top header row |
| `status_bar_style()` | Bottom status strip |
| `empty_state_style()` | Centered placeholder |
| `scroll_area_style()` | Scrollable content area |
| `input_area_style()` | Bottom input row |
| `session_item_style(selected)` | Session list item |
| `button_style(tone, disabled)` | Action button |
| `textarea_style(disabled)` | Message input field |

## Reusable Components

Style builders + desktop-gated Dioxus wrappers in `components.rs`:

| Function | Semantic API |
|----------|-------------|
| `card_style(tone, density)` | Bordered container |
| `status_dot_style(tone, size)` | Colored circle |
| `badge_style(tone)` | Pill label |
| `banner_style(tone)` | Full-width alert panel |
| `monospace_block_style()` | Code block |
| `section_header_style(tone)` | Section label |
| `labeled_row_style()` | Key-value row |
| `message_bubble_style(tone)` | Chat message container |

## Architecture

```
design_tokens.rs    Always compiled. Const tokens + semantic enums.
layout.rs           Always compiled (style builders). Desktop-gated render wrappers.
components.rs       Always compiled (style builders). Desktop-gated render wrappers.
```

## Authority Boundary

The design system is **presentation-only**:

- ❌ Imports no backend crates (`openwand_store`, `openwand_session`, `openwand_memory`, `openwand_workflow`)
- ❌ Performs no persistence
- ❌ Executes no tools
- ❌ Verifies nothing, certifies nothing
- ❌ Mutates no trace or memory state
- ❌ Resolves no approvals, routes no actions
- ❌ Creates no workflow records
- ❌ Adds no dependencies to `Cargo.toml`

This is enforced by guard tests in each module.

## Migration Rule for Future UI Waves

When replacing placeholder `*_components.rs` files:

1. **Use `UiTone`** — never pass raw hex colors to style builders or render functions
2. **Use style builders** — call `card_style(tone, density)` instead of inline CSS
3. **Use layout primitives** — call `sidebar_style(side)` instead of hardcoded `width: 260px`
4. **Keep pure helpers** — data extraction stays in the `_components.rs` file; rendering uses design-system tokens
5. **Add guard tests** — verify the migrated file still imports no backend crates

Every migrated surface must prove adoption by replacing at least one hardcoded inline style with a design-system token call.
