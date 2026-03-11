<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import '../app.css';
	import type { Snippet } from 'svelte';
	import { auth, resolvedTheme, initThemeListener, applyTheme } from '$lib/stores';
	import { isTauri } from '$lib/tauri/commands';
	import SplashScreen from '$lib/components/SplashScreen.svelte';

	interface Props {
		children: Snippet;
	}

	let { children }: Props = $props();

	// Start with everything hidden until we know what context we're in
	let contextChecked = $state(false);
	let inTauri = $state(false);
	let showSplash = $state(false);

	// Website-only routes that Tauri should skip
	const websiteOnlyRoutes = ['/', '/signup', '/login', '/onboarding'];

	function isWebsiteOnlyRoute(path: string): boolean {
		return websiteOnlyRoutes.some(route =>
			path === route || path.startsWith('/onboarding/')
		);
	}

	// Initialize auth state and theme listener
	onMount(() => {
		auth.init();
		const cleanup = initThemeListener();

		// Check if we're in Tauri
		inTauri = isTauri();

		if (inTauri) {
			// Show splash screen, keep app hidden
			showSplash = true;
		} else {
			// Not Tauri, show app immediately
			contextChecked = true;
		}

		return cleanup;
	});

	// Handle splash screen completion
	function handleSplashReady() {
		showSplash = false;
		contextChecked = true;

		// Redirect to dashboard if on a website-only route
		if (inTauri && isWebsiteOnlyRoute($page.url.pathname)) {
			goto('/dashboard');
		}
	}

	// Apply theme whenever it changes
	$effect(() => {
		applyTheme($resolvedTheme);
	});
</script>

{#if showSplash}
	<SplashScreen onReady={handleSplashReady} />
{/if}

<div class="app" class:visible={contextChecked}>
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
