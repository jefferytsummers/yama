<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		title: string;
		subtitle: string;
		variant?: 'default' | 'gradient' | 'dark';
		layout?: 'centered' | 'split' | 'split-reverse';
		visual?: Snippet;
		children: Snippet;
	}

	let {
		title,
		subtitle,
		variant = 'default',
		layout = 'centered',
		visual,
		children
	}: Props = $props();
</script>

<section class="value-section {variant} {layout}">
	<div class="section-container">
		{#if layout === 'centered'}
			<div class="section-header">
				<h2 class="section-title">{title}</h2>
				<p class="section-subtitle">{subtitle}</p>
			</div>
			<div class="section-content">
				{@render children()}
			</div>
			{#if visual}
				<div class="section-visual">
					{@render visual()}
				</div>
			{/if}
		{:else}
			<div class="section-text">
				<h2 class="section-title">{title}</h2>
				<p class="section-subtitle">{subtitle}</p>
				<div class="section-content">
					{@render children()}
				</div>
			</div>
			{#if visual}
				<div class="section-visual">
					{@render visual()}
				</div>
			{/if}
		{/if}
	</div>
</section>

<style>
	.value-section {
		padding: var(--space-12) var(--space-6);
	}

	.value-section.default {
		background: var(--color-bg-primary);
	}

	.value-section.gradient {
		background: var(--gradient-depth);
	}

	.value-section.dark {
		background: var(--color-bg-secondary);
	}

	.section-container {
		max-width: 1200px;
		margin: 0 auto;
	}

	/* Centered layout */
	.centered .section-header {
		text-align: center;
		margin-bottom: var(--space-10);
	}

	.centered .section-content {
		text-align: center;
	}

	.centered .section-visual {
		margin-top: var(--space-10);
	}

	/* Split layouts */
	.split .section-container,
	.split-reverse .section-container {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-12);
		align-items: center;
	}

	.split-reverse .section-text {
		order: 2;
	}

	.split-reverse .section-visual {
		order: 1;
	}

	.section-title {
		font-size: var(--text-h1);
		font-weight: 600;
		color: var(--color-text-primary);
		margin: 0 0 var(--space-2);
	}

	.section-subtitle {
		font-size: var(--text-body);
		color: var(--color-text-secondary);
		margin: 0 0 var(--space-6);
		line-height: var(--leading-relaxed);
	}

	.split .section-subtitle,
	.split-reverse .section-subtitle {
		max-width: 480px;
	}

	@media (max-width: 900px) {
		.split .section-container,
		.split-reverse .section-container {
			grid-template-columns: 1fr;
			gap: var(--space-8);
		}

		.split-reverse .section-text {
			order: 1;
		}

		.split-reverse .section-visual {
			order: 2;
		}

		.section-title {
			font-size: var(--text-h2);
		}
	}

	@media (max-width: 768px) {
		.value-section {
			padding: var(--space-10) var(--space-4);
		}
	}
</style>
