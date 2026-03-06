# Yama Frontend Design

**Extends the global `frontend-design` skill with Yama-specific constraints.**

Follow all principles from the global frontend-design skill (bold aesthetic choices, distinctive typography, intentional design) BUT apply them within the Yama "Obsidian Lens" design system.

## Single Source of Truth

**CRITICAL**: Both web (SvelteKit) and native (egui) interfaces MUST use identical styling.

| Asset | Location | Purpose |
|-------|----------|---------|
| Design Tokens | `docs/brand/design-tokens.json` | **Canonical source** for all values |
| Brand Guide | `docs/brand/BRAND.md` | Design principles and usage |
| Web CSS | `web/src/app.css` | CSS variables (derived from tokens) |
| egui Theme | `shared/yama-theme/src/lib.rs` | Rust constants (derived from tokens) |
| Style Guide | `web/src/routes/style-guide/` | Visual reference |

When adding or modifying design tokens:
1. Update `design-tokens.json` first
2. Update `app.css` CSS variables to match
3. Update `yama-theme` Rust constants to match
4. Verify both platforms render identically

## Brand: "Obsidian Lens"

Dark, sophisticated, precision-focused. Like volcanic glass used to focus light.

- **Precision** - Frame-accurate, data-dense interfaces
- **Depth** - Layered surfaces with subtle elevation
- **Stability** - Consistent, predictable interactions
- **Clarity** - Information hierarchy through contrast

## Color Palette (use these exact values)

### Backgrounds
| Name | Hex | CSS Variable | Rust Constant |
|------|-----|--------------|---------------|
| Obsidian | `#0D0D0F` | `--color-obsidian` | `colors::OBSIDIAN` |
| Basalt | `#16161A` | `--color-basalt` | `colors::BASALT` |
| Slate | `#1E1E24` | `--color-slate` | `colors::SLATE` |
| Graphite | `#2A2A32` | `--color-graphite` | `colors::GRAPHITE` |
| Stone | `#3D3D47` | `--color-stone` | `colors::STONE` |

### Accents
| Name | Hex | CSS Variable | Rust Constant |
|------|-----|--------------|---------------|
| Amber | `#F59E0B` | `--color-amber` | `colors::AMBER` |
| Ember | `#EF4444` | `--color-ember` | `colors::EMBER` |
| Jade | `#10B981` | `--color-jade` | `colors::JADE` |
| Azure | `#3B82F6` | `--color-azure` | `colors::AZURE` |
| Violet | `#8B5CF6` | `--color-violet` | `colors::VIOLET` |

### Text
| Name | Hex | CSS Variable | Rust Constant |
|------|-----|--------------|---------------|
| Chalk | `#FAFAFA` | `--color-chalk` | `colors::CHALK` |
| Silver | `#A1A1AA` | `--color-silver` | `colors::SILVER` |
| Ash | `#71717A` | `--color-ash` | `colors::ASH` |

## Spacing Scale

| Token | Value | CSS | Rust |
|-------|-------|-----|------|
| 1 | 4px | `--space-1` | `spacing::S1` |
| 2 | 8px | `--space-2` | `spacing::S2` |
| 3 | 12px | `--space-3` | `spacing::S3` |
| 4 | 16px | `--space-4` | `spacing::S4` |
| 6 | 24px | `--space-6` | `spacing::S6` |
| 8 | 32px | `--space-8` | `spacing::S8` |

## Border Radius

| Token | Value | CSS | Rust |
|-------|-------|-----|------|
| sm | 4px | `--radius-sm` | `radius::SM` |
| md | 6px | `--radius-md` | `radius::MD` |
| lg | 8px | `--radius-lg` | `radius::LG` |
| xl | 12px | `--radius-xl` | `radius::XL` |

## Cross-Platform Components

Design components that can be implemented identically in both frameworks:

### Card
```
Background: Slate (#1E1E24)
Border: 1px Stone (#3D3D47)
Radius: 8px (lg)
Padding: 16px (space-4)
Hover: Graphite (#2A2A32)
```

### Primary Button
```
Background: Amber (#F59E0B)
Text: Obsidian (#0D0D0F)
Radius: 6px (md)
Padding: 8px 16px
Hover: 10% lighter (#FBBF24)
```

### Status Indicators
```
Running: Jade (#10B981) + glow + pulse animation
Stopped: Ash (#71717A)
Starting/Stopping: Amber (#F59E0B) + pulse animation
Error: Ember (#EF4444) + glow
Inference: Violet (#8B5CF6) + glow + pulse animation
```

### Progress Bar
```
Track: Stone (#3D3D47)
Fill: Gradient Amber→Ember
Height: 4px (default), 8px (large)
Radius: full (9999px)
```

## Typography

Both platforms should use equivalent fonts:

| Role | Web | egui |
|------|-----|------|
| Display | Geist / SF Pro Display | System default |
| Body | Geist / SF Pro Text | System default |
| Mono | Geist Mono / SF Mono | System monospace |

## Implementation Checklist

When creating UI for either platform:

- [ ] Colors match exactly between web and egui
- [ ] Spacing uses token values, not arbitrary pixels
- [ ] Border radius consistent with token scale
- [ ] Status indicators use correct semantic colors
- [ ] Animations reserved for meaningful state changes
- [ ] No hardcoded values - always use tokens/variables
- [ ] Component added to style guide if new pattern

## Platform-Specific Files

**Web (SvelteKit)**
- Entry: `web/src/app.css`
- Components: `web/src/components/`
- Style guide: `web/src/routes/style-guide/+page.svelte`

**Native (egui)**
- Theme crate: `shared/yama-theme/`
- Apply with: `YamaTheme::new().apply(&ctx)`
- Colors: `yama_theme::colors::*`
- Spacing: `yama_theme::spacing::*`

## Forbidden

- Light themes or bright backgrounds
- Colors not in the palette
- Hardcoded color/spacing values
- Visual differences between web and native
- Generic fonts (Inter, Roboto, Arial)
- Excessive or gratuitous animations
