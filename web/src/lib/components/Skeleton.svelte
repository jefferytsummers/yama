<script lang="ts">
	interface Props {
		variant?: 'text' | 'circular' | 'rectangular';
		width?: string;
		height?: string;
		lines?: number;
	}

	let {
		variant = 'text',
		width = '100%',
		height = variant === 'text' ? '1em' : '100px',
		lines = 1
	}: Props = $props();
</script>

{#if variant === 'text' && lines > 1}
	<div class="skeleton-lines" style="width: {width}">
		{#each Array(lines) as _, i}
			<div
				class="skeleton skeleton-text"
				style="width: {i === lines - 1 ? '70%' : '100%'}; height: {height}"
			></div>
		{/each}
	</div>
{:else}
	<div
		class="skeleton skeleton-{variant}"
		style="width: {width}; height: {height}"
	></div>
{/if}

<style>
	.skeleton {
		background: linear-gradient(
			90deg,
			var(--color-graphite) 25%,
			var(--color-stone) 50%,
			var(--color-graphite) 75%
		);
		background-size: 200% 100%;
		animation: shimmer 1.5s infinite;
	}

	.skeleton-text {
		border-radius: var(--radius-sm);
	}

	.skeleton-rectangular {
		border-radius: var(--radius-md);
	}

	.skeleton-circular {
		border-radius: 50%;
	}

	.skeleton-lines {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	@keyframes shimmer {
		0% {
			background-position: -200% 0;
		}
		100% {
			background-position: 200% 0;
		}
	}
</style>
