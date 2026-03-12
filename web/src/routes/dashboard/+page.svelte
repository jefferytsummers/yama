<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { Button, Card, Badge, Modal, ProjectWizard } from '$lib/components';
	import { auth, isAuthenticated, currentUser } from '$lib/stores';
	import { projectsApi, type Project } from '$lib/api/projects';
	import type { ProjectData } from '$lib/types';

	// State
	let projects = $state<Project[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let showWizard = $state(false);

	// Redirect unauthenticated users
	$effect(() => {
		if (!$isAuthenticated) {
			goto('/');
		}
	});

	onMount(async () => {
		await loadProjects();
	});

	async function loadProjects() {
		loading = true;
		error = null;
		try {
			projects = await projectsApi.list();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load projects';
		} finally {
			loading = false;
		}
	}

	function handleLogout() {
		auth.logout();
		goto('/');
	}

	function handleNewProject() {
		showWizard = true;
	}

	async function handleWizardComplete(data: ProjectData) {
		try {
			const project = await projectsApi.create({
				name: data.name,
				description: data.description || undefined,
				tags: data.tags
			});
			showWizard = false;
			goto(`/dashboard/projects/${project.id}`);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to create project';
		}
	}

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric'
		});
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
			<div class="welcome-text">
				<h1>Welcome back{$currentUser?.name ? `, ${$currentUser.name}` : ''}!</h1>
				<p>Manage your video analysis projects.</p>
			</div>
			<Button variant="primary" onclick={handleNewProject}>
				<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<path d="M12 5v14M5 12h14" />
				</svg>
				New Project
			</Button>
		</div>

		{#if loading}
			<div class="loading">
				<div class="spinner"></div>
				<p>Loading projects...</p>
			</div>
		{:else if error}
			<Card>
				<div class="error-state">
					<p>{error}</p>
					<Button variant="secondary" onclick={loadProjects}>
						Try Again
					</Button>
				</div>
			</Card>
		{:else if projects.length === 0}
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
		{:else}
			<div class="projects-grid">
				{#each projects as project}
					<a href="/dashboard/projects/{project.id}" class="project-card">
						<Card interactive>
							<div class="project-content">
								<div class="project-header">
									<h3>{project.name}</h3>
									<Badge variant="neutral">{project.library_count} libraries</Badge>
								</div>
								{#if project.description}
									<p class="project-description">{project.description}</p>
								{/if}
								<div class="project-footer">
									<span class="project-date">
										<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
											<rect x="3" y="4" width="18" height="18" rx="2" ry="2" />
											<line x1="16" y1="2" x2="16" y2="6" />
											<line x1="8" y1="2" x2="8" y2="6" />
											<line x1="3" y1="10" x2="21" y2="10" />
										</svg>
										{formatDate(project.created_at)}
									</span>
									<span class="view-link">
										View
										<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
											<path d="M5 12h14M12 5l7 7-7 7" />
										</svg>
									</span>
								</div>
							</div>
						</Card>
					</a>
				{/each}
			</div>
		{/if}

		<div class="status-section">
			<Card padding="sm">
				<div class="status-row">
					<Badge variant="success" glow pulse>VLM Ready</Badge>
					<Badge variant="neutral">{projects.length} Projects</Badge>
				</div>
			</Card>
		</div>
	</main>
</div>

<!-- Project Wizard Modal -->
<Modal bind:open={showWizard} onclose={() => showWizard = false} title="Create New Project" width="800px">
	<ProjectWizard
		context="app"
		onComplete={handleWizardComplete}
		onCancel={() => showWizard = false}
	/>
</Modal>

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
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: var(--space-8);
	}

	.welcome-text h1 {
		font-size: var(--text-h1);
		font-weight: 600;
		color: var(--color-chalk);
		margin: 0 0 var(--space-2);
	}

	.welcome-text p {
		font-size: var(--text-body);
		color: var(--color-silver);
		margin: 0;
	}

	.loading {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: var(--space-4);
		padding: var(--space-12);
		color: var(--color-silver);
	}

	.spinner {
		width: 32px;
		height: 32px;
		border: 3px solid var(--color-stone);
		border-top-color: var(--color-amber);
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
	}

	@keyframes spin {
		to { transform: rotate(360deg); }
	}

	.error-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-4);
		padding: var(--space-6);
		text-align: center;
	}

	.error-state p {
		color: var(--color-ember);
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

	.projects-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
		gap: var(--space-4);
		margin-bottom: var(--space-8);
	}

	.project-card {
		text-decoration: none;
		color: inherit;
		display: block;
	}

	.project-content {
		padding: var(--space-4);
	}

	.project-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: var(--space-2);
	}

	.project-header h3 {
		font-size: var(--text-h3);
		font-weight: 600;
		color: var(--color-chalk);
		margin: 0;
	}

	.project-description {
		font-size: var(--text-small);
		color: var(--color-silver);
		margin: 0 0 var(--space-4);
		display: -webkit-box;
		-webkit-line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}

	.project-footer {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.project-date {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--text-small);
		color: var(--color-ash);
	}

	.view-link {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		font-size: var(--text-small);
		color: var(--color-amber);
		opacity: 0;
		transition: opacity var(--duration-fast);
	}

	.project-card:hover .view-link {
		opacity: 1;
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

		.welcome-section {
			flex-direction: column;
			align-items: flex-start;
			gap: var(--space-4);
		}

		.user-email {
			display: none;
		}
	}
</style>
