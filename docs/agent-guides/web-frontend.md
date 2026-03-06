# Web Frontend Guide

Deep reference for SvelteKit frontend development.

## Overview

- **Framework**: SvelteKit 2.x
- **Language**: TypeScript (strict mode)
- **Styling**: CSS (component-scoped)
- **Build**: Vite
- **Connection**: WebSocket to Event Bus (:8765)

## File Structure

```
web/
├── src/
│   ├── routes/           # SvelteKit pages
│   │   ├── +page.svelte  # Dashboard
│   │   ├── +layout.svelte
│   │   └── services/
│   │       └── +page.svelte
│   ├── components/       # Reusable components
│   │   ├── ServiceCard.svelte
│   │   ├── MetricsChart.svelte
│   │   └── Header.svelte
│   └── lib/              # Shared utilities
│       ├── index.ts      # Re-exports
│       ├── api.ts        # REST API client
│       ├── stores.ts     # Svelte stores
│       └── websocket.ts  # Event bus client
├── static/               # Static assets
├── package.json
├── svelte.config.js
├── tsconfig.json
└── vite.config.ts
```

## Stores

Location: `web/src/lib/stores.ts`

### Service Store

```typescript
function createServicesStore() {
    const { subscribe, set, update } = writable<ServiceInfo[]>([]);
    let pollInterval: ReturnType<typeof setInterval> | null = null;

    return {
        subscribe,
        async refresh() {
            const services = await api.getServices();
            set(services);
        },
        startPolling(intervalMs = 5000) {
            this.refresh();
            pollInterval = setInterval(() => this.refresh(), intervalMs);
        },
        stopPolling() {
            if (pollInterval) {
                clearInterval(pollInterval);
                pollInterval = null;
            }
        },
        async startService(id: string) {
            const result = await api.startService(id);
            if (result.success) await this.refresh();
            return result;
        },
        async stopService(id: string) {
            const result = await api.stopService(id);
            if (result.success) await this.refresh();
            return result;
        }
    };
}

export const services = createServicesStore();
```

### Metrics Store

```typescript
function createMetricsStore() {
    const { subscribe, set } = writable<MetricsResponse | null>(null);
    let pollInterval: ReturnType<typeof setInterval> | null = null;

    return {
        subscribe,
        async refresh() {
            const metrics = await api.getMetrics();
            set(metrics);
        },
        startPolling(intervalMs = 2000) {
            this.refresh();
            pollInterval = setInterval(() => this.refresh(), intervalMs);
        },
        stopPolling() {
            if (pollInterval) clearInterval(pollInterval);
        }
    };
}

export const metrics = createMetricsStore();
```

### Derived Stores

```typescript
import { derived, type Readable } from 'svelte/store';

export const runningServicesCount: Readable<number> = derived(
    services,
    ($services) => $services.filter((s) => s.status === 'running').length
);

export const totalServicesCount: Readable<number> = derived(
    services,
    ($services) => $services.length
);
```

### Connection Status

```typescript
export const connectionStatus = writable<'connected' | 'disconnected' | 'connecting'>('disconnected');
```

## API Client

Location: `web/src/lib/api.ts`

### Types

```typescript
export interface ServiceInfo {
    id: string;
    name: string;
    status: 'running' | 'stopped' | 'error';
    type: string;
    version?: string;
    uptime_ms?: number;
}

export interface MetricsResponse {
    cpu_usage: number;
    memory_usage: number;
    gpu_usage?: number;
    fps?: number;
    active_connections: number;
}

export interface ConfigResponse {
    http_server: { bind: string; port: number };
    event_bus: { websocket_bind: string; unix_socket: string };
    orchestrator: { runtime: string; health_interval: number };
}
```

### API Methods

```typescript
const BASE_URL = 'http://localhost:8080/api';

export const api = {
    async getHealth(): Promise<{ status: string }> {
        const res = await fetch(`${BASE_URL}/health`);
        return res.json();
    },

    async getServices(): Promise<ServiceInfo[]> {
        const res = await fetch(`${BASE_URL}/services`);
        return res.json();
    },

    async startService(id: string): Promise<{ success: boolean }> {
        const res = await fetch(`${BASE_URL}/services/${id}/start`, { method: 'POST' });
        return res.json();
    },

    async stopService(id: string): Promise<{ success: boolean }> {
        const res = await fetch(`${BASE_URL}/services/${id}/stop`, { method: 'POST' });
        return res.json();
    },

    async getMetrics(): Promise<MetricsResponse> {
        const res = await fetch(`${BASE_URL}/metrics`);
        return res.json();
    },

    async getConfig(): Promise<ConfigResponse> {
        const res = await fetch(`${BASE_URL}/config`);
        return res.json();
    },

    async getVideoSources(): Promise<VideoSourceInfo[]> {
        const res = await fetch(`${BASE_URL}/video-sources`);
        return res.json();
    }
};
```

## WebSocket Client

Location: `web/src/lib/websocket.ts`

### EventBusClient

