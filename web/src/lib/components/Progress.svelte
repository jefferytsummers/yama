<script lang="ts">
	interface Props {
		value?: number;
		max?: number;
		indeterminate?: boolean;
		variant?: 'default' | 'gradient';
		size?: 'sm' | 'md' | 'lg';
		showLabel?: boolean;
	}

	let {
		value = 0,
		max = 100,
		indeterminate = false,
		variant = 'default',
		size = 'md',
		showLabel = false
	}: Props = $props();

	const percentage = $derived(Math.min(Math.max((value / max) * 100, 0), 100));
</script>

<div class="progress-wrapper">
	<div
		class="progress progress-{size}"
		role="progressbar"
		aria-valuenow={indeterminate ? undefined : value}
		aria-valuemin={0}
		aria-valuemax={max}
	>
		<div
			class="progress-fill"
			class:indeterminate
			class:gradient={variant === 'gradient'}
			style={indeterminate ? '' : `width: ${percentage}%`}
		></div>
	</div>
	{#if showLabel && !indeterminate}
		<span class="progress-label">{Math.round(percentage)}%</span>
	{/if}
</div>

<style>
	.progress-wrapper {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.progress {
		flex: 1;
		background: var(--color-stone);
		border-radius: var(--radius-full);
		overflow: hidden;
	}

	/* Sizes */
	.progress-sm {
		height: 4px;
	}
	.progress-md {
		height: 6px;
	}
	.progress-lg {
		height: 8px;
	}

	.progress-fill {
		height: 100%;
		background: var(--color-amber);
		border-radius: var(--radius-full);
		transition: width var(--duration-normal) var(--ease-out);
	}

	.progress-fill.gradient {
		background: var(--gradient-magma);
	}

	.progress-fill.indeterminate {
		width: 30%;
		animation: progressIndeterminate 1.5s ease-in-out infinite;
	}

	.progress-label {
		font-size: var(--text-small);
		font-weight: 500;
		color: var(--color-silver);
		min-width: 36px;
		text-align: right;
		font-variant-numeric: tabular-nums;
	}

	@keyframes progressIndeterminate {
		0% {
			transform: translateX(-100%);
		}
		50% {
			transform: translateX(200%);
		}
		100% {
			transform: translateX(400%);
		}
	}
</style>
