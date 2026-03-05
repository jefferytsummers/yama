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
</script>

<div class="card service-card">
	<div class="service-header">
		<h3 class="service-name">{service.name}</h3>
		<span class={`service-status ${getStatusClass(service.status)}`}>
			{service.status}
		</span>
	</div>

	{#if service.container_id}
		<div class="container-id">
			<span class="label">Container:</span>
			<code>{service.container_id.slice(0, 12)}</code>
		</div>
	{/if}

	<div class="service-actions">
		{#if service.status === 'stopped' || service.status === 'failed'}
			<button onclick={handleStart} disabled={loading}>
				{loading ? 'Starting...' : 'Start'}
			</button>
		{:else if service.status === 'running'}
			<button onclick={handleStop} disabled={loading} class="secondary">
				{loading ? 'Stopping...' : 'Stop'}
			</button>
		{:else}
			<button disabled>
				{service.status}...
			</button>
		{/if}
	</div>
</div>

<style>
	.service-card {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.service-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.service-name {
		font-size: 1.125rem;
		font-weight: 600;
		margin: 0;
	}

	.service-status {
		font-size: 0.875rem;
		font-weight: 500;
		text-transform: capitalize;
	}

	.container-id {
		font-size: 0.875rem;
		color: var(--color-text-muted);
	}

	.container-id .label {
		margin-right: 0.5rem;
	}

	.container-id code {
		font-family: var(--font-mono);
		background-color: var(--color-bg);
		padding: 0.125rem 0.375rem;
		border-radius: 4px;
	}

	.service-actions {
		display: flex;
		gap: 0.5rem;
	}

	button.secondary {
		background-color: var(--color-surface);
		border: 1px solid var(--color-text-muted);
	}

	button.secondary:hover {
		border-color: var(--color-error);
		color: var(--color-error);
	}
</style>
