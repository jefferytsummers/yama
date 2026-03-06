# Web Frontend Constraints

Non-negotiable requirements for `web/` code.

## Language & Framework

- TypeScript only (strict mode)
- SvelteKit 2.x with Vite
- No JavaScript files

## Design System

**CRITICAL**: All frontend work MUST follow the Yama design system.

- Brand guide: `docs/brand/BRAND.md`
- Design tokens: `docs/brand/design-tokens.json`
- CSS variables: `web/src/app.css`
- Style guide page: `/style-guide` route

### Required Design Tokens

Use CSS custom properties - never hardcode colors or spacing:

```css
/* Colors */
--color-obsidian     /* Primary background */
--color-basalt       /* Secondary background */
--color-slate        /* Card surfaces */
--color-amber        /* Primary accent */
--color-jade         /* Success */
--color-ember        /* Error */
--color-violet       /* AI/inference */

/* Text */
--color-text         /* Primary (chalk) */
--color-text-secondary
--color-text-muted

/* Spacing */
--space-1 through --space-12

/* Radius */
--radius-sm, --radius-md, --radius-lg, --radius-xl
```

### Component Patterns

- Use `.card` class for surface containers
- Use `.badge` with semantic modifiers (`.success`, `.error`, `.inference`)
- Use `.status-dot` for status indicators
- Use `.progress` / `.progress-bar` for progress

## Stores

- Use existing stores from `lib/stores.ts`
- Don't create duplicate state
- Stores: `services`, `metrics`, `config`, `videoSources`, `hostStatus`

## API Client

- Use `lib/api.ts` for REST calls
- Base URL: `http://localhost:8080/api`
- Handle errors gracefully

## WebSocket

- Event bus at `ws://localhost:8765`
- Use `lib/websocket.ts` EventBusClient
- Binary Protobuf (JSON bridge for dev)

## Required Patterns

```typescript
// Store usage
import { services, metrics } from '$lib/stores';

onMount(() => {
    services.startPolling(5000);
});

onDestroy(() => {
    services.stopPolling();
});

// Reactive subscription
$: runningCount = $services.filter(s => s.status === 'running').length;
```

## Component Structure

```svelte
<script lang="ts">
    // Imports first
    // Props with export let
    // Local state
    // Reactive statements ($:)
    // Functions
</script>

<!-- Template -->

<style>
    /* Use design system variables */
    .my-component {
        background-color: var(--color-surface);
        border: 1px solid var(--color-border);
        border-radius: var(--radius-lg);
        padding: var(--space-4);
    }
</style>
```

## Styling

- Use CSS variables from `app.css` for all theming
- Component-scoped styles (Svelte default)
- Responsive at 768px and 1024px
- Follow "Obsidian Lens" dark theme aesthetic

## Build

```bash
npm run dev     # Development
npm run build   # Production
```

## See Also

- Brand guide: `docs/brand/BRAND.md`
- Design tokens: `docs/brand/design-tokens.json`
- Full guide: `docs/agent-guides/web-frontend.md`
- API types: `lib/api.ts`
