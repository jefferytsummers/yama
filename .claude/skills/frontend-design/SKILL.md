# Yama Frontend Design

Frontend development for Yama following the "Obsidian Lens" design system.

## Design System Reference

**CRITICAL**: Before creating any frontend components, review:

1. `docs/brand/BRAND.md` - Complete brand guidelines
2. `docs/brand/design-tokens.json` - Exportable tokens
3. `web/src/app.css` - CSS custom properties
4. `web/src/routes/style-guide/+page.svelte` - Component examples

## Brand: "Obsidian Lens"

Yama's visual identity draws from volcanic glass (obsidian) - dark, sophisticated, and precision-focused for video monitoring and AI analysis.

**Core Principles:**
- Precision - Frame-accurate, data-dense interfaces
- Depth - Layered surfaces with subtle elevation
- Stability - Consistent, predictable interactions
- Clarity - Information hierarchy through contrast

## Color Palette

### Backgrounds (dark to light)
```css
--color-obsidian: #0D0D0F   /* Primary background */
--color-basalt: #16161A     /* Secondary/nav background */
--color-slate: #1E1E24      /* Card surfaces */
--color-graphite: #2A2A32   /* Hover states */
--color-stone: #3D3D47      /* Borders */
```

### Accents
```css
--color-amber: #F59E0B      /* Primary accent, CTAs */
--color-ember: #EF4444      /* Errors, destructive */
--color-jade: #10B981       /* Success, running */
--color-azure: #3B82F6      /* Links, info */
--color-violet: #8B5CF6     /* AI/inference indicators */
```

### Text
```css
--color-chalk: #FAFAFA      /* Primary text */
--color-silver: #A1A1AA     /* Secondary text */
--color-ash: #71717A        /* Muted/disabled */
```

## Typography

- Display/Headings: `var(--font-display)` - Geist or system sans
- Body: `var(--font-body)` - Geist or system sans
- Code/Data: `var(--font-mono)` - Geist Mono or system mono

**Never use**: Inter, Roboto, Arial, or generic sans-serif directly.

## Component Patterns

### Cards
```svelte
<div class="card">
  <!-- Uses --color-surface, --color-border, --radius-lg -->
</div>
```

### Buttons
- Primary: Amber background, obsidian text
- Secondary: Transparent with stone border
- Destructive: Ember background

### Status Indicators
```svelte
<span class="status-dot running"></span>  <!-- Jade with glow -->
<span class="status-dot stopped"></span>  <!-- Muted -->
<span class="status-dot inferring"></span> <!-- Violet with pulse -->
```

### Badges
```svelte
<span class="badge success">Running</span>
<span class="badge error">Failed</span>
<span class="badge inference">Analyzing</span>
```

## Implementation Requirements

1. **Always use CSS variables** - Never hardcode colors
2. **Use spacing tokens** - `--space-1` through `--space-12`
3. **Use radius tokens** - `--radius-sm`, `--radius-md`, `--radius-lg`, `--radius-xl`
4. **Follow existing components** - Reference `web/src/components/` for patterns
5. **Test in style guide** - Add new components to `/style-guide`

## Animations

Use sparingly and purposefully:
- `--duration-fast: 100ms` - Hover states
- `--duration-normal: 200ms` - Transitions
- `--duration-slow: 300ms` - Page transitions

Reserved animations:
- `pulse` - Active/running status
- `glow` - AI/inference activity
- `spin` - Loading spinners

## Don't

- Use light themes or bright backgrounds
- Add competing accent colors
- Use generic fonts
- Hardcode colors or spacing values
- Add excessive animations
- Deviate from the established palette

## Framework

- SvelteKit 2.x with TypeScript
- Component-scoped `<style>` blocks
- Responsive breakpoints: 768px, 1024px
