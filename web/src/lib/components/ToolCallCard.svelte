<script lang="ts">
	interface Props {
		name: string;
		status: 'pending' | 'running' | 'complete' | 'error';
		arguments?: Record<string, unknown>;
		result?: string;
		duration?: number;
	}

	let {
		name,
		status,
		arguments: args,
		result,
		duration
	}: Props = $props();

	let expanded = $state(false);

	const statusConfig = {
		pending: { label: 'Pending', color: 'var(--color-silver)' },
		running: { label: 'Running', color: 'var(--color-amber)' },
		complete: { label: 'Complete', color: 'var(--color-jade)' },
		error: { label: 'Error', color: 'var(--color-ember)' }
	};

	const config = $derived(statusConfig[status]);
</script>

<div class="tool-call" class:expanded>
	<button class="tool-header" onclick={() => (expanded = !expanded)}>
		<div class="tool-icon" style="color: {config.color}">
			{#if status === 'running'}
				<svg class="spinner" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<circle cx="12" cy="12" r="10" stroke-opacity="0.25" />
					<path d="M12 2a10 10 0 0 1 10 10" />
				</svg>
			{:else if status === 'complete'}
				<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<path d="M20 6L9 17l-5-5" />
				</svg>
			{:else if status === 'error'}
				<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<circle cx="12" cy="12" r="10" />
					<path d="M12 8v4M12 16h.01" />
				</svg>
			{:else}
				<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<circle cx="12" cy="12" r="10" stroke-opacity="0.5" />
				</svg>
			{/if}
		</div>

		<span class="tool-name">{name}</span>

		<span class="tool-status" style="color: {config.color}">
			{config.label}
			{#if duration}
				<span class="tool-duration">({duration}ms)</span>
			{/if}
		</span>

		<svg
			class="tool-chevron"
			class:rotated={expanded}
			width="16"
			height="16"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="2"
		>
			<path d="M6 9l6 6 6-6" />
		</svg>
	</button>

	{#if expanded}
		<div class="tool-body animate-slide-down">
			{#if args}
				<div class="tool-section">
					<span class="tool-section-label">Arguments</span>
					<pre class="tool-code">{JSON.stringify(args, null, 2)}</pre>
				</div>
			{/if}
			{#if result}
				<div class="tool-section">
					<span class="tool-section-label">Result</span>
					<pre class="tool-code">{result}</pre>
				</div>
			{/if}
		</div>
	{/if}
</div>

<style>
	.tool-call {
		background: var(--color-basalt);
		border: 1px solid var(--color-stone);
		border-radius: var(--radius-lg);
		overflow: hidden;
		transition: border-color var(--duration-fast) var(--ease-out);
	}

	.tool-call:hover {
		border-color: var(--color-silver);
	}

	.tool-header {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		width: 100%;
		padding: var(--space-3);
		background: none;
		border: none;
		font-family: var(--font-sans);
		cursor: pointer;
		text-align: left;
	}

	.tool-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}

	.spinner {
		animation: spin 1s linear infinite;
	}

	.tool-name {
		flex: 1;
		font-family: var(--font-mono);
		font-size: var(--text-small);
		color: var(--color-chalk);
	}

	.tool-status {
		font-size: var(--text-small);
		font-weight: 500;
	}

	.tool-duration {
		font-weight: 400;
		opacity: 0.7;
	}

	.tool-chevron {
		color: var(--color-ash);
		transition: transform var(--duration-fast) var(--ease-out);
	}

	.tool-chevron.rotated {
		transform: rotate(180deg);
	}

	.tool-body {
		padding: 0 var(--space-3) var(--space-3);
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}

	.tool-section {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}

	.tool-section-label {
		font-size: var(--text-tiny);
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-ash);
	}

	.tool-code {
		font-family: var(--font-mono);
		font-size: var(--text-small);
		color: var(--color-silver);
		background: var(--color-obsidian);
		padding: var(--space-2);
		border-radius: var(--radius-sm);
		overflow-x: auto;
		margin: 0;
		white-space: pre-wrap;
		word-break: break-all;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
</style>
