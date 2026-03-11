# Yama Web Frontend

SvelteKit frontend for Yama Content Analyst. Runs in Tauri webview (desktop) or standalone (web).

## Quick Start

```bash
npm run dev          # Dev server at localhost:5173
npm run build        # Production build to build/
npm run preview      # Preview production build
npm run check        # TypeScript + Svelte checks
```

## Architecture

```
web/
├── src/
│   ├── app.css              # Design tokens + global styles
│   ├── app.html             # HTML template
│   ├── lib/
│   │   ├── components/      # Reusable UI components
│   │   ├── api/             # API client for backend
│   │   ├── stores/          # Svelte stores
│   │   └── types/           # TypeScript types
│   └── routes/              # SvelteKit routes
└── static/                  # Static assets
```

## Design System

**Single source of truth:** `docs/brand/design-tokens.json`

All CSS variables in `app.css` are derived from the design tokens. When adding new tokens:
1. Update `design-tokens.json` first
2. Add CSS variable to `app.css`
3. Use the CSS variable in components

### Color Usage

```css
/* Backgrounds (dark → light) */
var(--color-obsidian)   /* #0D0D0F - deepest */
var(--color-basalt)     /* #16161A - secondary */
var(--color-slate)      /* #1E1E24 - cards */
var(--color-graphite)   /* #2A2A32 - hover */
var(--color-stone)      /* #3D3D47 - borders */

/* Accents */
var(--color-amber)      /* #F59E0B - primary CTA */
var(--color-ember)      /* #EF4444 - error/destructive */
var(--color-jade)       /* #10B981 - success */
var(--color-azure)      /* #3B82F6 - info/links */
var(--color-violet)     /* #8B5CF6 - AI/inference */

/* Text */
var(--color-chalk)      /* #FAFAFA - primary */
var(--color-silver)     /* #A1A1AA - secondary */
var(--color-pewter)     /* #8E8E99 - tertiary */
var(--color-ash)        /* #71717A - decorative only */
```

## Components

All components use Svelte 5 runes (`$props`, `$state`, `$derived`).

| Component | Purpose |
|-----------|---------|
| `Button` | Action triggers with variants |
| `Card` | Surface containers |
| `Input` | Form text input |
| `Badge` | Status indicators |
| `Progress` | Progress bars |
| `Skeleton` | Loading placeholders |
| `Modal` | Dialog overlays |
| `Tabs` | Tab navigation |
| `Tooltip` | Contextual help |
| `Icon` | SVG icons |
| `DropZone` | File upload area |
| `ChatBubble` | Chat messages |
| `ToolCallCard` | Tool execution UI |

## Import Pattern

```typescript
import { Button, Card, Badge } from '$lib/components';
```

## Tauri Integration

In Tauri context, the API client connects to the embedded HTTP server:

```typescript
// Detected automatically via window.__TAURI__
import { apiClient } from '$lib/api/client';
```

## Style Guide

Visit `/style-guide` to see all components with interactive demos.

## Constraints

- Dark theme only (Obsidian Lens)
- No light theme flash on load
- Use design tokens exclusively - no hardcoded colors
- Components must match egui theme (shared/yama-theme)
- Svelte 5 runes syntax only
