<script lang="ts">
	import { Button, Badge, Card } from '$lib/components';
	import type { ProjectConfig } from '$lib/api/projects';

	interface Tool {
		id: string;
		name: string;
		description: string;
		category: 'analysis' | 'search' | 'extraction' | 'utility';
	}

	interface Model {
		id: string;
		name: string;
		type: string;
		size: string;
	}

	interface Workflow {
		id: string;
		name: string;
		description: string;
	}

	interface Props {
		isOpen: boolean;
		config: ProjectConfig;
		onClose: () => void;
		onSave: (config: ProjectConfig) => void;
	}

	let { isOpen, config, onClose, onSave }: Props = $props();

	// Available tools
	const tools: Tool[] = [
		{ id: 'search_videos', name: 'Search Videos', description: 'Semantic search across video library', category: 'search' },
		{ id: 'analyze_frame', name: 'Analyze Frame', description: 'Detailed analysis of a single frame', category: 'analysis' },
		{ id: 'extract_clip', name: 'Extract Clip', description: 'Extract video segments by timestamp', category: 'extraction' },
		{ id: 'summarize_video', name: 'Summarize Video', description: 'Generate video summary with key moments', category: 'analysis' },
		{ id: 'detect_objects', name: 'Detect Objects', description: 'Object detection and tracking', category: 'analysis' },
		{ id: 'transcribe_audio', name: 'Transcribe Audio', description: 'Speech-to-text transcription', category: 'extraction' },
		{ id: 'generate_thumbnail', name: 'Generate Thumbnail', description: 'Create representative thumbnails', category: 'utility' },
		{ id: 'compare_frames', name: 'Compare Frames', description: 'Visual comparison between frames', category: 'analysis' }
	];

	// Available models
	const models: Model[] = [
		{ id: 'qwen2.5-vl-3b', name: 'Qwen2.5-VL-3B', type: 'VLM', size: '3.2GB' },
		{ id: 'llava-1.6-7b', name: 'LLaVA 1.6 7B', type: 'VLM', size: '6.8GB' },
		{ id: 'whisper-large-v3', name: 'Whisper Large v3', type: 'Transcription', size: '1.5GB' },
		{ id: 'clip-vit-l14', name: 'CLIP ViT-L/14', type: 'Embedding', size: '500MB' }
	];

	// Available workflows
	const workflows: Workflow[] = [
		{ id: 'full-indexing', name: 'Full Indexing', description: 'Complete video analysis with all features' },
		{ id: 'quick-scan', name: 'Quick Scan', description: 'Fast analysis with key frames only' },
		{ id: 'transcript-only', name: 'Transcript Only', description: 'Audio transcription without visual analysis' }
	];

	// Local state for editing - reset when config changes
	let enabledTools = $state<string[]>([]);
	let selectedModel = $state('qwen2.5-vl-3b');
	let selectedWorkflow = $state('full-indexing');

	// Sync state when config changes
	$effect(() => {
		enabledTools = [...config.enabled_tools];
		selectedModel = config.default_model || 'qwen2.5-vl-3b';
	});

	// Expanded sections
	let expandedSections = $state<Record<string, boolean>>({
		tools: true,
		models: false,
		workflows: false
	});

	// Dirty state - compare current state with original config
	let isDirty = $derived.by(() => {
		const originalTools = [...config.enabled_tools].sort();
		const currentTools = [...enabledTools].sort();
		const toolsChanged = JSON.stringify(currentTools) !== JSON.stringify(originalTools);
		const modelChanged = selectedModel !== (config.default_model || 'qwen2.5-vl-3b');
		return toolsChanged || modelChanged;
	});

	function toggleTool(toolId: string) {
		if (enabledTools.includes(toolId)) {
			enabledTools = enabledTools.filter(t => t !== toolId);
		} else {
			enabledTools = [...enabledTools, toolId];
		}
	}

	function toggleSection(section: string) {
		expandedSections = { ...expandedSections, [section]: !expandedSections[section] };
	}

	function handleSave() {
		onSave({
			enabled_tools: enabledTools,
			default_model: selectedModel
		});
	}

	function handleClose() {
		// The $effect will reset to original values when panel reopens
		onClose();
	}

	// Group tools by category
	const toolsByCategory = $derived(
		tools.reduce((acc, tool) => {
			if (!acc[tool.category]) acc[tool.category] = [];
			acc[tool.category].push(tool);
			return acc;
		}, {} as Record<string, Tool[]>)
	);

	const categoryLabels: Record<string, string> = {
		analysis: 'Analysis',
		search: 'Search',
		extraction: 'Extraction',
		utility: 'Utility'
	};
