<script lang="ts">
	import { hostStatus, connectionStatus } from '$lib/stores';

	function getStatusClass(status: string): string {
		switch (status) {
			case 'running':
				return 'status-running';
			case 'stopped':
				return 'status-stopped';
			case 'starting':
			case 'stopping':
				return 'status-starting';
			default:
				return 'status-error';
		}
	}

	function getStatusBorderColor(status: string): string {
		switch (status) {
			case 'running':
				return 'var(--color-jade)';
			case 'stopped':
				return 'var(--color-stone)';
			case 'starting':
			case 'stopping':
				return 'var(--color-amber)';
			default:
				return 'var(--color-ember)';
		}
	}
</script>

<div class="console">
	<div class="console-header">
		<div class="console-title">
			<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
				<rect x="2" y="3" width="20" height="14" rx="2" />
				<path d="M8 21h8M12 17v4" />
			</svg>
			<h3>Host Console</h3>
		</div>
		<span class="connection-badge" class:connected={$connectionStatus === 'connected'}>
			<span class="connection-dot"></span>
			<span class="connection-text">{$connectionStatus === 'connected' ? 'Live' : 'Offline'}</span>
		</span>
	</div>

	{#if $hostStatus}
		<div class="host-info">
			<div class="info-row">
				<span class="info-label">Version</span>
				<code class="info-value">{$hostStatus.version}</code>
			</div>
			<div class="info-row">
				<span class="info-label">Platform</span>
				<code class="info-value">{$hostStatus.platform}/{$hostStatus.arch}</code>
			</div>
		</div>

		<div class="subsystems">
			{#each [
				$hostStatus.event_bus,
				$hostStatus.http_server,
				$hostStatus.orchestrator,
				$hostStatus.compositor
			] as subsystem}
				<div class="subsystem" style="--border-color: {getStatusBorderColor(subsystem.status)}">
					<div class="subsystem-header">
						<span class="subsystem-name">{subsystem.name}</span>
						<span class="subsystem-status {getStatusClass(subsystem.status)}">
							<span class="status-dot"></span>
							{subsystem.status}
						</span>
					</div>
					<code class="subsystem-details">{subsystem.details}</code>
				</div>
			{/each}
		</div>
	{:else}
		<div class="loading">
			<span class="spinner"></span>
			<span>Connecting to host...</span>
		</div>
	{/if}
</div>

<style>
	.console {
		background-color: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		padding: var(--space-4);
		font-family: var(--font-mono);
		font-size: var(--text-small);
	}

	.console-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: var(--space-4);
		padding-bottom: var(--space-3);
		border-bottom: 1px solid var(--color-border);
	}

	.console-title {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		color: var(--color-text-secondary);
	}

	.console-title h3 {
		margin: 0;
		font-size: var(--text-body);
		font-family: var(--font-body);
	}

	.connection-badge {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-1) var(--space-2);
		border-radius: var(--radius-sm);
		font-size: var(--text-tiny);
		font-weight: 500;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		background-color: color-mix(in srgb, var(--color-ember) 15%, transparent);
		color: var(--color-ember);
	}

	.connection-badge.connected {
		background-color: color-mix(in srgb, var(--color-jade) 15%, transparent);
		color: var(--color-jade);
	}

	.connection-dot {
		width: 6px;
		height: 6px;
		border-radius: var(--radius-full);
		background-color: currentColor;
	}

	.connection-badge.connected .connection-dot {
		animation: pulse 2s ease-in-out infinite;
		box-shadow: 0 0 6px currentColor;
	}

	.host-info {
		display: flex;
		gap: var(--space-4);
		margin-bottom: var(--space-4);
		padding: var(--space-3);
		background-color: var(--color-bg);
		border-radius: var(--radius-md);
	}

	.info-row {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}

	.info-label {
		font-size: var(--text-tiny);
		color: var(--color-text-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		font-family: var(--font-body);
	}

	.info-value {
		color: var(--color-text-secondary);
		background: none;
		padding: 0;
	}

	.subsystems {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.subsystem {
		padding: var(--space-3);
		background-color: var(--color-bg);
		border-radius: var(--radius-md);
		border-left: 3px solid var(--border-color, var(--color-amber));
		transition: background-color var(--duration-fast) var(--ease-out);
	}

	.subsystem:hover {
		background-color: var(--color-basalt);
	}

	.subsystem-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: var(--space-1);
	}

	.subsystem-name {
		font-weight: 600;
		color: var(--color-text);
		font-family: var(--font-body);
	}

	.subsystem-status {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		font-size: var(--text-tiny);
		text-transform: uppercase;
		letter-spacing: 0.03em;
	}

	.subsystem-status .status-dot {
		width: 5px;
		height: 5px;
		border-radius: var(--radius-full);
		background-color: currentColor;
	}

	.subsystem-status.status-running .status-dot {
		animation: pulse 2s ease-in-out infinite;
	}

	.subsystem-details {
		font-size: var(--text-tiny);
		color: var(--color-text-muted);
		background: none;
		padding: 0;
	}

	.loading {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-3);
		color: var(--color-text-muted);
		padding: var(--space-8);
	}
</style>
