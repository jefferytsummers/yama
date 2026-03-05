<script lang="ts">
	import { runningServicesCount, totalServicesCount, connectionStatus } from '$lib/stores';
</script>

<nav>
	<div class="nav-brand">
		<span class="logo">Y</span>
		<span class="title">Yama Host UI</span>
	</div>

	<div class="nav-links">
		<a href="/" class="nav-link">Dashboard</a>
		<a href="/services" class="nav-link">Services</a>
		<a href="/config" class="nav-link">Config</a>
		<a href="/logs" class="nav-link">Logs</a>
	</div>

	<div class="nav-status">
		<span class="service-count">
			{$runningServicesCount}/{$totalServicesCount} services
		</span>
		<span class="connection-indicator" class:connected={$connectionStatus === 'connected'}>
			{$connectionStatus === 'connected' ? 'Connected' : 'Disconnected'}
		</span>
	</div>
</nav>

<style>
	nav {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 1rem 2rem;
		background-color: var(--color-bg-secondary);
		border-bottom: 1px solid var(--color-surface);
	}

	.nav-brand {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.logo {
		width: 32px;
		height: 32px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: linear-gradient(135deg, var(--color-primary), var(--color-secondary));
		border-radius: 8px;
		font-weight: bold;
		font-size: 1.25rem;
	}

	.title {
		font-weight: 600;
		font-size: 1.125rem;
	}

	.nav-links {
		display: flex;
		gap: 1.5rem;
	}

	.nav-link {
		color: var(--color-text-muted);
		transition: color 0.2s;
	}

	.nav-link:hover {
		color: var(--color-text);
		text-decoration: none;
	}

	.nav-status {
		display: flex;
		align-items: center;
		gap: 1rem;
	}

	.service-count {
		font-family: var(--font-mono);
		font-size: 0.875rem;
		color: var(--color-text-muted);
	}

	.connection-indicator {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.875rem;
		color: var(--color-error);
	}

	.connection-indicator::before {
		content: '';
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background-color: var(--color-error);
	}

	.connection-indicator.connected {
		color: var(--color-success);
	}

	.connection-indicator.connected::before {
		background-color: var(--color-success);
	}
</style>