</script>

{#if isOpen}
	<!-- Backdrop -->
	<button class="backdrop" onclick={handleClose} aria-label="Close settings"></button>

	<!-- Panel -->
	<aside class="panel" class:open={isOpen}>
		<header class="panel-header">
			<h2>Project Settings</h2>
			<button class="close-btn" onclick={handleClose} aria-label="Close">
				<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<path d="M18 6L6 18M6 6l12 12" />
				</svg>
			</button>
		</header>

		<div class="panel-content">
			<!-- Tools Section -->
			<section class="section">
				<button class="section-header" onclick={() => toggleSection('tools')}>
					<div class="section-title">
						<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
							<path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z" />
						</svg>
						<span>Tools</span>
						<Badge variant="neutral">{enabledTools.length} enabled</Badge>
					</div>
					<svg class="chevron" class:expanded={expandedSections.tools} width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<path d="M6 9l6 6 6-6" />
					</svg>
				</button>

				{#if expandedSections.tools}
					<div class="section-content">
						{#each Object.entries(toolsByCategory) as [category, categoryTools]}
							<div class="category">
								<span class="category-label">{categoryLabels[category]}</span>
								{#each categoryTools as tool}
									<label class="tool-item">
										<div class="tool-info">
											<span class="tool-name">{tool.name}</span>
											<span class="tool-desc">{tool.description}</span>
										</div>
										<button
											class="toggle"
											class:active={enabledTools.includes(tool.id)}
											onclick={() => toggleTool(tool.id)}
											aria-pressed={enabledTools.includes(tool.id)}
										>
											<span class="toggle-knob"></span>
										</button>
									</label>
								{/each}
							</div>
						{/each}
					</div>
				{/if}
			</section>

			<!-- Models Section -->
			<section class="section">
				<button class="section-header" onclick={() => toggleSection('models')}>
					<div class="section-title">
						<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
							<path d="M12 2L2 7l10 5 10-5-10-5z" />
							<path d="M2 17l10 5 10-5" />
							<path d="M2 12l10 5 10-5" />
						</svg>
						<span>Models</span>
					</div>
					<svg class="chevron" class:expanded={expandedSections.models} width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<path d="M6 9l6 6 6-6" />
					</svg>
				</button>

				{#if expandedSections.models}
					<div class="section-content">
						{#each models as model}
							<label class="model-item" class:selected={selectedModel === model.id}>
								<input
									type="radio"
									name="model"
									value={model.id}
									bind:group={selectedModel}
								/>
								<div class="model-info">
									<div class="model-header">
										<span class="model-name">{model.name}</span>
										<Badge variant="neutral">{model.type}</Badge>
									</div>
									<span class="model-size">{model.size}</span>
								</div>
							</label>
						{/each}
					</div>
				{/if}
			</section>

			<!-- Workflows Section -->
			<section class="section">
				<button class="section-header" onclick={() => toggleSection('workflows')}>
					<div class="section-title">
						<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
							<path d="M22 12h-4l-3 9L9 3l-3 9H2" />
						</svg>
						<span>Workflows</span>
					</div>
					<svg class="chevron" class:expanded={expandedSections.workflows} width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<path d="M6 9l6 6 6-6" />
					</svg>
				</button>

				{#if expandedSections.workflows}
					<div class="section-content">
						{#each workflows as workflow}
							<label class="workflow-item" class:selected={selectedWorkflow === workflow.id}>
								<input
									type="radio"
									name="workflow"
									value={workflow.id}
									bind:group={selectedWorkflow}
								/>
								<div class="workflow-info">
									<span class="workflow-name">{workflow.name}</span>
									<span class="workflow-desc">{workflow.description}</span>
								</div>
							</label>
						{/each}
					</div>
				{/if}
			</section>
		</div>

		<footer class="panel-footer">
			<Button variant="ghost" onclick={handleClose}>Cancel</Button>
			<Button variant="primary" onclick={handleSave} disabled={!isDirty}>
				Save Changes
			</Button>
		</footer>
	</aside>
{/if}

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.5);
		z-index: 100;
		border: none;
		cursor: pointer;
	}

	.panel {
		position: fixed;
		top: 0;
		right: 0;
		bottom: 0;
		width: 400px;
		max-width: 100vw;
		background: var(--color-basalt);
		border-left: 1px solid var(--color-stone);
		z-index: 101;
		display: flex;
		flex-direction: column;
		transform: translateX(100%);
		transition: transform var(--duration-normal) var(--ease-out);
	}

	.panel.open {
		transform: translateX(0);
	}

	.panel-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: var(--space-4) var(--space-5);
		border-bottom: 1px solid var(--color-stone);
	}

	.panel-header h2 {
		font-size: var(--text-h3);
		font-weight: 600;
		color: var(--color-chalk);
		margin: 0;
	}

	.close-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 32px;
		border: none;
		background: transparent;
		color: var(--color-silver);
		border-radius: var(--radius-sm);
		cursor: pointer;
		transition: background var(--duration-fast), color var(--duration-fast);
	}

	.close-btn:hover {
		background: var(--color-graphite);
		color: var(--color-chalk);
	}

	.panel-content {
		flex: 1;
		overflow-y: auto;
		padding: var(--space-4);
	}

	.section {
		margin-bottom: var(--space-3);
		background: var(--color-slate);
		border-radius: var(--radius-lg);
		overflow: hidden;
	}

	.section-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		width: 100%;
		padding: var(--space-4);
		border: none;
		background: transparent;
		cursor: pointer;
		text-align: left;
	}

	.section-header:hover {
		background: var(--color-graphite);
	}

	.section-title {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		color: var(--color-chalk);
		font-weight: 500;
	}

	.chevron {
		color: var(--color-silver);
		transition: transform var(--duration-fast);
	}

	.chevron.expanded {
		transform: rotate(180deg);
	}

	.section-content {
		padding: 0 var(--space-4) var(--space-4);
	}

	.category {
		margin-bottom: var(--space-3);
	}

	.category:last-child {
		margin-bottom: 0;
	}

	.category-label {
		display: block;
		font-size: var(--text-tiny);
		font-weight: 600;
		color: var(--color-ash);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		margin-bottom: var(--space-2);
	}

	.tool-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: var(--space-3);
		background: var(--color-graphite);
		border-radius: var(--radius-md);
		margin-bottom: var(--space-2);
		cursor: pointer;
	}

	.tool-item:hover {
		background: var(--color-stone);
	}

	.tool-info {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}

	.tool-name {
		font-size: var(--text-body);
		font-weight: 500;
		color: var(--color-chalk);
	}

	.tool-desc {
		font-size: var(--text-small);
		color: var(--color-silver);
	}

	.toggle {
		position: relative;
		width: 44px;
		height: 24px;
		background: var(--color-stone);
		border: none;
		border-radius: 12px;
		cursor: pointer;
		transition: background var(--duration-fast);
	}

	.toggle.active {
		background: var(--color-jade);
	}

	.toggle-knob {
		position: absolute;
		top: 2px;
		left: 2px;
		width: 20px;
		height: 20px;
		background: var(--color-chalk);
		border-radius: 50%;
		transition: transform var(--duration-fast);
	}

	.toggle.active .toggle-knob {
		transform: translateX(20px);
	}

	.model-item,
	.workflow-item {
		display: flex;
		align-items: flex-start;
		gap: var(--space-3);
		padding: var(--space-3);
		background: var(--color-graphite);
		border-radius: var(--radius-md);
		margin-bottom: var(--space-2);
		cursor: pointer;
		border: 2px solid transparent;
		transition: border-color var(--duration-fast), background var(--duration-fast);
	}

	.model-item:hover,
	.workflow-item:hover {
		background: var(--color-stone);
	}

	.model-item.selected,
	.workflow-item.selected {
		border-color: var(--color-amber);
	}

	.model-item input,
	.workflow-item input {
		display: none;
	}

	.model-info,
	.workflow-info {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}

	.model-header {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.model-name,
	.workflow-name {
		font-size: var(--text-body);
		font-weight: 500;
		color: var(--color-chalk);
	}

	.model-size,
	.workflow-desc {
		font-size: var(--text-small);
		color: var(--color-silver);
	}

	.panel-footer {
		display: flex;
		justify-content: flex-end;
		gap: var(--space-3);
		padding: var(--space-4) var(--space-5);
		border-top: 1px solid var(--color-stone);
		background: var(--color-basalt);
	}
</style>
