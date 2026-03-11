<script lang="ts">
	import type { AgentPreset } from '$lib/types';
	import { Badge } from '$lib/components';
	import ModelChip from './ModelChip.svelte';

	interface Props {
		preset: AgentPreset;
		selected?: boolean;
		expanded?: boolean;
		onSelect: () => void;
		onToggleExpand: () => void;
	}

	let { preset, selected = false, expanded = false, onSelect, onToggleExpand }: Props = $props();

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' || e.key === ' ') {
			e.preventDefault();
			onSelect();
		}
	}
</script>

<div
	class="preset-card"
	class:selected
	class:expanded
	role="radio"
	aria-checked={selected}
	tabindex="0"
	onclick={onSelect}
	onkeydown={handleKeydown}
>
	<div class="preset-header">
		<div class="preset-title-row">
			<div class="radio-indicator">
				<div class="radio-inner"></div>
			</div>
			<h3 class="preset-name">{preset.name}</h3>
			{#if preset.badge}
				<Badge
					variant={preset.badge === 'RECOMMENDED' ? 'success' : preset.badge === 'FAST' ? 'info' : 'warning'}
					size="sm"
				>
					{preset.badge}
				</Badge>
			{/if}
		</div>
		<p class="preset-tagline">{preset.tagline}</p>
	</div>

	<button
		type="button"
		class="expand-button"
		onclick={(e) => { e.stopPropagation(); onToggleExpand(); }}
		aria-expanded={expanded}
		aria-label={expanded ? 'Collapse details' : 'Expand details'}
	>
		<span class="expand-text">{expanded ? 'Less' : 'Details'}</span>
		<svg
			width="16"
			height="16"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="2"
			class="expand-icon"
			class:rotated={expanded}
		>
			<polyline points="6 9 12 15 18 9" />
		</svg>
	</button>

	{#if expanded}
		<div class="preset-details">
			<div class="details-section">
				<h4>Capabilities</h4>
				<ul class="capabilities-list">
					{#each preset.capabilities as cap}
						<li>
							<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
								<polyline points="20 6 9 17 4 12" />
							</svg>
							{cap}
						</li>
					{/each}
				</ul>
			</div>

			{#if preset.models.length > 0}
				<div class="details-section">
					<h4>Models Included</h4>
					<div class="models-grid">
						{#each preset.models as model}
							<ModelChip {model} />
						{/each}
					</div>
				</div>
			{/if}

			<div class="details-footer">
				<span class="detail-item">
					<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4M7 10l5 5 5-5M12 15V3" />
					</svg>
					{preset.downloadSize}
				</span>
				<span class="detail-item">
					<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<rect x="4" y="4" width="16" height="16" rx="2" />
						<rect x="9" y="9" width="6" height="6" />
					</svg>
					{preset.requirements}
				</span>
			</div>
		</div>
	{/if}
</div>

<style>
	.preset-card {
		position: relative;
		padding: var(--space-4);
		background: var(--color-slate);
		border: 2px solid var(--color-stone);
		border-radius: var(--radius-xl);
		cursor: pointer;
		transition: all var(--duration-fast) var(--ease-out);
	}

	.preset-card:hover {
		background: var(--color-graphite);
		border-color: var(--color-silver);
	}

	.preset-card:focus-visible {
		outline: none;
		border-color: var(--color-amber);
		box-shadow: var(--shadow-glow);
	}

	.preset-card.selected {
		border-color: var(--color-amber);
		background: rgba(245, 158, 11, 0.05);
	}

	.preset-header {
		margin-bottom: var(--space-3);
	}

	.preset-title-row {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		margin-bottom: var(--space-2);
	}

	.radio-indicator {
		width: 20px;
		height: 20px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--color-basalt);
		border: 2px solid var(--color-stone);
		border-radius: 50%;
		flex-shrink: 0;
		transition: all var(--duration-fast) var(--ease-out);
	}

	.preset-card.selected .radio-indicator {
		border-color: var(--color-amber);
	}

	.radio-inner {
		width: 10px;
		height: 10px;
		background: var(--color-amber);
		border-radius: 50%;
		transform: scale(0);
		transition: transform var(--duration-fast) var(--ease-out);
	}

	.preset-card.selected .radio-inner {
		transform: scale(1);
	}

	.preset-name {
		font-size: var(--text-h3);
		font-weight: 600;
		color: var(--color-chalk);
		margin: 0;
	}

	.preset-tagline {
		font-size: var(--text-small);
		color: var(--color-silver);
		margin: 0;
		line-height: var(--leading-relaxed);
	}

	.expand-button {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		padding: var(--space-1) var(--space-2);
		font-family: var(--font-sans);
		font-size: var(--text-small);
		color: var(--color-amber);
		background: transparent;
		border: none;
		cursor: pointer;
		transition: opacity var(--duration-fast) var(--ease-out);
	}

	.expand-button:hover {
		opacity: 0.8;
	}

	.expand-icon {
		transition: transform var(--duration-fast) var(--ease-out);
	}

	.expand-icon.rotated {
		transform: rotate(180deg);
	}

	.preset-details {
		margin-top: var(--space-4);
		padding-top: var(--space-4);
		border-top: 1px solid var(--color-stone);
	}

	.details-section {
		margin-bottom: var(--space-4);
	}

	.details-section h4 {
		font-size: var(--text-small);
		font-weight: 600;
		color: var(--color-ash);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		margin: 0 0 var(--space-2);
	}

	.capabilities-list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.capabilities-list li {
		display: flex;
		align-items: flex-start;
		gap: var(--space-2);
		font-size: var(--text-small);
		color: var(--color-silver);
	}

	.capabilities-list li svg {
		flex-shrink: 0;
		color: var(--color-jade);
		margin-top: 2px;
	}

	.models-grid {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.details-footer {
		display: flex;
		gap: var(--space-6);
		padding-top: var(--space-3);
		border-top: 1px solid var(--color-stone);
	}

	.detail-item {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--text-small);
		color: var(--color-ash);
	}

	.detail-item svg {
		color: var(--color-silver);
	}
</style>
