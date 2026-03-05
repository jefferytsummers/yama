<script lang="ts">
	import { metrics } from '$lib/stores';

	function formatBytes(mb: number): string {
		if (mb >= 1024) {
			return `${(mb / 1024).toFixed(1)} GB`;
		}
		return `${mb} MB`;
	}

	function formatUptime(seconds: number): string {
		const hours = Math.floor(seconds / 3600);
		const minutes = Math.floor((seconds % 3600) / 60);
		if (hours > 0) {
			return `${hours}h ${minutes}m`;
		}
		return `${minutes}m`;
	}
</script>

<div class="card metrics-panel">
	<h3>System Metrics</h3>

	{#if $metrics}
		<div class="metrics-grid">
			<div class="metric">
				<span class="metric-label">CPU</span>
				<span class="metric-value">{$metrics.cpu_usage_percent.toFixed(1)}%</span>
				<div class="progress-bar">
					<div class="progress-fill" style="width: {$metrics.cpu_usage_percent}%"></div>
				</div>
			</div>

			<div class="metric">
				<span class="metric-label">Memory</span>
				<span class="metric-value">
					{formatBytes($metrics.memory_used_mb)} / {formatBytes($metrics.memory_total_mb)}
				</span>
				<div class="progress-bar">
					<div
						class="progress-fill"
						style="width: {($metrics.memory_used_mb / $metrics.memory_total_mb) * 100}%"
					></div>
				</div>
			</div>

			{#if $metrics.gpu_usage_percent !== null}
				<div class="metric">
					<span class="metric-label">GPU</span>
					<span class="metric-value">{$metrics.gpu_usage_percent.toFixed(1)}%</span>
					<div class="progress-bar">
						<div class="progress-fill gpu" style="width: {$metrics.gpu_usage_percent}%"></div>
					</div>
				</div>
			{/if}

			<div class="metric">
				<span class="metric-label">Uptime</span>
				<span class="metric-value">{formatUptime($metrics.uptime_seconds)}</span>
			</div>
		</div>
	{:else}
		<p class="loading">Loading metrics...</p>
	{/if}
</div>

<style>
	.metrics-panel h3 {
		margin-bottom: 1rem;
	}

	.metrics-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 1rem;
	}

	.metric {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.metric-label {
		font-size: 0.75rem;
		color: var(--color-text-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.metric-value {
		font-family: var(--font-mono);
		font-size: 1.125rem;
		font-weight: 600;
	}

	.progress-bar {
		height: 4px;
		background-color: var(--color-bg);
		border-radius: 2px;
		overflow: hidden;
		margin-top: 0.25rem;
	}

	.progress-fill {
		height: 100%;
		background: linear-gradient(90deg, var(--color-primary), var(--color-secondary));
		transition: width 0.3s ease;
	}

	.progress-fill.gpu {
		background: linear-gradient(90deg, var(--color-success), #00b894);
	}

	.loading {
		color: var(--color-text-muted);
		font-style: italic;
	}
</style>
