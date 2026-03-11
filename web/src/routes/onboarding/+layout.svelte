<script lang="ts">
	import { goto } from '$app/navigation';
	import { isTauri } from '$lib/tauri/commands';
	import { onMount } from 'svelte';

	let { children } = $props();

	// Redirect Tauri app to dashboard - onboarding is website-only
	onMount(() => {
		if (isTauri()) {
			goto('/dashboard');
		}
	});
</script>

{#if !isTauri()}
	{@render children()}
{/if}
