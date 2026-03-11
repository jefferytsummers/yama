<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		open?: boolean;
		title?: string;
		width?: string;
		onclose?: () => void;
		children: Snippet;
		footer?: Snippet;
	}

	let {
		open = $bindable(false),
		title = '',
		width = '420px',
		onclose,
		children,
		footer
	}: Props = $props();

	function handleBackdropClick(e: MouseEvent) {
		if (e.target === e.currentTarget) {
			onclose?.();
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			onclose?.();
		}
	}
</script>

<svelte:window onkeydown={open ? handleKeydown : undefined} />

{#if open}
	<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
	<div class="modal-backdrop animate-fade-in" onclick={handleBackdropClick}>
		<div class="modal animate-scale-in" style="width: {width}">
			<header class="modal-header">
				<h2 class="modal-title">{title}</h2>
				<button class="modal-close" onclick={onclose} aria-label="Close modal">
					<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<path d="M18 6L6 18M6 6l12 12" />
					</svg>
				</button>
			</header>
			<div class="modal-body">
				{@render children()}
			</div>
			{#if footer}
				<footer class="modal-footer">
					{@render footer()}
				</footer>
			{/if}
		</div>
	</div>
{/if}

<style>
	.modal-backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.7);
		backdrop-filter: blur(4px);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 100;
		padding: var(--space-6);
	}

	.modal {
		background: var(--color-slate);
		border: 1px solid var(--color-stone);
		border-radius: var(--radius-xl);
		max-width: calc(100vw - 48px);
		max-height: calc(100vh - 48px);
		overflow: hidden;
		display: flex;
		flex-direction: column;
		box-shadow: var(--shadow-medium);
	}

	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: var(--space-4) var(--space-6);
		border-bottom: 1px solid var(--color-stone);
	}

	.modal-title {
		font-size: var(--text-h2);
		font-weight: 600;
		color: var(--color-chalk);
	}

	.modal-close {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 32px;
		background: none;
		border: none;
		border-radius: var(--radius-sm);
		color: var(--color-ash);
		cursor: pointer;
		transition: color var(--duration-fast) var(--ease-out),
			background-color var(--duration-fast) var(--ease-out);
	}

	.modal-close:hover {
		color: var(--color-chalk);
		background: var(--color-graphite);
	}

	.modal-body {
		flex: 1;
		padding: var(--space-6);
		overflow-y: auto;
	}

	.modal-footer {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: var(--space-3);
		padding: var(--space-4) var(--space-6);
		border-top: 1px solid var(--color-stone);
	}
</style>
