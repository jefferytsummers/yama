<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { Button, Card, Badge, SettingsPanel } from '$lib/components';
	import { projectsApi, type Project, type ProjectConfig } from '$lib/api/projects';

	let project = $state<Project | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let settingsOpen = $state(false);
	let saving = $state(false);

	// Default config if none exists
	const defaultConfig: ProjectConfig = {
		enabled_tools: ['search_videos', 'analyze_frame', 'summarize_video'],
		default_model: 'qwen2.5-vl-3b'
	};

	onMount(async () => {
		const projectId = $page.params.id;
		if (!projectId) {
			error = 'No project ID provided';
			loading = false;
			return;
		}
		try {
			project = await projectsApi.get(projectId);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load project';
		} finally {
			loading = false;
		}
	});

	async function handleSaveConfig(config: ProjectConfig) {
		if (!project) return;

		saving = true;
		try {
			project = await projectsApi.updateConfig(project.id, config);
			settingsOpen = false;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to save config';
		} finally {
			saving = false;
		}
	}

	async function handleDelete() {
		if (!project) return;
		if (!confirm(`Delete "${project.name}"? This cannot be undone.`)) return;

		try {
			await projectsApi.delete(project.id);
			goto('/dashboard');
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to delete project';
		}
	}

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleDateString('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric'
		});
	}
</script>

<svelte:head>
	<title>{project?.name ?? 'Project'} - Yama</title>
</svelte:head>

