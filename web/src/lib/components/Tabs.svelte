<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Tab {
		id: string;
		label: string;
		disabled?: boolean;
	}

	interface Props {
		tabs: Tab[];
		activeTab?: string;
		onchange?: (tabId: string) => void;
		children?: Snippet<[string]>;
	}

	let {
		tabs,
		activeTab = $bindable(tabs[0]?.id ?? ''),
		onchange,
		children
	}: Props = $props();

	function selectTab(tabId: string) {
		activeTab = tabId;
		onchange?.(tabId);
	}
</script>

<div class="tabs-container">
	<div class="tabs-list" role="tablist">
		{#each tabs as tab}
			<button
				class="tab"
				class:active={activeTab === tab.id}
				disabled={tab.disabled}
				role="tab"
				aria-selected={activeTab === tab.id}
				aria-controls="panel-{tab.id}"
				onclick={() => selectTab(tab.id)}
			>
				{tab.label}
			</button>
		{/each}
	</div>
	{#if children}
		<div class="tabs-panel" role="tabpanel" id="panel-{activeTab}">
			{@render children(activeTab)}
		</div>
	{/if}
</div>

<style>
	.tabs-container {
		display: flex;
		flex-direction: column;
	}

	.tabs-list {
		display: flex;
		gap: var(--space-1);
		padding: var(--space-1);
		background: var(--color-basalt);
		border-radius: var(--radius-lg);
		width: fit-content;
	}

	.tab {
		padding: var(--space-2) var(--space-4);
		font-family: var(--font-sans);
		font-size: var(--text-body);
		font-weight: 500;
		color: var(--color-silver);
		background: transparent;
		border: none;
		border-radius: var(--radius-md);
		cursor: pointer;
		transition:
			color var(--duration-fast) var(--ease-out),
			background-color var(--duration-fast) var(--ease-out);
	}

	.tab:hover:not(:disabled):not(.active) {
		color: var(--color-chalk);
		background: var(--color-slate);
	}

	.tab.active {
		color: var(--color-chalk);
		background: var(--color-slate);
	}

	.tab:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.tabs-panel {
		padding: var(--space-4) 0;
	}
</style>
