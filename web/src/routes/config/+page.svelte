<script lang="ts">
	import { config } from '$lib/stores';
</script>

<svelte:head>
	<title>Configuration - Yama Host UI</title>
</svelte:head>

<div class="config-page">
	<header class="page-header">
		<h1>Configuration</h1>
		<p class="subtitle">View and manage system configuration</p>
	</header>

	{#if $config}
		<div class="config-grid">
			<section class="card config-section">
				<h2>Compositor</h2>
				<dl class="config-list">
					<dt>Backend</dt>
					<dd>{$config.compositor.backend}</dd>
					<dt>Renderer</dt>
					<dd>{$config.compositor.renderer}</dd>
					<dt>Resolution</dt>
					<dd>{$config.compositor.width} x {$config.compositor.height}</dd>
					<dt>FPS</dt>
					<dd>{$config.compositor.fps}</dd>
					<dt>VSync</dt>
					<dd>{$config.compositor.vsync ? 'Enabled' : 'Disabled'}</dd>
				</dl>
			</section>

			<section class="card config-section">
				<h2>Event Bus</h2>
				<dl class="config-list">
					<dt>WebSocket</dt>
					<dd><code>{$config.event_bus.websocket_bind}</code></dd>
					<dt>Unix Socket</dt>
					<dd><code>{$config.event_bus.unix_socket}</code></dd>
				</dl>
			</section>

			<section class="card config-section">
				<h2>Orchestrator</h2>
				<dl class="config-list">
					<dt>Runtime</dt>
					<dd>{$config.orchestrator.runtime}</dd>
					<dt>Health Interval</dt>
					<dd>{$config.orchestrator.health_interval}s</dd>
					<dt>Max Restarts</dt>
					<dd>{$config.orchestrator.max_restarts}</dd>
					<dt>Services</dt>
					<dd>{$config.orchestrator.service_count} configured</dd>
				</dl>
			</section>

			<section class="card config-section">
				<h2>HTTP Server</h2>
				<dl class="config-list">
					<dt>Bind Address</dt>
					<dd><code>{$config.http_server.bind}:{$config.http_server.port}</code></dd>
					<dt>CORS</dt>
					<dd>{$config.http_server.cors_enabled ? 'Enabled' : 'Disabled'}</dd>
				</dl>
			</section>
		</div>
	{:else}
		<div class="card loading">
			<p>Loading configuration...</p>
		</div>
	{/if}
</div>

<style>
	.config-page {
		display: flex;
		flex-direction: column;
		gap: 2rem;
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

	.config-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
		gap: 1.5rem;
	}

	.config-section h2 {
		font-size: 1rem;
		margin-bottom: 1rem;
		color: var(--color-primary);
	}

	.config-list {
		display: grid;
		grid-template-columns: auto 1fr;
		gap: 0.5rem 1rem;
	}

	.config-list dt {
		color: var(--color-text-muted);
		font-size: 0.875rem;
	}

	.config-list dd {
		margin: 0;
		font-family: var(--font-mono);
		font-size: 0.875rem;
	}

	.config-list code {
		background-color: var(--color-bg);
		padding: 0.125rem 0.375rem;
		border-radius: 4px;
	}

	.loading {
		text-align: center;
		padding: 3rem;
		color: var(--color-text-muted);
	}
</style>