<div class="project-page">
	<header class="page-header">
		<div class="header-left">
			<button class="back-btn" onclick={() => goto('/dashboard')}>
				<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<path d="M19 12H5M12 19l-7-7 7-7" />
				</svg>
			</button>
			<a href="/dashboard" class="logo">
				<svg width="28" height="28" viewBox="0 0 64 64" fill="none">
					<path d="M32 8L8 52h48L32 8z" stroke="currentColor" stroke-width="2" fill="none" />
					<path d="M32 18L14 48h36L32 18z" fill="currentColor" opacity="0.2" />
				</svg>
				<span>YAMA</span>
			</a>
		</div>
		<div class="header-right">
			{#if project}
				<Button variant="ghost" size="sm" onclick={() => settingsOpen = true}>
					<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<circle cx="12" cy="12" r="3" />
						<path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
					</svg>
					Settings
				</Button>
			{/if}
		</div>
	</header>

	<main class="page-content">
		{#if loading}
			<div class="loading">
				<div class="spinner"></div>
				<p>Loading project...</p>
			</div>
		{:else if error}
			<Card>
				<div class="error-state">
					<svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
						<circle cx="12" cy="12" r="10" />
						<path d="M12 8v4M12 16h.01" />
					</svg>
					<h2>Error</h2>
					<p>{error}</p>
					<Button variant="primary" onclick={() => goto('/dashboard')}>
						Back to Dashboard
					</Button>
				</div>
			</Card>
		{:else if project}
			<div class="project-header">
				<div class="project-title">
					<h1>{project.name}</h1>
					{#if project.description}
						<p class="description">{project.description}</p>
					{/if}
				</div>
				<div class="project-meta">
					<span class="meta-item">
						<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
							<rect x="3" y="4" width="18" height="18" rx="2" ry="2" />
							<line x1="16" y1="2" x2="16" y2="6" />
							<line x1="8" y1="2" x2="8" y2="6" />
							<line x1="3" y1="10" x2="21" y2="10" />
						</svg>
						Created {formatDate(project.created_at)}
					</span>
					<Badge variant="neutral">{project.library_count} libraries</Badge>
				</div>
			</div>

			<div class="project-sections">
				<!-- Config Summary -->
				<Card>
					<div class="section">
						<div class="section-header">
							<h2>Configuration</h2>
							<Button variant="ghost" size="sm" onclick={() => settingsOpen = true}>
								Edit
							</Button>
						</div>
						<div class="config-grid">
							<div class="config-item">
								<span class="config-label">Enabled Tools</span>
								<div class="config-badges">
									{#each (project.config?.enabled_tools ?? defaultConfig.enabled_tools) as tool}
										<Badge variant="neutral">{tool}</Badge>
									{/each}
								</div>
							</div>
							<div class="config-item">
								<span class="config-label">Default Model</span>
								<Badge variant="info">{project.config?.default_model ?? defaultConfig.default_model}</Badge>
							</div>
						</div>
					</div>
				</Card>

				<!-- Quick Actions -->
				<Card>
					<div class="section">
						<h2>Quick Actions</h2>
						<div class="actions-grid">
							<Button variant="primary" onclick={() => goto(`/chat?project=${project?.id}`)}>
								<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
									<path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
								</svg>
								Start Chat
							</Button>
							<Button variant="secondary">
								<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
									<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
									<polyline points="17 8 12 3 7 8" />
									<line x1="12" y1="3" x2="12" y2="15" />
								</svg>
								Add Videos
							</Button>
							<Button variant="ghost" onclick={handleDelete}>
								<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
									<polyline points="3 6 5 6 21 6" />
									<path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
								</svg>
								Delete Project
							</Button>
						</div>
					</div>
				</Card>

				<!-- Libraries (Empty State) -->
				<Card>
					<div class="section">
						<h2>Video Libraries</h2>
						<div class="empty-libraries">
							<svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
								<path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
							</svg>
							<p>No video libraries yet. Add a folder to start indexing.</p>
							<Button variant="secondary">
								Add Library
							</Button>
						</div>
					</div>
				</Card>
			</div>
		{/if}
	</main>

	<!-- Settings Panel -->
	{#if project}
		<SettingsPanel
			isOpen={settingsOpen}
			config={project.config ?? defaultConfig}
			onClose={() => settingsOpen = false}
			onSave={handleSaveConfig}
		/>
	{/if}
</div>

<style>
	.project-page {
		display: flex;
		flex-direction: column;
		min-height: 100vh;
		background: var(--color-obsidian);
	}

	.page-header {
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
		gap: var(--space-4);
	}

	.back-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 36px;
		height: 36px;
		border: none;
		background: transparent;
		color: var(--color-silver);
		border-radius: var(--radius-md);
		cursor: pointer;
		transition: background var(--duration-fast), color var(--duration-fast);
	}

	.back-btn:hover {
		background: var(--color-graphite);
		color: var(--color-chalk);
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
		gap: var(--space-3);
	}

	.page-content {
		flex: 1;
		padding: var(--space-8);
		max-width: 900px;
		margin: 0 auto;
		width: 100%;
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
		text-align: center;
		gap: var(--space-4);
		padding: var(--space-8);
		color: var(--color-ember);
	}

	.error-state h2 {
		color: var(--color-chalk);
		margin: 0;
	}

	.error-state p {
		color: var(--color-silver);
		margin: 0;
	}

	.project-header {
		margin-bottom: var(--space-6);
	}

	.project-title h1 {
		font-size: var(--text-h1);
		font-weight: 600;
		color: var(--color-chalk);
		margin: 0 0 var(--space-2);
	}

	.description {
		font-size: var(--text-body);
		color: var(--color-silver);
		margin: 0;
	}

	.project-meta {
		display: flex;
		align-items: center;
		gap: var(--space-4);
		margin-top: var(--space-3);
	}

	.meta-item {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--text-small);
		color: var(--color-silver);
	}

	.project-sections {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}

	.section {
		padding: var(--space-5);
	}

	.section h2 {
		font-size: var(--text-h3);
		font-weight: 600;
		color: var(--color-chalk);
		margin: 0 0 var(--space-4);
	}

	.section-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: var(--space-4);
	}

	.section-header h2 {
		margin: 0;
	}

	.config-grid {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}

	.config-item {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.config-label {
		font-size: var(--text-small);
		font-weight: 500;
		color: var(--color-silver);
	}

	.config-badges {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
	}

	.actions-grid {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-3);
	}

	.empty-libraries {
		display: flex;
		flex-direction: column;
		align-items: center;
		text-align: center;
		gap: var(--space-4);
		padding: var(--space-6);
		color: var(--color-stone);
	}

	.empty-libraries p {
		color: var(--color-silver);
		margin: 0;
	}
</style>
