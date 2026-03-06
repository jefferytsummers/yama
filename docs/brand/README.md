# Yama Brand Assets

This directory contains the brand identity and design system for Yama.

## Contents

| File | Description |
|------|-------------|
| `BRAND.md` | Complete brand guidelines and design principles |
| `design-tokens.json` | Exportable design tokens for tooling integration |
| `logo.svg` | Primary logo (512x512) |
| `favicon.svg` | Favicon (32x32) |

## Quick Reference

### Color Palette

**Core (backgrounds)**
- Obsidian `#0D0D0F` - Primary background
- Basalt `#16161A` - Secondary background
- Slate `#1E1E24` - Surface/cards
- Graphite `#2A2A32` - Elevated surfaces
- Stone `#3D3D47` - Borders

**Accent**
- Amber `#F59E0B` - Primary accent, CTAs
- Ember `#EF4444` - Errors
- Jade `#10B981` - Success
- Azure `#3B82F6` - Links, info
- Violet `#8B5CF6` - AI/inference

**Text**
- Chalk `#FAFAFA` - Primary text
- Silver `#A1A1AA` - Secondary text
- Ash `#71717A` - Muted text

### Typography

- Display: Geist (or system sans-serif)
- Monospace: Geist Mono (or system monospace)

### Implementation

**Web (SvelteKit)**
CSS variables are defined in `web/src/app.css`. All components should use these variables for consistency.

**Native (egui)**
Use the `yama-theme` crate for consistent theming:

```rust
use yama_theme::{YamaTheme, colors, spacing};

// Apply theme
let theme = YamaTheme::new();
theme.apply(&egui_ctx);

// Use colors
ui.painter().rect_filled(rect, 8.0, colors::SURFACE);
```

### Style Guide

Visit `/style-guide` in the web app to see all components rendered with the design system.

## Brand Concept

**"Obsidian Lens"** - The visual identity draws from volcanic glass (obsidian) used to focus light and see clearly:

- Dark, sophisticated interfaces (obsidian black)
- Sharp, precise accent colors (volcanic amber/magma)
- Clean geometric forms (crystalline structure)
- Professional monitoring aesthetic (control room)
