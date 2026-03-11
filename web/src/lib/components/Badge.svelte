<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		variant?: 'neutral' | 'success' | 'warning' | 'error' | 'info' | 'inference';
		size?: 'sm' | 'md';
		pulse?: boolean;
		glow?: boolean;
		children: Snippet;
	}

	let {
		variant = 'neutral',
		size = 'md',
		pulse = false,
		glow = false,
		children
	}: Props = $props();
</script>

<span
	class="badge badge-{variant} badge-{size}"
	class:pulse
	class:glow
>
	<span class="badge-dot"></span>
	<span class="badge-text">
		{@render children()}
	</span>
</span>

<style>
	.badge {
		display: inline-flex;
		align-items: center;
		gap: var(--space-1);
		font-weight: 500;
		border-radius: var(--radius-full);
		white-space: nowrap;
	}

	/* Sizes */
	.badge-sm {
		height: 20px;
		padding: 0 var(--space-2);
		font-size: var(--text-tiny);
	}

	.badge-md {
		height: 24px;
		padding: 0 var(--space-3);
		font-size: var(--text-small);
	}

	.badge-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	/* Variants */
	.badge-neutral {
		background: var(--color-graphite);
		color: var(--color-pewter);
	}
	.badge-neutral .badge-dot {
		background: var(--color-pewter);
	}

	.badge-success {
		background: rgba(16, 185, 129, 0.15);
		color: var(--color-jade);
	}
	.badge-success .badge-dot {
		background: var(--color-jade);
	}

	.badge-warning {
		background: rgba(245, 158, 11, 0.15);
		color: var(--color-amber);
	}
	.badge-warning .badge-dot {
		background: var(--color-amber);
	}

	.badge-error {
		background: rgba(239, 68, 68, 0.15);
		color: var(--color-ember);
	}
	.badge-error .badge-dot {
		background: var(--color-ember);
	}

	.badge-info {
		background: rgba(59, 130, 246, 0.15);
		color: var(--color-azure);
	}
	.badge-info .badge-dot {
		background: var(--color-azure);
	}

	.badge-inference {
		background: rgba(139, 92, 246, 0.15);
		color: var(--color-violet);
	}
	.badge-inference .badge-dot {
		background: var(--color-violet);
	}

	/* Pulse animation */
	.pulse .badge-dot {
		animation: pulse 2s ease-in-out infinite;
	}

	/* Glow effect */
	.glow.badge-success {
		box-shadow: 0 0 12px rgba(16, 185, 129, 0.3);
	}
	.glow.badge-error {
		box-shadow: 0 0 12px rgba(239, 68, 68, 0.3);
	}
	.glow.badge-inference {
		box-shadow: 0 0 12px rgba(139, 92, 246, 0.3);
	}

	@keyframes pulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.4;
		}
	}
</style>
