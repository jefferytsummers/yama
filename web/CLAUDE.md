# Web Frontend Constraints

Non-negotiable requirements for `web/` code.

## Language & Framework

- TypeScript only (strict mode)
- SvelteKit 2.x with Vite
- No JavaScript files

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
    /* Component-scoped CSS */
</style>
```

## Styling

- Use CSS variables for theming
- Component-scoped styles (Svelte default)
- Responsive at 768px and 1024px

## Build

```bash
npm run dev     # Development
npm run build   # Production
```

## See Also

- Full guide: `docs/agent-guides/web-frontend.md`
- API types: `lib/api.ts`
