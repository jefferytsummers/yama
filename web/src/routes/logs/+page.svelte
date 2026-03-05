<script lang="ts">
	import { onMount } from 'svelte';

	interface LogEntry {
		timestamp: string;
		level: 'info' | 'warn' | 'error' | 'debug';
		source: string;
		message: string;
	}

	let logs = $state<LogEntry[]>([]);
	let filterLevel = $state<string>('all');
	let filterSource = $state<string>('');
	let autoScroll = $state(true);
	let logContainer: HTMLElement;

	// Placeholder logs for demonstration
	const placeholderLogs: LogEntry[] = [
		{
			timestamp: new Date().toISOString(),
			level: 'info',
			source: 'orchestrator',
			message: 'Starting orchestrator...'
		},
		{
			timestamp: new Date().toISOString(),
			level: 'info',
			source: 'event_bus',
			message: 'WebSocket server listening on 0.0.0.0:8765'
		},
		{
			timestamp: new Date().toISOString(),
			level: 'info',
			source: 'http_server',
			message: 'HTTP server listening on http://0.0.0.0:8080'
		},
		{
			timestamp: new Date().toISOString(),
			level: 'debug',
			source: 'orchestrator',
			message: 'Health check completed for all services'
		}
	];

	onMount(() => {
		// Load placeholder logs
		logs = placeholderLogs;

		// TODO: Subscribe to real-time logs via WebSocket
	});

	$effect(() => {
		if (autoScroll && logContainer) {
			logContainer.scrollTop = logContainer.scrollHeight;
		}
	});

	const filteredLogs = $derived(
		logs.filter((log) => {
			if (filterLevel !== 'all' && log.level !== filterLevel) return false;
			if (filterSource && !log.source.includes(filterSource)) return false;
			return true;
		})
	);

	function getLevelClass(level: string): string {
		switch (level) {
			case 'error':
				return 'level-error';
			case 'warn':
				return 'level-warn';
			case 'debug':
				return 'level-debug';
			default:
				return 'level-info';
		}
	}

	function formatTime(isoString: string): string {
		return new Date(isoString).toLocaleTimeString();
	}

	function clearLogs() {
		logs = [];
	}
</script>

<svelte:head>
	<title>Logs - Yama Host UI</title>
</svelte:head>

<div class="logs-page">
	<header class="page-header">
		<h1>Logs</h1>
		<p class="subtitle">Real-time system logs</p>
	</header>

	<div class="toolbar">
		<div class="filters">
			<label>
				Level:
				<select bind:value={filterLevel}>
					<option value="all">All</option>
					<option value="error">Error</option>
					<option value="warn">Warning</option>
					<option value="info">Info</option>
					<option value="debug">Debug</option>
				</select>
			</label>

			<label>
				Source:
				<input type="text" bind:value={filterSource} placeholder="Filter by source..." />
			</label>
		</div>

		<div class="actions">
			<label class="checkbox">
				<input type="checkbox" bind:checked={autoScroll} />
				Auto-scroll
			</label>
			<button onclick={clearLogs} class="secondary">Clear</button>
		</div>
	</div>

	<div class="card log-container" bind:this={logContainer}>
		{#each filteredLogs as log (log.timestamp + log.message)}
			<div class="log-entry">
				<span class="log-time">{formatTime(log.timestamp)}</span>
				<span class={`log-level ${getLevelClass(log.level)}`}>{log.level.toUpperCase()}</span>
				<span class="log-source">[{log.source}]</span>
				<span class="log-message">{log.message}</span>
			</div>
		{:else}
			<p class="empty">No logs to display</p>
		{/each}
	</div>
</div>

<style>
	.logs-page {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
		height: calc(100vh - 12rem);
	}

	.page-header {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.page-header h1 {
		margin: 0;
	}

	.subtitle {
		color: var(--color-text-muted);
		margin: 0;
	}

	.toolbar {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 1rem;
		flex-wrap: wrap;
	}

	.filters {
		display: flex;
		gap: 1rem;
	}

	.filters label {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.875rem;
	}

	.filters select,
	.filters input {
		padding: 0.375rem 0.75rem;
		background-color: var(--color-surface);
		border: 1px solid var(--color-bg);
		border-radius: var(--radius);
		color: var(--color-text);
		font-family: inherit;
	}

	.actions {
		display: flex;
		align-items: center;
		gap: 1rem;
	}

	.checkbox {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.875rem;
		cursor: pointer;
	}

	button.secondary {
		background-color: var(--color-surface);
		border: 1px solid var(--color-text-muted);
	}

	.log-container {
		flex: 1;
		overflow-y: auto;
		font-family: var(--font-mono);
		font-size: 0.8125rem;
		line-height: 1.6;
		padding: 1rem;
	}

	.log-entry {
		display: flex;
		gap: 0.75rem;
		white-space: nowrap;
	}

	.log-time {
		color: var(--color-text-muted);
	}

	.log-level {
		min-width: 5ch;
		font-weight: 600;
	}

	.level-error {
		color: var(--color-error);
	}

	.level-warn {
		color: var(--color-warning);
	}

	.level-info {
		color: var(--color-success);
	}

	.level-debug {
		color: var(--color-text-muted);
	}

	.log-source {
		color: var(--color-secondary);
	}

	.log-message {
		white-space: pre-wrap;
		word-break: break-word;
	}

	.empty {
		color: var(--color-text-muted);
		font-style: italic;
		text-align: center;
		padding: 2rem;
	}
</style>
