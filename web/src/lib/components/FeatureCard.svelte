<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		icon: Snippet;
		title: string;
		description: string;
	}

	let { icon, title, description }: Props = $props();
</script>

<div class="feature-card">
	<div class="feature-icon">
		{@render icon()}
	</div>
	<h3 class="feature-title">{title}</h3>
	<p class="feature-description">{description}</p>
</div>

<style>
	.feature-card {
		display: flex;
		flex-direction: column;
		align-items: center;
		text-align: center;
		padding: var(--space-6);
		background: var(--color-bg-secondary);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-xl);
		transition: all var(--duration-normal) var(--ease-out);
	}

	.feature-card:hover {
		background: var(--color-bg-tertiary);
		border-color: var(--color-primary);
		transform: translateY(-4px);
	}

	.feature-icon {
		width: 56px;
		height: 56px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: color-mix(in srgb, var(--color-primary) 10%, transparent);
		border-radius: var(--radius-lg);
		color: var(--color-primary);
		margin-bottom: var(--space-4);
	}

	.feature-title {
		font-size: var(--text-h3);
		font-weight: 600;
		color: var(--color-text-primary);
		margin: 0 0 var(--space-2);
	}

	.feature-description {
		font-size: var(--text-body);
		color: var(--color-text-secondary);
		line-height: var(--leading-relaxed);
		margin: 0;
	}
</style>
