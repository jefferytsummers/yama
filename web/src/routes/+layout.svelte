<script lang="ts">
	import { onMount } from 'svelte';
	import '../app.css';
	import type { Snippet } from 'svelte';
	import { auth, resolvedTheme, initThemeListener, applyTheme } from '$lib/stores';

	interface Props {
		children: Snippet;
	}

	let { children }: Props = $props();

	// Initialize auth state and theme listener
	onMount(() => {
		auth.init();
		const cleanup = initThemeListener();
		return cleanup;
	});

	// Apply theme whenever it changes
	$effect(() => {
		applyTheme($resolvedTheme);
	});
</script>

<div class="app">
	{@render children()}
</div>

<style>
	.app {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
		background: var(--color-bg-primary);
		color: var(--color-text-primary);
		transition: background-color var(--duration-normal) var(--ease-out),
		            color var(--duration-normal) var(--ease-out);
	}
</style>
