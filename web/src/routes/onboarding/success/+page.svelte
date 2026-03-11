<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { Button } from '$lib/components';
	import { currentUser, isAuthenticated } from '$lib/stores';
	import type { ProjectData } from '$lib/types';

	let projectData = $state<ProjectData | null>(null);

	// Redirect unauthenticated users
	$effect(() => {
		if (!$isAuthenticated) {
			goto('/signup');
		}
	});

	onMount(() => {
		const stored = sessionStorage.getItem('onboarding_project');
		if (stored) {
			try {
				projectData = JSON.parse(stored);
			} catch {
				// Ignore parse errors
			}
		}
	});
</script>

<svelte:head>
	<title>You're All Set! - Yama</title>
</svelte:head>

<div class="success-page">
	<div class="success-content">
		<div class="success-icon">
			<svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
				<path d="M22 11.08V12a10 10 0 11-5.93-9.14" />
				<polyline points="22 4 12 14.01 9 11.01" />
			</svg>
		</div>

		<h1 class="success-title">You're all set!</h1>

		<p class="success-message">
			{#if projectData}
				Your project <strong>"{projectData.name}"</strong> has been created.
			{:else}
				Your project has been created.
			{/if}
		</p>

		<div class="email-card">
			<div class="email-icon">
				<svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z" />
					<polyline points="22,6 12,13 2,6" />
				</svg>
			</div>
			<div class="email-content">
				<h2>Check your email</h2>
				<p>
					We've sent download instructions to
					{#if $currentUser}
						<strong>{$currentUser.email}</strong>
					{:else}
						your email address
					{/if}.
				</p>
			</div>
		</div>

		<div class="next-steps">
			<h3>What's next?</h3>
			<ul class="steps-list">
				<li>
					<div class="step-number">1</div>
					<div class="step-content">
						<strong>Download the app</strong>
						<span>Open the email and click the download link</span>
					</div>
				</li>
				<li>
					<div class="step-number">2</div>
					<div class="step-content">
						<strong>Install & sign in</strong>
						<span>Run the installer and sign in with your account</span>
					</div>
				</li>
				<li>
					<div class="step-number">3</div>
					<div class="step-content">
						<strong>Start analyzing</strong>
						<span>Your project will be synced automatically</span>
					</div>
				</li>
			</ul>
		</div>

		<div class="actions">
			<Button variant="secondary" onclick={() => goto('/')}>
				Back to Home
			</Button>
		</div>

		<p class="help-text">
			Didn't receive the email?
			<button type="button" class="resend-link">Resend it</button>
			or check your spam folder.
		</p>
	</div>
</div>

<style>
	.success-page {
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: 100vh;
		padding: var(--space-8);
		background: var(--color-obsidian);
	}

	.success-content {
		max-width: 500px;
		text-align: center;
	}

	.success-icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 96px;
		height: 96px;
		background: rgba(16, 185, 129, 0.1);
		border: 2px solid var(--color-jade);
		border-radius: 50%;
		color: var(--color-jade);
		margin-bottom: var(--space-6);
	}

	.success-title {
		font-size: var(--text-display);
		font-weight: 700;
		color: var(--color-chalk);
		margin: 0 0 var(--space-2);
	}

	.success-message {
		font-size: var(--text-body);
		color: var(--color-silver);
		margin: 0 0 var(--space-8);
	}

	.success-message strong {
		color: var(--color-chalk);
	}

	.email-card {
		display: flex;
		align-items: center;
		gap: var(--space-4);
		padding: var(--space-6);
		background: var(--color-slate);
		border: 1px solid var(--color-stone);
		border-radius: var(--radius-xl);
		text-align: left;
		margin-bottom: var(--space-8);
	}

	.email-icon {
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 64px;
		height: 64px;
		background: rgba(245, 158, 11, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-amber);
	}

	.email-content h2 {
		font-size: var(--text-h3);
		font-weight: 600;
		color: var(--color-chalk);
		margin: 0 0 var(--space-1);
	}

	.email-content p {
		font-size: var(--text-body);
		color: var(--color-silver);
		margin: 0;
	}

	.email-content strong {
		color: var(--color-chalk);
	}

	.next-steps {
		text-align: left;
		margin-bottom: var(--space-8);
	}

	.next-steps h3 {
		font-size: var(--text-small);
		font-weight: 600;
		color: var(--color-ash);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		margin: 0 0 var(--space-4);
	}

	.steps-list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}

	.steps-list li {
		display: flex;
		align-items: flex-start;
		gap: var(--space-3);
	}

	.step-number {
		width: 28px;
		height: 28px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--color-slate);
		border: 1px solid var(--color-stone);
		border-radius: 50%;
		font-size: var(--text-small);
		font-weight: 600;
		color: var(--color-silver);
		flex-shrink: 0;
	}

	.step-content {
		display: flex;
		flex-direction: column;
		padding-top: 2px;
	}

	.step-content strong {
		font-size: var(--text-body);
		font-weight: 500;
		color: var(--color-chalk);
	}

	.step-content span {
		font-size: var(--text-small);
		color: var(--color-ash);
	}

	.actions {
		margin-bottom: var(--space-6);
	}

	.help-text {
		font-size: var(--text-small);
		color: var(--color-ash);
		margin: 0;
	}

	.resend-link {
		background: none;
		border: none;
		color: var(--color-amber);
		cursor: pointer;
		font-size: inherit;
		padding: 0;
	}

	.resend-link:hover {
		text-decoration: underline;
	}
</style>
