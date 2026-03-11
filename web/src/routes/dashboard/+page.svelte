<script lang="ts">
	import { goto } from '$app/navigation';
	import { Button, Card, Badge } from '$lib/components';
	import { auth, isAuthenticated, currentUser } from '$lib/stores';

	// Redirect unauthenticated users
	$effect(() => {
		if (!$isAuthenticated) {
			goto('/');
		}
	});

	function handleLogout() {
		auth.logout();
		goto('/');
	}

	function handleNewProject() {
		// In the desktop app context, this would open the project wizard inline
		// For now, redirect to onboarding (which uses the same wizard)
		goto('/onboarding');
	}
</script>

<svelte:head>
	<title>Dashboard - Yama</title>
</svelte:head>

<div class="dashboard">
	<header class="dashboard-header">
		<div class="header-left">
			<a href="/dashboard" class="logo">
				<svg width="32" height="32" viewBox="0 0 64 64" fill="none">
					<path d="M32 8L8 52h48L32 8z" stroke="currentColor" stroke-width="2" fill="none" />
					<path d="M32 18L14 48h36L32 18z" fill="currentColor" opacity="0.2" />
				</svg>
				<span>YAMA</span>
			</a>
		</div>
		<div class="header-right">
			{#if $currentUser}
				<span class="user-email">{$currentUser.email}</span>
			{/if}
			<Button variant="ghost" size="sm" onclick={handleLogout}>
				Sign Out
			</Button>
		</div>
	</header>

	<main class="dashboard-content">
		<div class="welcome-section">
			<h1>Welcome back{$currentUser?.name ? `, ${$currentUser.name}` : ''}!</h1>
			<p>This is a preview of your Yama dashboard. Download the desktop app to start analyzing videos.</p>
		</div>

		<div class="empty-state">
			<Card>
				<div class="empty-content">
					<div class="empty-icon">
						<svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
							<rect x="3" y="3" width="18" height="18" rx="2" />
							<path d="M12 8v8M8 12h8" />
						</svg>
					</div>
					<h2>No projects yet</h2>
					<p>Create your first project to start analyzing videos with AI.</p>
					<Button variant="primary" onclick={handleNewProject}>
						<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
							<path d="M12 5v14M5 12h14" />
						</svg>
						New Project
					</Button>
				</div>
			</Card>
		</div>

		<div class="status-section">
			<Card padding="sm">
				<div class="status-row">
					<Badge variant="success" glow pulse>VLM Ready</Badge>
					<Badge variant="neutral">0 Projects</Badge>
					<Badge variant="neutral">0 Videos</Badge>
				</div>
			</Card>
		</div>
	</main>
</div>

<style>
	.dashboard {
		display: flex;
		flex-direction: column;
		min-height: 100vh;
		background: var(--color-obsidian);
	}

	.dashboard-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: var(--space-4) var(--space-6);
		border-bottom: 1px solid var(--color-stone);
		background: var(--color-basalt);
	}

	.header-left {
		display: flex;
		align-items: center;
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

	.header-right {
		display: flex;
		align-items: center;
		gap: var(--space-4);
	}

	.user-email {
		font-size: var(--text-small);
		color: var(--color-silver);
	}

	.dashboard-content {
		flex: 1;
		padding: var(--space-8);
		max-width: 1200px;
		margin: 0 auto;
		width: 100%;
	}

	.welcome-section {
		margin-bottom: var(--space-8);
	}

	.welcome-section h1 {
		font-size: var(--text-h1);
		font-weight: 600;
		color: var(--color-chalk);
		margin: 0 0 var(--space-2);
	}

	.welcome-section p {
		font-size: var(--text-body);
		color: var(--color-silver);
		margin: 0;
	}

	.empty-state {
		margin-bottom: var(--space-8);
	}

	.empty-content {
		display: flex;
		flex-direction: column;
		align-items: center;
		text-align: center;
		padding: var(--space-8);
		gap: var(--space-4);
	}

	.empty-icon {
		color: var(--color-stone);
	}

	.empty-content h2 {
		font-size: var(--text-h2);
		font-weight: 600;
		color: var(--color-chalk);
		margin: 0;
	}

	.empty-content p {
		font-size: var(--text-body);
		color: var(--color-silver);
		margin: 0;
		max-width: 400px;
	}

	.status-section {
		position: fixed;
		bottom: var(--space-6);
		left: 50%;
		transform: translateX(-50%);
	}

	.status-row {
		display: flex;
		align-items: center;
		gap: var(--space-4);
	}

	@media (max-width: 640px) {
		.dashboard-content {
			padding: var(--space-4);
		}

		.user-email {
			display: none;
		}
	}
</style>
