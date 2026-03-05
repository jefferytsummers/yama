<script lang="ts">
	import ServiceCard from '../../components/ServiceCard.svelte';
	import { services, runningServicesCount, totalServicesCount } from '$lib/stores';
</script>

<svelte:head>
	<title>Services - Yama Host UI</title>
</svelte:head>

<div class="services-page">
	<header class="page-header">
		<h1>Services</h1>
		<p class="subtitle">
			{$runningServicesCount} of {$totalServicesCount} services running
		</p>
	</header>

	<div class="services-grid">
		{#each $services as service (service.name)}
			<ServiceCard {service} />
		{:else}
			<div class="empty-state card">
				<h3>No Services</h3>
				<p>No services are configured. Add services in the orchestrator configuration.</p>
			</div>
		{/each}
	</div>
</div>

<style>
	.services-page {
		display: flex;
		flex-direction: column;
		gap: 2rem;
	}

	.page-header {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.page-header h1 {
		margin: 0;
	}

	.subtitle {
		color: var(--color-text-muted);
		margin: 0;
	}

	.services-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
		gap: 1rem;
	}

	.empty-state {
		grid-column: 1 / -1;
		text-align: center;
		padding: 3rem;
	}

	.empty-state h3 {
		margin-bottom: 0.5rem;
	}

	.empty-state p {
		color: var(--color-text-muted);
		margin: 0;
	}
</style>
