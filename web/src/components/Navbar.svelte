<script lang="ts">
	import { runningServicesCount, totalServicesCount, connectionStatus } from '$lib/stores';
</script>

<nav>
	<div class="nav-brand">
		<div class="logo">
			<svg width="20" height="20" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
				<path d="M12 2L2 19H22L12 2Z" fill="currentColor" />
				<path d="M12 6L6 17H18L12 6Z" fill="var(--color-obsidian)" />
			</svg>
		</div>
		<span class="title">Yama</span>
		<span class="subtitle">Host UI</span>
	</div>

	<div class="nav-links">
		<a href="/" class="nav-link">Dashboard</a>
		<a href="/services" class="nav-link">Services</a>
		<a href="/config" class="nav-link">Config</a>
		<a href="/logs" class="nav-link">Logs</a>
	</div>

	<div class="nav-status">
		<span class="service-count">
			<span class="count">{$runningServicesCount}</span>
			<span class="separator">/</span>
			<span class="total">{$totalServicesCount}</span>
			<span class="label">services</span>
		</span>
		<div class="connection-indicator" class:connected={$connectionStatus === 'connected'}>
			<span class="dot"></span>
			<span class="text">{$connectionStatus === 'connected' ? 'Connected' : 'Disconnected'}</span>
		</div>
	</div>
</nav>

<style>
	nav {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: var(--space-3) var(--space-6);
		background-color: var(--color-bg-secondary);
		border-bottom: 1px solid var(--color-border);
		position: sticky;
		top: 0;
		z-index: 100;
	}

	.nav-brand {
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}

	.logo {
		width: 36px;
		height: 36px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: linear-gradient(135deg, var(--color-amber), var(--color-ember));
		border-radius: var(--radius-lg);
		color: var(--color-obsidian);
		box-shadow: 0 2px 8px rgba(245, 158, 11, 0.3);
	}

	.title {
		font-family: var(--font-display);
		font-weight: 600;
		font-size: var(--text-h2);
		letter-spacing: 0.02em;
		background: linear-gradient(90deg, var(--color-chalk), var(--color-silver));
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
	}

	.subtitle {
		font-size: var(--text-small);
		color: var(--color-text-muted);
		padding: var(--space-1) var(--space-2);
		background-color: var(--color-surface);
		border-radius: var(--radius-sm);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		font-weight: 500;
	}

	.nav-links {
		display: flex;
		gap: var(--space-1);
	}

	.nav-link {
		color: var(--color-text-secondary);
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-md);
		font-weight: 500;
		font-size: var(--text-body);
		transition:
			color var(--duration-fast) var(--ease-out),
			background-color var(--duration-fast) var(--ease-out);
	}

	.nav-link:hover {
		color: var(--color-text);
		background-color: var(--color-surface);
		text-decoration: none;
	}

	.nav-link:focus-visible {
		outline: none;
		box-shadow: var(--shadow-glow);
	}

	.nav-status {
		display: flex;
		align-items: center;
		gap: var(--space-4);
	}

	.service-count {
		display: flex;
		align-items: baseline;
		gap: var(--space-1);
		font-family: var(--font-mono);
		font-size: var(--text-small);
		color: var(--color-text-muted);
		padding: var(--space-2) var(--space-3);
		background-color: var(--color-surface);
		border-radius: var(--radius-md);
	}

	.service-count .count {
		color: var(--color-jade);
		font-weight: 600;
	}

	.service-count .separator {
		color: var(--color-border);
	}

	.service-count .total {
		color: var(--color-text-secondary);
	}

	.service-count .label {
		color: var(--color-text-muted);
		font-size: var(--text-tiny);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.connection-indicator {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--text-small);
		color: var(--color-error);
		padding: var(--space-2) var(--space-3);
		background-color: color-mix(in srgb, var(--color-error) 10%, transparent);
		border-radius: var(--radius-md);
		transition:
			color var(--duration-normal) var(--ease-out),
			background-color var(--duration-normal) var(--ease-out);
	}

	.connection-indicator .dot {
		width: 8px;
		height: 8px;
		border-radius: var(--radius-full);
		background-color: var(--color-error);
		transition: background-color var(--duration-normal) var(--ease-out);
	}

	.connection-indicator.connected {
		color: var(--color-success);
		background-color: color-mix(in srgb, var(--color-success) 10%, transparent);
	}

	.connection-indicator.connected .dot {
		background-color: var(--color-success);
		box-shadow: 0 0 8px var(--color-success);
		animation: pulse 2s ease-in-out infinite;
	}

	.connection-indicator .text {
		font-weight: 500;
	}

	@media (max-width: 768px) {
		nav {
			flex-wrap: wrap;
			gap: var(--space-3);
		}

		.nav-links {
			order: 3;
			width: 100%;
			justify-content: center;
		}

		.subtitle {
			display: none;
		}
	}
</style>
