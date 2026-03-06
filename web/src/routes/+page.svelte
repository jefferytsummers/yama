<script lang="ts">
	import ServiceCard from '../components/ServiceCard.svelte';
	import MetricsPanel from '../components/MetricsPanel.svelte';
	import VideoPlaceholder from '../components/VideoPlaceholder.svelte';
	import Console from '../components/Console.svelte';
	import { services, videoSources } from '$lib/stores';
</script>

<svelte:head>
	<title>Dashboard - Yama Host UI</title>
</svelte:head>

<div class="dashboard">
	<section class="section overview-grid">
		<div>
			<h2>System Overview</h2>
			<MetricsPanel />
		</div>
		<div>
			<Console />
		</div>
	</section>

	<section class="section">
		<div class="section-header">
			<h2>Services</h2>
			<a href="/services" class="view-all">View all</a>
		</div>
		<div class="services-grid">
			{#each $services as service (service.name)}
				<ServiceCard {service} />
			{:else}
				<p class="empty">No services configured</p>
			{/each}
		</div>
	</section>

	<section class="section">
		<div class="section-header">
			<h2>Video Sources</h2>
		</div>
		<div class="video-grid">
			{#each $videoSources as source (source.name)}
				<VideoPlaceholder {source} />
			{:else}
				<p class="empty">No video sources configured</p>
			{/each}
		</div>
	</section>
</div>

<style>
	.dashboard {
		display: flex;
		flex-direction: column;
		gap: 2rem;
	}

	.section {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.section-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.section h2 {
		margin: 0;
		font-size: 1.25rem;
	}

	.view-all {
		font-size: 0.875rem;
	}

	.services-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
		gap: 1rem;
	}

	.video-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(400px, 1fr));
		gap: 1.5rem;
	}

	.empty {
		color: var(--color-text-muted);
		font-style: italic;
		padding: 2rem;
		text-align: center;
		background-color: var(--color-surface);
		border-radius: var(--radius);
	}

	.overview-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1.5rem;
	}

	.overview-grid h2 {
		margin-bottom: 1rem;
	}

	@media (max-width: 900px) {
		.overview-grid {
			grid-template-columns: 1fr;
		}
	}
</style>
