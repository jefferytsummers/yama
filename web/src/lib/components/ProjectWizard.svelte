<script lang="ts">
	import {
		Button,
		Input,
		TextArea,
		TagInput,
		DropZone,
		StepIndicator,
		PresetCard,
		ToolToggle,
		ModelChip,
		Badge
	} from '$lib/components';
	import {
		type ProjectData,
		type WizardContext,
		type ToolId,
		type PresetId,
		defaultProjectData,
		wizardSteps,
		agentPresets,
		availableTools
	} from '$lib/types';

	interface Props {
		context: WizardContext;
		onComplete: (data: ProjectData) => void;
		onCancel?: () => void;
	}

	let { context, onComplete, onCancel }: Props = $props();

	let currentStep = $state(0);
	let projectData = $state<ProjectData>({ ...defaultProjectData });
	let isSubmitting = $state(false);

	let canProceed = $derived.by(() => {
		switch (currentStep) {
			case 0: // Details
				return projectData.name.trim().length > 0;
			case 1: // Videos
				return true; // Videos are optional
			case 2: // Models
				return projectData.selectedPreset !== null;
			case 3: // Tools
				return projectData.enabledTools.length > 0;
			case 4: // Review
				return true;
			default:
				return false;
		}
	});

	let selectedPreset = $derived(
		agentPresets.find((p) => p.id === projectData.selectedPreset)
	);

	function handleNext() {
		if (currentStep < wizardSteps.length - 1) {
			currentStep++;
		} else {
			handleSubmit();
		}
	}

	function handleBack() {
		if (currentStep > 0) {
			currentStep--;
		}
	}

	async function handleSubmit() {
		isSubmitting = true;
		// Simulate API call
		await new Promise((resolve) => setTimeout(resolve, 1500));
		onComplete(projectData);
		isSubmitting = false;
	}

	function handleFileUpload(fileList: FileList) {
		const files = Array.from(fileList);
		const newFiles = files.map((file) => ({
			id: crypto.randomUUID(),
			name: file.name,
			size: file.size,
			progress: 100 // Simulated instant upload
		}));
		projectData.files = [...projectData.files, ...newFiles];
	}

	function removeFile(id: string) {
		projectData.files = projectData.files.filter((f) => f.id !== id);
	}

	function toggleTool(toolId: ToolId) {
		if (projectData.enabledTools.includes(toolId)) {
			projectData.enabledTools = projectData.enabledTools.filter((t) => t !== toolId);
		} else {
			projectData.enabledTools = [...projectData.enabledTools, toolId];
		}
	}

	function selectPreset(presetId: PresetId) {
		projectData.selectedPreset = presetId;
	}

	function togglePresetExpand(presetId: PresetId) {
		if (projectData.expandedPreset === presetId) {
			projectData.expandedPreset = null;
		} else {
			projectData.expandedPreset = presetId;
		}
	}

	function formatFileSize(bytes: number): string {
		if (bytes < 1024) return bytes + ' B';
		if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
		if (bytes < 1024 * 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
		return (bytes / (1024 * 1024 * 1024)).toFixed(1) + ' GB';
	}
</script>

<div class="wizard">
	<div class="wizard-header">
		<StepIndicator steps={wizardSteps} {currentStep} />
	</div>

	<div class="wizard-content">
		{#if currentStep === 0}
			<!-- Step 1: Project Details -->
			<div class="step-content">
				<h2 class="step-title">Project Details</h2>
				<p class="step-description">Give your project a name and optional description.</p>

				<div class="form-group">
					<Input
						label="Project Name"
						placeholder="My Video Project"
						bind:value={projectData.name}
					/>
				</div>

				<div class="form-group">
					<TextArea
						label="Description (optional)"
						placeholder="What is this project about?"
						bind:value={projectData.description}
						rows={3}
						maxLength={500}
						showCount
					/>
				</div>

				<div class="form-group">
					<TagInput
						label="Tags (optional)"
						bind:tags={projectData.tags}
						placeholder="Add tags..."
						maxTags={5}
					/>
				</div>
			</div>

		{:else if currentStep === 1}
			<!-- Step 2: Import Videos -->
			<div class="step-content">
				<h2 class="step-title">Import Videos</h2>
				<p class="step-description">
					Add videos to your project. You can also add more later.
				</p>

				<DropZone
					accept="video/*"
					multiple
					ondrop={handleFileUpload}
				/>

				{#if projectData.files.length > 0}
					<div class="file-list">
						<h3 class="file-list-title">Uploaded Files ({projectData.files.length})</h3>
						{#each projectData.files as file}
							<div class="file-item">
								<div class="file-info">
									<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
										<polygon points="23 7 16 12 23 17 23 7" />
										<rect x="1" y="5" width="15" height="14" rx="2" />
									</svg>
									<span class="file-name">{file.name}</span>
									<span class="file-size">{formatFileSize(file.size)}</span>
								</div>
								<button
									type="button"
									class="file-remove"
									onclick={() => removeFile(file.id)}
									aria-label="Remove file"
								>
									<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
										<line x1="18" y1="6" x2="6" y2="18" />
										<line x1="6" y1="6" x2="18" y2="18" />
									</svg>
								</button>
							</div>
						{/each}
					</div>
				{/if}
			</div>

		{:else if currentStep === 2}
			<!-- Step 3: Select Agent Preset -->
			<div class="step-content">
				<h2 class="step-title">Select AI Agent</h2>
				<p class="step-description">
					Choose an agent preset that matches your workflow. Each includes optimized models.
				</p>

				<div class="presets-grid">
					{#each agentPresets as preset}
						<PresetCard
							{preset}
							selected={projectData.selectedPreset === preset.id}
							expanded={projectData.expandedPreset === preset.id}
							onSelect={() => selectPreset(preset.id)}
							onToggleExpand={() => togglePresetExpand(preset.id)}
						/>
					{/each}
				</div>
			</div>

		{:else if currentStep === 3}
			<!-- Step 4: Configure Tools -->
			<div class="step-content">
				<h2 class="step-title">Enable Tools</h2>
				<p class="step-description">
					Select which tools the AI agent can use when analyzing your videos.
				</p>

				<div class="tools-grid">
					{#each availableTools as tool}
						<ToolToggle
							{tool}
							enabled={projectData.enabledTools.includes(tool.id)}
							onToggle={() => toggleTool(tool.id)}
						/>
					{/each}
				</div>
			</div>

		{:else if currentStep === 4}
			<!-- Step 5: Review -->
			<div class="step-content">
				<h2 class="step-title">Review & Create</h2>
				<p class="step-description">
					Review your project settings before creating.
				</p>

				<div class="review-sections">
					<div class="review-section">
						<h3>Project</h3>
						<div class="review-item">
							<span class="review-label">Name</span>
							<span class="review-value">{projectData.name}</span>
						</div>
						{#if projectData.description}
							<div class="review-item">
								<span class="review-label">Description</span>
								<span class="review-value">{projectData.description}</span>
							</div>
						{/if}
						{#if projectData.tags.length > 0}
							<div class="review-item">
								<span class="review-label">Tags</span>
								<div class="review-tags">
									{#each projectData.tags as tag}
										<span class="review-tag">{tag}</span>
									{/each}
								</div>
							</div>
						{/if}
					</div>

					<div class="review-section">
						<h3>Videos</h3>
						<div class="review-item">
							<span class="review-label">Files</span>
							<span class="review-value">
								{projectData.files.length === 0
									? 'No videos added'
									: `${projectData.files.length} video${projectData.files.length > 1 ? 's' : ''}`}
							</span>
						</div>
					</div>

					<div class="review-section">
						<h3>AI Agent</h3>
						{#if selectedPreset}
							<div class="review-item">
								<span class="review-label">Preset</span>
								<span class="review-value preset-value">
									{selectedPreset.name}
									{#if selectedPreset.badge}
										<Badge variant="success" size="sm">{selectedPreset.badge}</Badge>
									{/if}
								</span>
							</div>
							{#if selectedPreset.models.length > 0}
								<div class="review-models">
									{#each selectedPreset.models as model}
										<ModelChip {model} compact />
									{/each}
								</div>
							{/if}
						{/if}
					</div>

					<div class="review-section">
						<h3>Tools</h3>
						<div class="review-tools">
							{#each projectData.enabledTools as toolId}
								{@const tool = availableTools.find((t) => t.id === toolId)}
								{#if tool}
									<span class="review-tool">
										<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
											<polyline points="20 6 9 17 4 12" />
										</svg>
										{tool.name}
									</span>
								{/if}
							{/each}
						</div>
					</div>
				</div>
			</div>
		{/if}
	</div>

	<div class="wizard-footer">
		<div class="footer-left">
			{#if onCancel && currentStep === 0}
				<Button variant="ghost" onclick={onCancel}>Cancel</Button>
			{:else if currentStep > 0}
				<Button variant="secondary" onclick={handleBack}>Back</Button>
			{/if}
		</div>

		<div class="footer-right">
			<Button
				variant="primary"
				onclick={handleNext}
				disabled={!canProceed || isSubmitting}
				loading={isSubmitting}
			>
				{#if currentStep === wizardSteps.length - 1}
					{isSubmitting ? 'Creating...' : 'Create Project'}
				{:else}
					Continue
				{/if}
			</Button>
		</div>
	</div>
</div>

<style>
	.wizard {
		display: flex;
		flex-direction: column;
		height: 100%;
		max-width: 800px;
		margin: 0 auto;
	}

	.wizard-header {
		display: flex;
		justify-content: center;
		padding: var(--space-6);
		border-bottom: 1px solid var(--color-stone);
	}

	.wizard-content {
		flex: 1;
		overflow-y: auto;
		padding: var(--space-6);
	}

	.step-content {
		max-width: 600px;
		margin: 0 auto;
	}

	.step-title {
		font-size: var(--text-h1);
		font-weight: 600;
		color: var(--color-chalk);
		margin: 0 0 var(--space-2);
	}

	.step-description {
		font-size: var(--text-body);
		color: var(--color-silver);
		margin: 0 0 var(--space-6);
	}

	.form-group {
		margin-bottom: var(--space-4);
	}

	/* File List */
	.file-list {
		margin-top: var(--space-6);
	}

	.file-list-title {
		font-size: var(--text-small);
		font-weight: 600;
		color: var(--color-silver);
		margin: 0 0 var(--space-3);
	}

	.file-item {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: var(--space-3);
		background: var(--color-slate);
		border: 1px solid var(--color-stone);
		border-radius: var(--radius-md);
		margin-bottom: var(--space-2);
	}

	.file-info {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		color: var(--color-silver);
	}

	.file-name {
		color: var(--color-chalk);
		font-size: var(--text-body);
	}

	.file-size {
		font-size: var(--text-small);
		color: var(--color-ash);
	}

	.file-remove {
		display: flex;
		padding: var(--space-1);
		background: none;
		border: none;
		color: var(--color-ash);
		cursor: pointer;
		transition: color var(--duration-fast) var(--ease-out);
	}

	.file-remove:hover {
		color: var(--color-ember);
	}

	/* Presets Grid */
	.presets-grid {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}

	/* Tools Grid */
	.tools-grid {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}

	/* Review Sections */
	.review-sections {
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
	}

	.review-section {
		padding: var(--space-4);
		background: var(--color-slate);
		border: 1px solid var(--color-stone);
		border-radius: var(--radius-lg);
	}

	.review-section h3 {
		font-size: var(--text-small);
		font-weight: 600;
		color: var(--color-ash);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		margin: 0 0 var(--space-3);
	}

	.review-item {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		padding: var(--space-2) 0;
		border-bottom: 1px solid var(--color-stone);
	}

	.review-item:last-child {
		border-bottom: none;
	}

	.review-label {
		font-size: var(--text-small);
		color: var(--color-silver);
	}

	.review-value {
		font-size: var(--text-body);
		color: var(--color-chalk);
		text-align: right;
	}

	.preset-value {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.review-tags {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-1);
		justify-content: flex-end;
	}

	.review-tag {
		padding: var(--space-1) var(--space-2);
		background: var(--color-graphite);
		border-radius: var(--radius-sm);
		font-size: var(--text-small);
		color: var(--color-silver);
	}

	.review-models {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
		margin-top: var(--space-2);
	}

	.review-tools {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
	}

	.review-tool {
		display: inline-flex;
		align-items: center;
		gap: var(--space-1);
		padding: var(--space-1) var(--space-2);
		background: rgba(16, 185, 129, 0.1);
		border: 1px solid var(--color-jade);
		border-radius: var(--radius-sm);
		font-size: var(--text-small);
		color: var(--color-jade);
	}

	/* Footer */
	.wizard-footer {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: var(--space-4) var(--space-6);
		border-top: 1px solid var(--color-stone);
		background: var(--color-basalt);
	}

	.footer-left,
	.footer-right {
		display: flex;
		gap: var(--space-3);
	}

	@media (max-width: 640px) {
		.wizard-content {
			padding: var(--space-4);
		}

		.step-title {
			font-size: var(--text-h2);
		}

		.wizard-footer {
			padding: var(--space-4);
		}
	}
</style>
