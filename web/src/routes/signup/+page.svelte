<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { SignupForm } from '$lib/components';
	import { auth, isAuthenticated } from '$lib/stores';
	import { isTauri } from '$lib/tauri/commands';
	import type { OAuthProvider } from '$lib/types';

	let isLoading = $state(false);
	let error = $state<string | null>(null);

	// Tauri app goes straight to dashboard - signup is website-only
	onMount(() => {
		if (isTauri()) {
			goto('/dashboard');
		}
	});

	// Redirect authenticated users
	$effect(() => {
		if ($isAuthenticated) {
			goto('/onboarding');
		}
	});

	async function handleSubmit(data: { email: string; password: string; name: string }) {
		isLoading = true;
		error = null;

		try {
			await auth.signup({
				email: data.email,
				password: data.password,
				name: data.name || undefined
			});
			// Auth store will trigger redirect via $effect
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to create account. Please try again.';
		} finally {
			isLoading = false;
		}
	}

	function handleOAuth(provider: OAuthProvider) {
		// In a real app, this would initiate OAuth flow
		console.log(`OAuth with ${provider}`);
		error = `OAuth with ${provider} coming soon!`;
	}
</script>

<svelte:head>
	<title>Sign Up - Yama</title>
</svelte:head>

<div class="signup-page">
	<div class="signup-container">
		<div class="signup-header">
			<a href="/" class="logo-link" aria-label="Go to home page">
				<svg width="48" height="48" viewBox="0 0 64 64" fill="none">
					<path d="M32 8L8 52h48L32 8z" stroke="currentColor" stroke-width="2" fill="none" />
					<path d="M32 18L14 48h36L32 18z" fill="currentColor" opacity="0.2" />
				</svg>
			</a>
			<h1 class="signup-title">Create your account</h1>
			<p class="signup-subtitle">Start analyzing videos with AI-powered intelligence</p>
		</div>

		<SignupForm
			onSubmit={handleSubmit}
			onOAuth={handleOAuth}
			{isLoading}
			{error}
		/>

		<p class="login-link">
			Already have an account? <a href="/login">Sign in</a>
		</p>
	</div>

	<div class="signup-visual">
		<div class="visual-content">
			<div class="feature-highlight">
				<svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4M17 8l-5-5-5 5M12 3v12" />
				</svg>
				<h3>Import Any Video</h3>
				<p>Drop files or connect to streams</p>
			</div>
			<div class="feature-highlight">
				<svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<rect x="4" y="4" width="16" height="16" rx="2" />
					<rect x="9" y="9" width="6" height="6" />
					<path d="M9 1v3M15 1v3M9 20v3M15 20v3M20 9h3M20 14h3M1 9h3M1 14h3" />
				</svg>
				<h3>Local AI Processing</h3>
				<p>Your data never leaves your machine</p>
			</div>
			<div class="feature-highlight">
				<svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<path d="M21 15a2 2 0 01-2 2H7l-4 4V5a2 2 0 012-2h14a2 2 0 012 2z" />
				</svg>
				<h3>Ask Questions</h3>
				<p>Chat with your video content</p>
			</div>
		</div>
	</div>
</div>

<style>
	.signup-page {
		display: flex;
		min-height: 100vh;
		background: var(--color-obsidian);
	}

	.signup-container {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: var(--space-8);
		max-width: 500px;
		margin: 0 auto;
	}

	.signup-header {
		display: flex;
		flex-direction: column;
		align-items: center;
		text-align: center;
		margin-bottom: var(--space-8);
	}

	.logo-link {
		color: var(--color-amber);
		margin-bottom: var(--space-4);
		transition: transform var(--duration-fast) var(--ease-out);
	}

	.logo-link:hover {
		transform: scale(1.05);
	}

	.signup-title {
		font-size: var(--text-h1);
		font-weight: 600;
		color: var(--color-chalk);
		margin: 0 0 var(--space-2);
	}

	.signup-subtitle {
		font-size: var(--text-body);
		color: var(--color-silver);
		margin: 0;
	}

	.login-link {
		font-size: var(--text-small);
		color: var(--color-ash);
		margin-top: var(--space-6);
	}

	.login-link a {
		color: var(--color-amber);
		text-decoration: none;
	}

	.login-link a:hover {
		text-decoration: underline;
	}

	.signup-visual {
		display: none;
		flex: 1;
		background: linear-gradient(135deg, var(--color-basalt) 0%, var(--color-slate) 100%);
		border-left: 1px solid var(--color-stone);
	}

	.visual-content {
		display: flex;
		flex-direction: column;
		justify-content: center;
		gap: var(--space-8);
		padding: var(--space-12);
		height: 100%;
	}

	.feature-highlight {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		color: var(--color-amber);
	}

	.feature-highlight h3 {
		font-size: var(--text-h3);
		font-weight: 600;
		color: var(--color-chalk);
		margin: 0;
	}

	.feature-highlight p {
		font-size: var(--text-body);
		color: var(--color-silver);
		margin: 0;
	}

	@media (min-width: 1024px) {
		.signup-container {
			max-width: none;
			flex: 0 0 480px;
		}

		.signup-visual {
			display: block;
		}
	}
</style>
