<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { Input, Button } from '$lib/components';
	import OAuthButtons from '$lib/components/OAuthButtons.svelte';
	import { auth, isAuthenticated } from '$lib/stores';
	import { isTauri } from '$lib/tauri/commands';
	import type { OAuthProvider } from '$lib/types';

	let email = $state('');
	let password = $state('');
	let isLoading = $state(false);
	let error = $state<string | null>(null);

	// Tauri app goes straight to dashboard - login is website-only
	onMount(() => {
		if (isTauri()) {
			goto('/dashboard');
		}
	});

	// Redirect authenticated users
	$effect(() => {
		if ($isAuthenticated) {
			goto('/dashboard');
		}
	});

	async function handleSubmit(e: Event) {
		e.preventDefault();
		if (!email || !password) {
			error = 'Please enter your email and password';
			return;
		}

		isLoading = true;
		error = null;

		try {
			await auth.login(email, password);
			// Auth store will trigger redirect via $effect
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to sign in. Please try again.';
		} finally {
			isLoading = false;
		}
	}

	function handleOAuth(provider: OAuthProvider) {
		error = `OAuth with ${provider} coming soon!`;
	}
</script>

<svelte:head>
	<title>Sign In - Yama</title>
</svelte:head>

<div class="login-page">
	<div class="login-container">
		<div class="login-header">
			<a href="/" class="logo-link" aria-label="Go to home page">
				<svg width="48" height="48" viewBox="0 0 64 64" fill="none">
					<path d="M32 8L8 52h48L32 8z" stroke="currentColor" stroke-width="2" fill="none" />
					<path d="M32 18L14 48h36L32 18z" fill="currentColor" opacity="0.2" />
				</svg>
			</a>
			<h1 class="login-title">Welcome back</h1>
			<p class="login-subtitle">Sign in to your Yama account</p>
		</div>

		<form class="login-form" onsubmit={handleSubmit}>
			{#if error}
				<div class="form-error">
					<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<circle cx="12" cy="12" r="10" />
						<line x1="12" y1="8" x2="12" y2="12" />
						<line x1="12" y1="16" x2="12.01" y2="16" />
					</svg>
					{error}
				</div>
			{/if}

			<OAuthButtons onAuth={handleOAuth} disabled={isLoading} />

			<div class="divider">
				<span>or</span>
			</div>

			<div class="form-fields">
				<Input
					type="email"
					placeholder="Email address"
					bind:value={email}
					disabled={isLoading}
				/>
				<Input
					type="password"
					placeholder="Password"
					bind:value={password}
					disabled={isLoading}
				/>
			</div>

			<Button
				type="submit"
				variant="primary"
				size="lg"
				fullWidth
				disabled={isLoading}
				loading={isLoading}
			>
				{isLoading ? 'Signing in...' : 'Sign In'}
			</Button>

			<div class="form-links">
				<a href="/forgot-password">Forgot password?</a>
			</div>
		</form>

		<p class="signup-link">
			Don't have an account? <a href="/signup">Sign up</a>
		</p>
	</div>
</div>

<style>
	.login-page {
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: 100vh;
		padding: var(--space-8);
		background: var(--color-obsidian);
	}

	.login-container {
		width: 100%;
		max-width: 400px;
	}

	.login-header {
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

	.login-title {
		font-size: var(--text-h1);
		font-weight: 600;
		color: var(--color-chalk);
		margin: 0 0 var(--space-2);
	}

	.login-subtitle {
		font-size: var(--text-body);
		color: var(--color-silver);
		margin: 0;
	}

	.login-form {
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
	}

	.form-error {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-3) var(--space-4);
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid var(--color-ember);
		border-radius: var(--radius-md);
		color: var(--color-ember);
		font-size: var(--text-small);
	}

	.divider {
		display: flex;
		align-items: center;
		gap: var(--space-4);
	}

	.divider::before,
	.divider::after {
		content: '';
		flex: 1;
		height: 1px;
		background: var(--color-stone);
	}

	.divider span {
		font-size: var(--text-small);
		color: var(--color-ash);
	}

	.form-fields {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}

	.form-links {
		text-align: center;
	}

	.form-links a {
		font-size: var(--text-small);
		color: var(--color-amber);
		text-decoration: none;
	}

	.form-links a:hover {
		text-decoration: underline;
	}

	.signup-link {
		font-size: var(--text-small);
		color: var(--color-ash);
		text-align: center;
		margin-top: var(--space-6);
	}

	.signup-link a {
		color: var(--color-amber);
		text-decoration: none;
	}

	.signup-link a:hover {
		text-decoration: underline;
	}
</style>
