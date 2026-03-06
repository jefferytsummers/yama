# Yama Brand Identity

## Overview

**Yama** (山 - Japanese for "mountain") is a Vision AI compositor that brings clarity and insight to video streams. The brand embodies:

- **Precision** - Real-time, frame-accurate video analysis
- **Depth** - Multi-layered understanding of visual content
- **Stability** - Rock-solid reliability like a mountain
- **Clarity** - Clear insights emerging from visual noise

## Brand Concept: "Obsidian Lens"

The visual identity draws from the metaphor of an obsidian lens - volcanic glass that ancient cultures used to focus light and see clearly. This connects:

- Dark, sophisticated interfaces (obsidian black)
- Sharp, precise accent colors (volcanic amber/magma)
- Clean geometric forms (crystalline structure)
- Professional monitoring aesthetic (control room)

---

## Color Palette

### Core Colors

| Name | Hex | RGB | Usage |
|------|-----|-----|-------|
| **Obsidian** | `#0D0D0F` | `13, 13, 15` | Primary background |
| **Basalt** | `#16161A` | `22, 22, 26` | Secondary background |
| **Slate** | `#1E1E24` | `30, 30, 36` | Surface/cards |
| **Graphite** | `#2A2A32` | `42, 42, 50` | Elevated surfaces |
| **Stone** | `#3D3D47` | `61, 61, 71` | Borders, dividers |

### Accent Colors

| Name | Hex | RGB | Usage |
|------|-----|-----|-------|
| **Amber** | `#F59E0B` | `245, 158, 11` | Primary accent, CTAs |
| **Ember** | `#EF4444` | `239, 68, 68` | Errors, destructive |
| **Jade** | `#10B981` | `16, 185, 129` | Success, running |
| **Azure** | `#3B82F6` | `59, 130, 246` | Info, links |
| **Violet** | `#8B5CF6` | `139, 92, 246` | AI/inference indicators |

### Text Colors

| Name | Hex | RGB | Usage |
|------|-----|-----|-------|
| **Chalk** | `#FAFAFA` | `250, 250, 250` | Primary text |
| **Silver** | `#A1A1AA` | `161, 161, 170` | Secondary text |
| **Ash** | `#71717A` | `113, 113, 122` | Muted/disabled text |

### Gradients

**Magma Gradient** (Primary CTA)
```css
linear-gradient(135deg, #F59E0B 0%, #EF4444 100%)
```

**Depth Gradient** (Surfaces)
```css
linear-gradient(180deg, #1E1E24 0%, #16161A 100%)
```

**AI Glow** (Inference indicators)
```css
radial-gradient(circle, rgba(139, 92, 246, 0.3) 0%, transparent 70%)
```

---

## Typography

### Font Stack

**Display / Headings**
```
"Geist", "SF Pro Display", -apple-system, BlinkMacSystemFont, sans-serif
```

**Body Text**
```
"Geist", "SF Pro Text", -apple-system, BlinkMacSystemFont, sans-serif
```

**Monospace / Code**
```
"Geist Mono", "SF Mono", "JetBrains Mono", "Fira Code", monospace
```

### Type Scale

| Level | Size | Weight | Line Height | Usage |
|-------|------|--------|-------------|-------|
| Display | 32px | 600 | 1.2 | Hero headings |
| H1 | 24px | 600 | 1.3 | Page titles |
| H2 | 20px | 600 | 1.4 | Section headers |
| H3 | 16px | 600 | 1.4 | Card titles |
| Body | 14px | 400 | 1.5 | Default text |
| Small | 12px | 400 | 1.5 | Labels, captions |
| Tiny | 10px | 500 | 1.4 | Badges, status |

### Font Features

Enable these OpenType features for Geist:
- `ss01` - Alternate digits
- `cv01` - Single-story 'a'
- `tnum` - Tabular numbers (for data)

---

## Spacing

Base unit: **4px**

| Token | Value | Usage |
|-------|-------|-------|
| `space-1` | 4px | Tight inline spacing |
| `space-2` | 8px | Icon gaps, small padding |
| `space-3` | 12px | Button padding, list gaps |
| `space-4` | 16px | Card padding, section gaps |
| `space-5` | 20px | - |
| `space-6` | 24px | Large section padding |
| `space-8` | 32px | Page margins |
| `space-10` | 40px | Major section breaks |

---

## Border Radius

