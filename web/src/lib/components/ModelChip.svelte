<script lang="ts">
	import type { ModelInfo } from '$lib/types';

	interface Props {
		model: ModelInfo;
		compact?: boolean;
	}

	let { model, compact = false }: Props = $props();
</script>

{#if compact}
	<span class="model-chip compact">
		<span class="model-icon">{model.icon}</span>
		<span class="model-name">{model.name}</span>
	</span>
{:else}
	<div class="model-chip">
		<span class="model-icon">{model.icon}</span>
		<div class="model-info">
			<span class="model-name">{model.name}</span>
			<span class="model-role">{model.role}</span>
		</div>
	</div>
{/if}

<style>
	.model-chip {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
		background: var(--color-basalt);
		border: 1px solid var(--color-stone);
		border-radius: var(--radius-md);
	}

	.model-chip.compact {
		display: inline-flex;
		padding: var(--space-1) var(--space-2);
		gap: var(--space-1);
	}

	.model-icon {
		font-size: 1.25rem;
		line-height: 1;
	}

	.model-chip.compact .model-icon {
		font-size: 1rem;
	}

	.model-info {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.model-name {
		font-size: var(--text-small);
		font-weight: 500;
		color: var(--color-chalk);
	}

	.model-chip.compact .model-name {
		font-size: var(--text-small);
		font-weight: 400;
	}

	.model-role {
		font-size: 11px;
		color: var(--color-ash);
	}
</style>
