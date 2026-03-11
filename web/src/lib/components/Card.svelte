<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		elevated?: boolean;
		interactive?: boolean;
		padding?: 'none' | 'sm' | 'md' | 'lg';
		onclick?: (e: MouseEvent) => void;
		children: Snippet;
	}

	let {
		elevated = false,
		interactive = false,
		padding = 'md',
		onclick,
		children
	}: Props = $props();
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
	class="card"
	class:elevated
	class:interactive
	class:padding-none={padding === 'none'}
	class:padding-sm={padding === 'sm'}
	class:padding-md={padding === 'md'}
	class:padding-lg={padding === 'lg'}
	onclick={interactive ? onclick : undefined}
	onkeydown={interactive ? (e) => e.key === 'Enter' && onclick?.(e as unknown as MouseEvent) : undefined}
	role={interactive ? 'button' : undefined}
	tabindex={interactive ? 0 : undefined}
>
	{@render children()}
</div>

<style>
	.card {
		background: var(--color-slate);
		border: 1px solid var(--color-stone);
		border-radius: var(--radius-lg);
		transition:
			background-color var(--duration-fast) var(--ease-out),
			border-color var(--duration-fast) var(--ease-out),
			box-shadow var(--duration-fast) var(--ease-out);
	}

	.elevated {
		background: var(--color-graphite);
		box-shadow: var(--shadow-subtle);
	}

	.interactive {
		cursor: pointer;
	}

	.interactive:hover {
		background: var(--color-graphite);
		border-color: var(--color-amber);
	}

	.interactive:focus-visible {
		outline: none;
		box-shadow: var(--shadow-glow);
	}

	/* Padding variants */
	.padding-none {
		padding: 0;
	}
	.padding-sm {
		padding: var(--space-2);
	}
	.padding-md {
		padding: var(--space-4);
	}
	.padding-lg {
		padding: var(--space-6);
	}
</style>