```typescript
export interface EventMessage {
    topic: string;
    source: string;
    payload: unknown;
    timestamp: string;
}

export type EventHandler = (event: EventMessage) => void;

export class EventBusClient {
    private ws: WebSocket | null = null;
    private handlers: Map<string, Set<EventHandler>> = new Map();
    private reconnectAttempts = 0;
    private maxReconnectAttempts = 5;

    constructor(private url: string = 'ws://localhost:8765') {}

    connect(): void {
        this.ws = new WebSocket(this.url);

        this.ws.onopen = () => {
            console.log('Event bus connected');
            this.reconnectAttempts = 0;
        };

        this.ws.onclose = () => {
            this.scheduleReconnect();
        };

        this.ws.onmessage = (event) => {
            const message = JSON.parse(event.data) as EventMessage;
            this.dispatch(message);
        };
    }

    subscribe(topic: string, handler: EventHandler): () => void {
        if (!this.handlers.has(topic)) {
            this.handlers.set(topic, new Set());
        }
        this.handlers.get(topic)!.add(handler);

        return () => {
            this.handlers.get(topic)?.delete(handler);
        };
    }

    private dispatch(event: EventMessage): void {
        // Exact match
        this.handlers.get(event.topic)?.forEach((handler) => handler(event));

        // Wildcard match
        this.handlers.forEach((handlers, pattern) => {
            if (pattern.endsWith('*') && event.topic.startsWith(pattern.slice(0, -1))) {
                handlers.forEach((handler) => handler(event));
            }
        });
    }
}

export const eventBus = new EventBusClient();
```

## Component Patterns

### ServiceCard Component

```svelte
<!-- web/src/components/ServiceCard.svelte -->
<script lang="ts">
    import type { ServiceInfo } from '$lib/api';
    import { services } from '$lib/stores';

    export let service: ServiceInfo;

    let loading = false;

    async function handleToggle() {
        loading = true;
        try {
            if (service.status === 'running') {
                await services.stopService(service.id);
            } else {
                await services.startService(service.id);
            }
        } finally {
            loading = false;
        }
    }
</script>

<div class="card" class:running={service.status === 'running'}>
    <h3>{service.name}</h3>
    <span class="status">{service.status}</span>
    <button on:click={handleToggle} disabled={loading}>
        {service.status === 'running' ? 'Stop' : 'Start'}
    </button>
</div>

<style>
    .card {
        padding: 1rem;
        border-radius: 8px;
        background: var(--card-bg);
    }

    .card.running {
        border-left: 4px solid var(--success);
    }

    .status {
        text-transform: uppercase;
        font-size: 0.75rem;
    }
</style>
```

### Page with Store Subscription

```svelte
<!-- web/src/routes/+page.svelte -->
<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { services, metrics, runningServicesCount } from '$lib/stores';
    import ServiceCard from '$lib/components/ServiceCard.svelte';

    onMount(() => {
        services.startPolling(5000);
        metrics.startPolling(2000);
    });

    onDestroy(() => {
        services.stopPolling();
        metrics.stopPolling();
    });
</script>

<div class="dashboard">
    <h1>Yama Dashboard</h1>

    <div class="stats">
        <div class="stat">
            <span class="value">{$runningServicesCount}</span>
            <span class="label">Running Services</span>
        </div>
        {#if $metrics}
            <div class="stat">
                <span class="value">{$metrics.cpu_usage.toFixed(1)}%</span>
                <span class="label">CPU</span>
            </div>
        {/if}
    </div>

    <div class="services">
        {#each $services as service (service.id)}
            <ServiceCard {service} />
        {/each}
    </div>
</div>
```

### Layout

```svelte
<!-- web/src/routes/+layout.svelte -->
<script lang="ts">
    import { connectionStatus } from '$lib/stores';
    import { eventBus } from '$lib/websocket';
    import { onMount } from 'svelte';

    onMount(() => {
        eventBus.connect();
    });
</script>

<div class="app">
    <header>
        <nav>
            <a href="/">Dashboard</a>
            <a href="/services">Services</a>
        </nav>
        <span class="status" class:connected={$connectionStatus === 'connected'}>
            {$connectionStatus}
        </span>
    </header>

    <main>
        <slot />
    </main>
</div>
```

## Reactive Patterns

### Using $: for Derived State

```svelte
<script lang="ts">
    import { services } from '$lib/stores';

    $: healthyServices = $services.filter(s => s.status === 'running');
    $: hasErrors = $services.some(s => s.status === 'error');
</script>
```

### Event Bus Subscription in Component

```svelte
<script lang="ts">
    import { eventBus } from '$lib/websocket';
    import { onMount, onDestroy } from 'svelte';

    let events: EventMessage[] = [];
    let unsubscribe: (() => void) | null = null;

    onMount(() => {
        unsubscribe = eventBus.subscribe('system.*', (event) => {
            events = [...events.slice(-99), event];
        });
    });

    onDestroy(() => {
        unsubscribe?.();
    });
</script>
```

## TypeScript Configuration

```json
// tsconfig.json
{
    "extends": "./.svelte-kit/tsconfig.json",
    "compilerOptions": {
        "strict": true,
        "noImplicitAny": true,
        "strictNullChecks": true
    }
}
```

## Build & Development

```bash
# Install dependencies
npm install

# Development server (hot reload)
npm run dev

# Build for production
npm run build

# Preview production build
npm run preview
```

## API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| /api/health | GET | Health check |
| /api/services | GET | List services |
| /api/services/{id}/start | POST | Start service |
| /api/services/{id}/stop | POST | Stop service |
| /api/metrics | GET | System metrics |
| /api/config | GET | Configuration |
| /api/video-sources | GET | Video sources |

## Error Handling

```typescript
// In stores or API calls
async function handleApiError<T>(promise: Promise<T>): Promise<T | null> {
    try {
        return await promise;
    } catch (error) {
        console.error('API error:', error);
        // Could dispatch to an error store
        return null;
    }
}
```

## Styling Conventions

- Use CSS variables for theming
- Component-scoped styles (default in Svelte)
- BEM-like naming for complex components
- Responsive breakpoints at 768px and 1024px

```css
:root {
    --primary: #3b82f6;
    --success: #22c55e;
    --error: #ef4444;
    --card-bg: #1f2937;
    --text: #f9fafb;
}
```