| Token | Value | Usage |
|-------|-------|-------|
| `radius-sm` | 4px | Badges, small elements |
| `radius-md` | 6px | Buttons, inputs |
| `radius-lg` | 8px | Cards, panels |
| `radius-xl` | 12px | Modals, large containers |
| `radius-full` | 9999px | Pills, avatars |

---

## Shadows

**Subtle** (cards, dropdowns)
```css
0 1px 2px rgba(0, 0, 0, 0.3), 0 1px 3px rgba(0, 0, 0, 0.2)
```

**Medium** (popovers, modals)
```css
0 4px 6px rgba(0, 0, 0, 0.4), 0 2px 4px rgba(0, 0, 0, 0.3)
```

**Glow** (focused elements)
```css
0 0 0 2px rgba(245, 158, 11, 0.3)
```

**AI Glow** (active inference)
```css
0 0 20px rgba(139, 92, 246, 0.4)
```

---

## Logo

The Yama logo is a stylized mountain peak formed by geometric shapes:

```
    ▲
   ╱ ╲
  ╱   ╲
 ╱─────╲
```

**Logo Mark**: Single mountain glyph
**Logotype**: "YAMA" in Geist Semi-Bold, letter-spacing 0.05em

### Logo Usage

- Minimum size: 24px height
- Clear space: Equal to height of logo on all sides
- On dark backgrounds: Use chalk (#FAFAFA) or amber (#F59E0B)
- On light backgrounds: Use obsidian (#0D0D0F)

### Favicon

32×32 and 16×16 versions showing just the mountain peak glyph in amber on obsidian.

---

## Component Patterns

### Buttons

**Primary** (amber background)
- Background: Amber (#F59E0B)
- Text: Obsidian (#0D0D0F)
- Hover: 10% lighter
- Active: 5% darker

**Secondary** (outlined)
- Background: transparent
- Border: Stone (#3D3D47)
- Text: Silver (#A1A1AA)
- Hover: Stone background

**Destructive**
- Background: Ember (#EF4444)
- Text: Chalk (#FAFAFA)

### Cards

- Background: Slate (#1E1E24)
- Border: 1px Stone (#3D3D47)
- Border-radius: radius-lg (8px)
- Padding: space-4 (16px)
- Hover: Graphite background (#2A2A32)

### Status Indicators

| Status | Color | Glow |
|--------|-------|------|
| Running | Jade (#10B981) | Yes |
| Stopped | Ash (#71717A) | No |
| Starting | Amber (#F59E0B) | Pulse |
| Error | Ember (#EF4444) | Yes |
| Inference | Violet (#8B5CF6) | Pulse |

### Progress Bars

- Track: Stone (#3D3D47)
- Fill: Magma gradient
- Height: 4px (inline), 8px (prominent)
- Border-radius: radius-full

---

## Motion

### Timing

| Token | Duration | Easing | Usage |
|-------|----------|--------|-------|
| `fast` | 100ms | ease-out | Hover states |
| `normal` | 200ms | ease-in-out | Transitions |
| `slow` | 300ms | ease-in-out | Page transitions |
| `deliberate` | 500ms | ease-out | Major state changes |

### Animations

**Pulse** (for active status)
```css
@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}
animation: pulse 2s ease-in-out infinite;
```

**Glow** (for inference activity)
```css
@keyframes glow {
  0%, 100% { box-shadow: 0 0 10px rgba(139, 92, 246, 0.3); }
  50% { box-shadow: 0 0 20px rgba(139, 92, 246, 0.6); }
}
animation: glow 1.5s ease-in-out infinite;
```

---

## Platform-Specific Notes

### Web (SvelteKit)
- Use CSS custom properties for all tokens
- Load Geist font via `@fontsource/geist` or CDN
- Prefer CSS animations over JS for performance

### Native (egui)
- Use the `yama_theme` Rust module for consistent colors
- Map design tokens to `egui::Color32` values
- egui uses slightly different rounding - use 6.0 instead of 8px for cards

---

## Usage Examples

### Good
- Dark backgrounds with amber accents for key actions
- Monospace font for data, timestamps, IDs
- Subtle borders to separate content areas
- Status indicators with appropriate semantic colors

### Avoid
- Bright backgrounds or light themes (not fitting for monitoring)
- Multiple competing accent colors on same screen
- Overuse of animations - reserve for meaningful state changes
- Generic sans-serif fonts
