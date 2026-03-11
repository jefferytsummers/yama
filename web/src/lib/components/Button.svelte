<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		variant?: 'primary' | 'secondary' | 'ghost' | 'destructive';
		size?: 'sm' | 'md' | 'lg';
		disabled?: boolean;
		loading?: boolean;
		fullWidth?: boolean;
		type?: 'button' | 'submit' | 'reset';
		onclick?: (e: MouseEvent) => void;
		children: Snippet;
	}

	let {
		variant = 'primary',
		size = 'md',
		disabled = false,
		loading = false,
		fullWidth = false,
		type = 'button',
		onclick,
		children
	}: Props = $props();

	const isDisabled = $derived(disabled || loading);
</script>

<button class="btn btn-{variant} btn-{size}" class:full-width={fullWidth} {type} disabled={isDisabled} {onclick}>
	{#if loading}
		<span class="spinner"></span>
	{/if}
	<span class="btn-content" class:loading>
		{@render children()}
	</span>
</button>

<style>
	.btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-2);
		font-family: var(--font-sans);
		font-weight: 500;
		border: none;
		border-radius: var(--radius-md);
		cursor: pointer;
		transition:
			background-color var(--duration-fast) var(--ease-out),
			transform var(--duration-fast) var(--ease-out),
			box-shadow var(--duration-fast) var(--ease-out);
		white-space: nowrap;
		user-select: none;
	}

	.btn:active:not(:disabled) {
		transform: scale(0.98);
	}

	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	/* Sizes */
	.btn-sm {
		height: 32px;
		padding: 0 var(--space-3);
		font-size: var(--text-small);
	}

	.btn-md {
		height: 40px;
		padding: 0 var(--space-4);
		font-size: var(--text-body);
	}

	.btn-lg {
		height: 48px;
		padding: 0 var(--space-6);
		font-size: var(--text-h3);
	}

	.full-width {
		width: 100%;
	}

	/* Primary */
	.btn-primary {
		background: var(--color-amber);
		color: var(--color-obsidian);
	}

	.btn-primary:hover:not(:disabled) {
		background: var(--color-amber-hover);
	}

	.btn-primary:focus-visible {
		box-shadow: var(--shadow-glow);
	}

	/* Secondary */
	.btn-secondary {
		background: transparent;
		color: var(--color-silver);
		border: 1px solid var(--color-stone);
	}

	.btn-secondary:hover:not(:disabled) {
		background: var(--color-graphite);
		color: var(--color-chalk);
	}

	/* Ghost */
	.btn-ghost {
		background: transparent;
		color: var(--color-silver);
	}

	.btn-ghost:hover:not(:disabled) {
		background: var(--color-slate);
		color: var(--color-chalk);
	}

	/* Destructive */
	.btn-destructive {
		background: var(--color-ember);
		color: var(--color-chalk);
	}

	.btn-destructive:hover:not(:disabled) {
		background: #dc2626;
	}

	.btn-destructive:focus-visible {
		box-shadow: var(--shadow-glow-error);
	}

	/* Loading state */
	.spinner {
		width: 16px;
		height: 16px;
		border: 2px solid currentColor;
		border-top-color: transparent;
		border-radius: 50%;
		animation: spin 0.6s linear infinite;
	}

	.btn-content.loading {
		opacity: 0.7;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
</style>
