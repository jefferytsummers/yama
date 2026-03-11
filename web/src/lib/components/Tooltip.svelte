<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		content: string;
		position?: 'top' | 'bottom' | 'left' | 'right';
		children: Snippet;
	}

	let {
		content,
		position = 'top',
		children
	}: Props = $props();

	let visible = $state(false);

	function show() {
		visible = true;
	}

	function hide() {
		visible = false;
	}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="tooltip-wrapper"
	onmouseenter={show}
	onmouseleave={hide}
	onfocus={show}
	onblur={hide}
>
	{@render children()}
	{#if visible && content}
		<div class="tooltip tooltip-{position}" role="tooltip">
			{content}
		</div>
	{/if}
</div>

<style>
	.tooltip-wrapper {
		position: relative;
		display: inline-flex;
	}

	.tooltip {
		position: absolute;
		z-index: 50;
		padding: var(--space-1) var(--space-2);
		font-size: var(--text-small);
		font-weight: 500;
		color: var(--color-chalk);
		background: var(--color-graphite);
		border: 1px solid var(--color-stone);
		border-radius: var(--radius-sm);
		white-space: nowrap;
		pointer-events: none;
		animation: fadeIn var(--duration-fast) var(--ease-out);
	}

	.tooltip-top {
		bottom: calc(100% + 8px);
		left: 50%;
		transform: translateX(-50%);
	}

	.tooltip-bottom {
		top: calc(100% + 8px);
		left: 50%;
		transform: translateX(-50%);
	}

	.tooltip-left {
		right: calc(100% + 8px);
		top: 50%;
		transform: translateY(-50%);
	}

	.tooltip-right {
		left: calc(100% + 8px);
		top: 50%;
		transform: translateY(-50%);
	}

	@keyframes fadeIn {
		from {
			opacity: 0;
		}
		to {
			opacity: 1;
		}
	}
</style>
