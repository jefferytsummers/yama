<script lang="ts">
	import { theme, resolvedTheme, type Theme } from '$lib/stores';

	interface Props {
		variant?: 'icon' | 'full';
		size?: 'sm' | 'md' | 'lg';
	}

	let { variant = 'icon', size = 'md' }: Props = $props();

	function toggleTheme() {
		// Simple toggle: flip to the opposite of current resolved theme
		theme.set($resolvedTheme === 'dark' ? 'light' : 'dark');
	}

	function setTheme(value: Theme) {
		theme.set(value);
	}

	const sizeClasses = {
		sm: 'size-sm',
		md: 'size-md',
		lg: 'size-lg'
	};
</script>

{#if variant === 'icon'}
	<button
		class="theme-toggle {sizeClasses[size]}"
		onclick={toggleTheme}
		aria-label="Toggle theme (current: {$resolvedTheme})"
		title="Toggle to {$resolvedTheme === 'dark' ? 'light' : 'dark'} theme"
	>
		{#if $resolvedTheme === 'dark'}
			<!-- Moon icon -->
			<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
				<path d="M21 12.79A9 9 0 1111.21 3 7 7 0 0021 12.79z" />
			</svg>
		{:else}
			<!-- Sun icon -->
			<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
				<circle cx="12" cy="12" r="5" />
				<line x1="12" y1="1" x2="12" y2="3" />
				<line x1="12" y1="21" x2="12" y2="23" />
				<line x1="4.22" y1="4.22" x2="5.64" y2="5.64" />
				<line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
				<line x1="1" y1="12" x2="3" y2="12" />
				<line x1="21" y1="12" x2="23" y2="12" />
				<line x1="4.22" y1="19.78" x2="5.64" y2="18.36" />
				<line x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
			</svg>
		{/if}
		{#if $theme === 'system'}
			<span class="system-indicator"></span>
		{/if}
	</button>
{:else}
	<div class="theme-selector" role="radiogroup" aria-label="Theme selection">
		<button
			class="theme-option"
			class:active={$theme === 'light'}
			onclick={() => setTheme('light')}
			role="radio"
			aria-checked={$theme === 'light'}
		>
			<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
				<circle cx="12" cy="12" r="5" />
				<line x1="12" y1="1" x2="12" y2="3" />
				<line x1="12" y1="21" x2="12" y2="23" />
				<line x1="4.22" y1="4.22" x2="5.64" y2="5.64" />
				<line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
				<line x1="1" y1="12" x2="3" y2="12" />
				<line x1="21" y1="12" x2="23" y2="12" />
				<line x1="4.22" y1="19.78" x2="5.64" y2="18.36" />
				<line x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
			</svg>
			<span>Light</span>
		</button>
		<button
			class="theme-option"
			class:active={$theme === 'dark'}
			onclick={() => setTheme('dark')}
			role="radio"
			aria-checked={$theme === 'dark'}
		>
			<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
				<path d="M21 12.79A9 9 0 1111.21 3 7 7 0 0021 12.79z" />
			</svg>
			<span>Dark</span>
		</button>
		<button
			class="theme-option"
			class:active={$theme === 'system'}
			onclick={() => setTheme('system')}
			role="radio"
			aria-checked={$theme === 'system'}
		>
			<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
				<rect x="2" y="3" width="20" height="14" rx="2" />
				<line x1="8" y1="21" x2="16" y2="21" />
				<line x1="12" y1="17" x2="12" y2="21" />
			</svg>
			<span>System</span>
		</button>
	</div>
{/if}

<style>
	.theme-toggle {
		position: relative;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--color-bg-tertiary);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-text-secondary);
		cursor: pointer;
		transition: all var(--duration-fast) var(--ease-out);
	}

	.theme-toggle:hover {
		background: var(--color-bg-hover);
		color: var(--color-text-primary);
		border-color: var(--color-text-muted);
	}

	.theme-toggle:focus-visible {
		outline: 2px solid var(--color-primary);
		outline-offset: 2px;
	}

	.theme-toggle svg {
		width: 100%;
		height: 100%;
	}

	.size-sm {
		width: 28px;
		height: 28px;
		padding: 5px;
	}

	.size-md {
		width: 36px;
		height: 36px;
		padding: 7px;
	}

	.size-lg {
		width: 44px;
		height: 44px;
		padding: 9px;
	}

	.system-indicator {
		position: absolute;
		bottom: 2px;
		right: 2px;
		width: 6px;
		height: 6px;
		background: var(--color-primary);
		border-radius: 50%;
	}

	/* Full selector variant */
	.theme-selector {
		display: flex;
		gap: var(--space-1);
		padding: var(--space-1);
		background: var(--color-bg-secondary);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
	}

	.theme-option {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
		background: transparent;
		border: none;
		border-radius: var(--radius-md);
		color: var(--color-text-tertiary);
		font-family: var(--font-sans);
		font-size: var(--text-small);
		cursor: pointer;
		transition: all var(--duration-fast) var(--ease-out);
	}

	.theme-option:hover {
		color: var(--color-text-secondary);
		background: var(--color-bg-hover);
	}

	.theme-option.active {
		color: var(--color-text-primary);
		background: var(--color-bg-tertiary);
	}

	.theme-option svg {
		width: 16px;
		height: 16px;
	}
</style>
