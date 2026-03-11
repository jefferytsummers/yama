<script lang="ts">
	import type { ConnectionStatus } from '$lib/stores/vlmConnection';

	interface Props {
		status: ConnectionStatus;
		label?: string;
		showLabel?: boolean;
		size?: 'sm' | 'md' | 'lg';
	}

	let {
		status,
		label,
		showLabel = true,
		size = 'md'
	}: Props = $props();

	const statusLabels: Record<ConnectionStatus, string> = {
		connected: 'Connected',
		connecting: 'Connecting',
		disconnected: 'Disconnected',
		stale: 'Stale'
	};

	let displayLabel = $derived(label ?? statusLabels[status]);

	const sizeClasses: Record<typeof size, string> = {
		sm: 'size-sm',
		md: 'size-md',
		lg: 'size-lg'
	};
</script>

<div class="connection-status {status} {sizeClasses[size]}" role="status" aria-live="polite">
	<span class="status-indicator">
		<span class="dot"></span>
		{#if status === 'connecting'}
			<span class="dot pulse-ring"></span>
		{/if}
	</span>
	{#if showLabel}
		<span class="status-label">{displayLabel}</span>
	{/if}
</div>

<style>
	.connection-status {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
	}

	.status-indicator {
		position: relative;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.dot {
		border-radius: 50%;
		transition: background-color var(--duration-normal) var(--ease-out);
	}

	/* Size variants */
	.size-sm .dot {
		width: 6px;
		height: 6px;
	}

	.size-md .dot {
		width: 8px;
		height: 8px;
	}

	.size-lg .dot {
		width: 10px;
		height: 10px;
	}

	.size-sm .status-label {
		font-size: var(--text-tiny);
	}

	.size-md .status-label {
		font-size: var(--text-small);
	}

	.size-lg .status-label {
		font-size: var(--text-body);
	}

	/* Status: Connected (solid green) */
	.connected .dot {
		background: var(--color-jade);
		box-shadow: 0 0 8px var(--color-jade);
	}

	.connected .status-label {
		color: var(--color-jade);
	}

	/* Status: Connecting (blinking green) */
	.connecting .dot {
		background: var(--color-jade);
		animation: blink 1s ease-in-out infinite;
	}

	.connecting .status-label {
		color: var(--color-jade);
	}

	.pulse-ring {
		position: absolute;
		background: transparent !important;
		border: 2px solid var(--color-jade);
		animation: pulse-ring 1.5s ease-out infinite;
		box-shadow: none !important;
	}

	.size-sm .pulse-ring {
		width: 14px;
		height: 14px;
	}

	.size-md .pulse-ring {
		width: 18px;
		height: 18px;
	}

	.size-lg .pulse-ring {
		width: 22px;
		height: 22px;
	}

	/* Status: Disconnected (solid red) */
	.disconnected .dot {
		background: var(--color-ember);
		box-shadow: 0 0 8px var(--color-ember);
	}

	.disconnected .status-label {
		color: var(--color-ember);
	}

	/* Status: Stale (solid yellow/amber) */
	.stale .dot {
		background: var(--color-amber);
		box-shadow: 0 0 8px var(--color-amber);
		animation: pulse-slow 2s ease-in-out infinite;
	}

	.stale .status-label {
		color: var(--color-amber);
	}

	/* Animations */
	@keyframes blink {
		0%, 100% {
			opacity: 1;
		}
		50% {
			opacity: 0.3;
		}
	}

	@keyframes pulse-ring {
		0% {
			transform: scale(1);
			opacity: 1;
		}
		100% {
			transform: scale(2);
			opacity: 0;
		}
	}

	@keyframes pulse-slow {
		0%, 100% {
			opacity: 1;
			transform: scale(1);
		}
		50% {
			opacity: 0.7;
			transform: scale(1.1);
		}
	}
</style>
