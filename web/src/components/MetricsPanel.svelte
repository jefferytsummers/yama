<script lang="ts">
	import { metrics } from '$lib/stores';

	function formatBytes(mb: number): string {
		if (mb >= 1024) {
			return `${(mb / 1024).toFixed(1)} GB`;
		}
		return `${mb.toFixed(0)} MB`;
	}

	function formatUptime(seconds: number): string {
		const hours = Math.floor(seconds / 3600);
		const minutes = Math.floor((seconds % 3600) / 60);
		if (hours > 0) {
			return `${hours}h ${minutes}m`;
		}
		return `${minutes}m`;
	}

	function getUsageClass(percent: number): string {
		if (percent >= 90) return 'critical';
		if (percent >= 70) return 'warning';
		return 'normal';
	}
</script>

<div class="metrics-panel">
	<div class="panel-header">
		<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
			<path d="M3 3v18h18" />
			<path d="M18 17V9M13 17V5M8 17v-3" />
		</svg>
		<h3>System Metrics</h3>
	</div>

	{#if $metrics}
		<div class="metrics-grid">
			<div class="metric-card">
				<div class="metric-header">
					<span class="metric-label">CPU</span>
					<span class="metric-value {getUsageClass($metrics.cpu_usage_percent)}">
						{$metrics.cpu_usage_percent.toFixed(1)}%
					</span>
				</div>
				<div class="progress">
					<div
						class="progress-bar {getUsageClass($metrics.cpu_usage_percent)}"
						style="width: {Math.min($metrics.cpu_usage_percent, 100)}%"
					></div>
				</div>
			</div>

			<div class="metric-card">
				<div class="metric-header">
					<span class="metric-label">Memory</span>
					<span class="metric-value">
						{formatBytes($metrics.memory_used_mb)}
					</span>
				</div>
				<div class="metric-subtitle">
					of {formatBytes($metrics.memory_total_mb)}
				</div>
				<div class="progress">
					<div
						class="progress-bar {getUsageClass(($metrics.memory_used_mb / $metrics.memory_total_mb) * 100)}"
						style="width: {($metrics.memory_used_mb / $metrics.memory_total_mb) * 100}%"
					></div>
				</div>
			</div>

			{#if $metrics.gpu_usage_percent !== null}
				<div class="metric-card gpu">
					<div class="metric-header">
						<span class="metric-label">GPU</span>
						<span class="metric-value inference">
							{$metrics.gpu_usage_percent.toFixed(1)}%
						</span>
					</div>
					<div class="progress">
						<div
							class="progress-bar inference"
							style="width: {Math.min($metrics.gpu_usage_percent, 100)}%"
						></div>
					</div>
				</div>
			{/if}

			<div class="metric-card uptime">
				<div class="metric-header">
					<span class="metric-label">Uptime</span>
					<span class="metric-value uptime-value">
						{formatUptime($metrics.uptime_seconds)}
					</span>
				</div>
				<div class="metric-subtitle">since last restart</div>
			</div>
		</div>
	{:else}
		<div class="loading">
			<span class="spinner"></span>
			<span>Loading metrics...</span>
		</div>
	{/if}
</div>

<style>
	.metrics-panel {
		background-color: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		padding: var(--space-4);
	}

	.panel-header {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		margin-bottom: var(--space-4);
		color: var(--color-text-secondary);
	}

	.panel-header h3 {
		margin: 0;
		font-size: var(--text-body);
	}

	.metrics-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: var(--space-3);
	}

	.metric-card {
		padding: var(--space-3);
		background-color: var(--color-bg);
		border-radius: var(--radius-md);
		transition: background-color var(--duration-fast) var(--ease-out);
	}

	.metric-card:hover {
		background-color: var(--color-basalt);
	}

	.metric-header {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
		margin-bottom: var(--space-1);
	}

	.metric-label {
		font-size: var(--text-tiny);
		font-weight: 500;
		color: var(--color-text-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.metric-value {
		font-family: var(--font-mono);
		font-size: var(--text-h3);
		font-weight: 600;
		color: var(--color-text);
	}

	.metric-value.normal {
		color: var(--color-jade);
	}

	.metric-value.warning {
		color: var(--color-amber);
	}

	.metric-value.critical {
		color: var(--color-ember);
	}

	.metric-value.inference {
		color: var(--color-violet);
	}

	.metric-value.uptime-value {
		color: var(--color-azure);
	}

	.metric-subtitle {
		font-size: var(--text-tiny);
		color: var(--color-text-muted);
		margin-bottom: var(--space-2);
	}

	.progress {
		height: 4px;
		background-color: var(--color-stone);
		border-radius: var(--radius-full);
		overflow: hidden;
		margin-top: var(--space-2);
	}

	.progress-bar {
		height: 100%;
		border-radius: var(--radius-full);
		transition: width var(--duration-slow) var(--ease-out);
		background: linear-gradient(90deg, var(--color-amber), var(--color-ember));
	}

	.progress-bar.normal {
		background: var(--color-jade);
	}

	.progress-bar.warning {
		background: linear-gradient(90deg, var(--color-amber), var(--color-ember));
	}

	.progress-bar.critical {
		background: var(--color-ember);
		animation: pulse 1s ease-in-out infinite;
	}

	.progress-bar.inference {
		background: linear-gradient(90deg, var(--color-violet), var(--color-azure));
	}

	.loading {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-3);
		color: var(--color-text-muted);
		padding: var(--space-6);
	}

	@media (max-width: 500px) {
		.metrics-grid {
			grid-template-columns: 1fr;
		}
	}
</style>
