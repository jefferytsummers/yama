<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import '../app.css';
	import Navbar from '../components/Navbar.svelte';
	import { services, metrics, config, videoSources, connectionStatus, hostStatus } from '$lib/stores';
	import { api } from '$lib/api';

	let { children } = $props();

	onMount(async () => {
		// Check API health
		try {
			await api.health();
			connectionStatus.set('connected');
		} catch {
			connectionStatus.set('disconnected');
		}

		// Start polling
		services.startPolling();
		metrics.startPolling();
		hostStatus.startPolling();

		// Load initial data
		config.refresh();
		videoSources.refresh();
	});

	onDestroy(() => {
		services.stopPolling();
		metrics.stopPolling();
		hostStatus.stopPolling();
	});
</script>

<div class="app">
	<Navbar />
	<main>
		{@render children()}
	</main>
</div>

<style>
	.app {
		display: flex;
		flex-direction: column;
		min-height: 100vh;
	}

	main {
		flex: 1;
		padding: 2rem;
		max-width: 1400px;
		margin: 0 auto;
		width: 100%;
	}
</style>
