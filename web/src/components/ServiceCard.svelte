<script lang="ts">
	import type { ServiceInfo } from '$lib/api';
	import { services } from '$lib/stores';

	interface Props {
		service: ServiceInfo;
	}

	let { service }: Props = $props();

	let loading = $state(false);

	async function handleStart() {
		loading = true;
		try {
			await services.startService(service.name);
		} finally {
			loading = false;
		}
	}

	async function handleStop() {
		loading = true;
		try {
			await services.stopService(service.name);
		} finally {
			loading = false;
		}
	}

	function getStatusClass(status: string): string {
		return `status-${status}`;
	}

	function getStatusIcon(status: string): string {
		switch (status) {
			case 'running':
				return 'pulse';
			case 'stopped':
				return '';
			case 'starting':
			case 'stopping':
				return 'spin';
			case 'failed':
				return '';
			default:
				return '';
		}
	}
</script>

<div class="service-card">
	<div class="service-header">
		<div class="service-info">
			<h3 class="service-name">{service.name}</h3>
			{#if service.container_id}
				<code class="container-id">{service.container_id.slice(0, 12)}</code>
			{/if}
		</div>
		<div class="status-badge {getStatusClass(service.status)}">
			<span class="status-dot {getStatusIcon(service.status)}"></span>
			<span class="status-text">{service.status}</span>
		</div>
	</div>

	<div class="service-actions">
		{#if service.status === 'stopped' || service.status === 'failed'}
			<button onclick={handleStart} disabled={loading} class="primary">
				{#if loading}
					<span class="spinner"></span>
					Starting
				{:else}
					<svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
						<path d="M8 5v14l11-7z" />
					</svg>
					Start
				{/if}
			</button>
		{:else if service.status === 'running'}
			<button onclick={handleStop} disabled={loading} class="secondary">
				{#if loading}
					<span class="spinner"></span>
					Stopping
				{:else}
					<svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
						<rect x="6" y="6" width="12" height="12" />
					</svg>
					Stop
				{/if}
			</button>
		{:else}
			<button disabled class="ghost">
				<span class="spinner"></span>
				{service.status}...
			</button>
		{/if}
	</div>
</div>

<style>
	.service-card {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		background-color: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		padding: var(--space-4);
		transition:
			background-color var(--duration-fast) var(--ease-out),
			border-color var(--duration-fast) var(--ease-out),
			transform var(--duration-fast) var(--ease-out);
	}

	.service-card:hover {
		background-color: var(--color-surface-hover);
		border-color: var(--color-text-muted);
		transform: translateY(-1px);
	}

	.service-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: var(--space-3);
	}

	.service-info {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		min-width: 0;
	}

	.service-name {
		font-size: var(--text-h3);
		font-weight: 600;
		margin: 0;
		color: var(--color-text);
	}

	.container-id {
		font-family: var(--font-mono);
		font-size: var(--text-small);
		color: var(--color-text-muted);
		background-color: var(--color-bg);
		padding: var(--space-1) var(--space-2);
		border-radius: var(--radius-sm);
		display: inline-block;
	}

	.status-badge {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--text-tiny);
		font-weight: 500;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		padding: var(--space-1) var(--space-2);
		border-radius: var(--radius-sm);
		background-color: var(--color-graphite);
		color: var(--color-text-secondary);
		flex-shrink: 0;
	}

	.status-badge.status-running {
		background-color: color-mix(in srgb, var(--color-success) 15%, transparent);
		color: var(--color-success);
	}

	.status-badge.status-stopped {
		background-color: var(--color-graphite);
		color: var(--color-text-muted);
	}

	.status-badge.status-starting,
	.status-badge.status-stopping {
		background-color: color-mix(in srgb, var(--color-amber) 15%, transparent);
		color: var(--color-amber);
	}

	.status-badge.status-failed {
		background-color: color-mix(in srgb, var(--color-error) 15%, transparent);
		color: var(--color-error);
	}

	.status-dot {
		width: 6px;
		height: 6px;
		border-radius: var(--radius-full);
		background-color: currentColor;
	}

	.status-dot.pulse {
		animation: pulse 2s ease-in-out infinite;
		box-shadow: 0 0 6px currentColor;
	}

	.status-dot.spin {
		animation: pulse 1s ease-in-out infinite;
	}

	.service-actions {
		display: flex;
		gap: var(--space-2);
	}

	.service-actions button {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		flex: 1;
		justify-content: center;
		padding: var(--space-2) var(--space-3);
		font-size: var(--text-small);
		font-weight: 500;
	}

	.service-actions button.primary {
		background-color: var(--color-primary);
		color: var(--color-obsidian);
	}

	.service-actions button.primary:hover:not(:disabled) {
		background-color: #FBBF24;
	}

	.service-actions button.secondary {
		background-color: transparent;
		border: 1px solid var(--color-border);
		color: var(--color-text-secondary);
	}

	.service-actions button.secondary:hover:not(:disabled) {
		border-color: var(--color-error);
		color: var(--color-error);
		background-color: color-mix(in srgb, var(--color-error) 10%, transparent);
	}

	.service-actions button.ghost {
		background-color: transparent;
		color: var(--color-text-muted);
	}

	.service-actions button svg {
		flex-shrink: 0;
	}
</style>
