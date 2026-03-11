---
name: sveltekit-component
description: Create a SvelteKit component with types, stores, and tests. Use for Phase 5 UI development.
allowed-tools: Read, Write, Edit, Glob, Grep, Bash
---

# SvelteKit Component Implementation

Create the `$ARGUMENTS` component following Yama frontend conventions.

## Implementation Steps

### 1. Define Types
Location: `web/src/lib/types/{component}.ts`

```typescript
export interface ComponentProps {
  // Define props
}

export interface ComponentState {
  // Define internal state if complex
}
```

### 2. Create Component
Location: `web/src/lib/components/{Component}.svelte`

```svelte
<script lang="ts">
  import type { ComponentProps } from '$lib/types/{component}';

  export let prop: string;

  // Reactive state
  let state = $state<string>('');

  // Derived values
  let derived = $derived(state.toUpperCase());

  // Effects
  $effect(() => {
    // Side effects
  });
</script>

<div class="component">
  <!-- Template -->
</div>

<style>
  .component {
    /* Use design tokens */
    background: var(--color-slate);
    border: 1px solid var(--color-stone);
    border-radius: var(--radius-lg);
    padding: var(--space-4);
  }
</style>
```

### 3. For SSE/Streaming Components

```svelte
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';

  let eventSource: EventSource | null = null;
  let items = $state<Item[]>([]);

  onMount(() => {
    eventSource = new EventSource(`/api/resource/${id}/stream`);

    eventSource.addEventListener('update', (e) => {
      const data = JSON.parse(e.data);
      items = [...items, data];
    });

    eventSource.addEventListener('error', () => {
      // Handle reconnection
    });
  });

  onDestroy(() => {
    eventSource?.close();
  });
</script>
```

### 4. Create Store (if needed)
Location: `web/src/lib/stores/{component}.ts`

```typescript
import { writable, derived } from 'svelte/store';

export const items = writable<Item[]>([]);

export const filteredItems = derived(items, ($items) =>
  $items.filter(item => item.active)
);

export function addItem(item: Item) {
  items.update(current => [...current, item]);
}
```

### 5. Add to Route
Location: `web/src/routes/{path}/+page.svelte`

```svelte
<script>
  import Component from '$lib/components/Component.svelte';
</script>

<Component prop="value" />
```

## Design Token Usage

Always use CSS variables from `app.css`:

```css
/* Backgrounds */
var(--color-obsidian)   /* #0D0D0F - deepest */
var(--color-basalt)     /* #16161A */
var(--color-slate)      /* #1E1E24 - cards */
var(--color-graphite)   /* #2A2A32 - hover */
var(--color-stone)      /* #3D3D47 - borders */

/* Accents */
var(--color-amber)      /* #F59E0B - primary */
var(--color-ember)      /* #EF4444 - error */
var(--color-jade)       /* #10B981 - success */
var(--color-azure)      /* #3B82F6 - info */
var(--color-violet)     /* #8B5CF6 - inference */

/* Text */
var(--color-chalk)      /* #FAFAFA - primary */
var(--color-silver)     /* #A1A1AA - secondary */
var(--color-ash)        /* #71717A - muted */

/* Spacing */
var(--space-1) /* 4px */
var(--space-2) /* 8px */
var(--space-3) /* 12px */
var(--space-4) /* 16px */
var(--space-6) /* 24px */
var(--space-8) /* 32px */

/* Radius */
var(--radius-sm) /* 4px */
var(--radius-md) /* 6px */
var(--radius-lg) /* 8px */
var(--radius-xl) /* 12px */
```

## Checklist

- [ ] TypeScript types defined
- [ ] Component uses design tokens (no hardcoded colors)
- [ ] Proper reactive state ($state, $derived)
- [ ] SSE cleanup in onDestroy if streaming
- [ ] Accessible (proper ARIA, focus management)
- [ ] Responsive (mobile breakpoints)
- [ ] Loading and error states handled
- [ ] Added to relevant route

## Reference

- Design tokens: `docs/brand/design-tokens.json`
- Prototype: `prototypes/content-analyst-web/index.html`
- Existing CSS: `web/src/app.css`
