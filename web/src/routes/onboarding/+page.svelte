<script lang="ts">
	import { goto } from '$app/navigation';
	import { isAuthenticated } from '$lib/stores';
	import type { ProjectData } from '$lib/types';
	import ProjectWizard from '$lib/components/ProjectWizard.svelte';

	// Redirect unauthenticated users
	$effect(() => {
		if (!$isAuthenticated) {
			goto('/signup');
		}
	});

	function handleComplete(data: ProjectData) {
		// Store project data in sessionStorage for the success page
		sessionStorage.setItem('onboarding_project', JSON.stringify(data));
		goto('/onboarding/success');
	}

	function handleCancel() {
		goto('/');
	}
</script>

<svelte:head>
	<title>Create Your First Project - Yama</title>
</svelte:head>

<div class="onboarding-page">
	<div class="onboarding-header">
		<a href="/" class="logo" aria-label="Go to home page">
			<svg width="32" height="32" viewBox="0 0 64 64" fill="none">
				<path d="M32 8L8 52h48L32 8z" stroke="currentColor" stroke-width="2" fill="none" />
				<path d="M32 18L14 48h36L32 18z" fill="currentColor" opacity="0.2" />
			</svg>
			<span>YAMA</span>
		</a>
	</div>

	<div class="onboarding-content">
		<ProjectWizard
			context="onboarding"
			onComplete={handleComplete}
			onCancel={handleCancel}
		/>
	</div>
</div>

<style>
	.onboarding-page {
		display: flex;
		flex-direction: column;
		min-height: 100vh;
		background: var(--color-obsidian);
	}

	.onboarding-header {
		display: flex;
		align-items: center;
		padding: var(--space-4) var(--space-6);
		border-bottom: 1px solid var(--color-stone);
	}

	.logo {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		color: var(--color-amber);
		text-decoration: none;
		font-size: var(--text-h3);
		font-weight: 700;
		letter-spacing: 0.05em;
	}

	.onboarding-content {
		flex: 1;
		display: flex;
		flex-direction: column;
	}
</style>
