<script lang="ts">
	import type { Tool } from '$lib/types';

	interface Props {
		tool: Tool;
		enabled?: boolean;
		disabled?: boolean;
		onToggle: (enabled: boolean) => void;
	}

	let { tool, enabled = false, disabled = false, onToggle }: Props = $props();

	function handleClick() {
		if (!disabled) {
			onToggle(!enabled);
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if ((e.key === 'Enter' || e.key === ' ') && !disabled) {
			e.preventDefault();
			onToggle(!enabled);
		}
	}
</script>

<div
	class="tool-toggle"
	class:enabled
	class:disabled
	onclick={handleClick}
	onkeydown={handleKeydown}
	role="switch"
	aria-checked={enabled}
	tabindex={disabled ? -1 : 0}
>
	<div class="toggle-content">
		<div class="toggle-header">
			<span class="toggle-name">{tool.name}</span>
			<div class="toggle-switch">
				<div class="toggle-track">
					<div class="toggle-thumb"></div>
				</div>
			</div>
		</div>
		<p class="toggle-description">{tool.description}</p>
	</div>
</div>

<style>
	.tool-toggle {
		display: flex;
		padding: var(--space-4);
		background: var(--color-slate);
		border: 1px solid var(--color-stone);
		border-radius: var(--radius-lg);
		cursor: pointer;
		transition: all var(--duration-fast) var(--ease-out);
		user-select: none;
	}

	.tool-toggle:hover:not(.disabled) {
		background: var(--color-graphite);
		border-color: var(--color-silver);
	}

	.tool-toggle:focus-visible {
		outline: none;
		border-color: var(--color-amber);
		box-shadow: var(--shadow-glow);
	}

	.tool-toggle.enabled {
		border-color: var(--color-jade);
		background: rgba(16, 185, 129, 0.1);
	}

	.tool-toggle.disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.toggle-content {
		flex: 1;
	}

	.toggle-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: var(--space-1);
	}

	.toggle-name {
		font-size: var(--text-body);
		font-weight: 500;
		color: var(--color-chalk);
	}

	.toggle-description {
		font-size: var(--text-small);
		color: var(--color-silver);
		margin: 0;
		line-height: var(--leading-relaxed);
	}

	.toggle-switch {
		flex-shrink: 0;
	}

	.toggle-track {
		width: 40px;
		height: 22px;
		padding: 2px;
		background: var(--color-stone);
		border-radius: 11px;
		transition: background var(--duration-fast) var(--ease-out);
	}

	.tool-toggle.enabled .toggle-track {
		background: var(--color-jade);
	}

	.toggle-thumb {
		width: 18px;
		height: 18px;
		background: var(--color-chalk);
		border-radius: 50%;
		transition: transform var(--duration-fast) var(--ease-out);
	}

	.tool-toggle.enabled .toggle-thumb {
		transform: translateX(18px);
	}
</style>
