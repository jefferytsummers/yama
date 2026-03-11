<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { Button, Card, Badge, Progress, TextArea, ConnectionStatus } from '$lib/components';
	import { vlmConnection, connectionStatus, vlmMetrics, modelReady } from '$lib/stores/vlmConnection';
	import type { VlmInferenceResponse } from '$lib/types/vlm';

	// Server URL (bound to input)
	let serverUrlInput = $state('http://localhost:8000');

	// Image state
	let selectedImage = $state<File | null>(null);
	let imagePreview = $state<string | null>(null);
	let imageBase64 = $state<string | null>(null);

	// Inference state
	let prompt = $state('Describe this image in detail.');
	let maxTokens = $state(256);
	let temperature = $state(0.7);
	let isInferring = $state(false);
	let response = $state<VlmInferenceResponse | null>(null);
	let error = $state<string | null>(null);

	// Derived
	let canInfer = $derived($modelReady && imageBase64 !== null && prompt.trim().length > 0 && !isInferring);

	onMount(() => {
		// Auto-connect on mount
		vlmConnection.connect(serverUrlInput);
	});

	onDestroy(() => {
		vlmConnection.disconnect();
	});

	function handleServerUrlChange() {
		vlmConnection.setServerUrl(serverUrlInput);
		response = null;
		error = null;
	}

	function handleImageSelect(e: Event) {
		const input = e.target as HTMLInputElement;
		const file = input.files?.[0];
		if (!file) return;

		selectedImage = file;
		error = null;
		response = null;

		// Create preview
		const reader = new FileReader();
		reader.onload = () => {
			imagePreview = reader.result as string;
			// Extract base64 data (remove data:image/...;base64, prefix)
			const base64Data = (reader.result as string).split(',')[1];
			imageBase64 = base64Data;
		};
		reader.readAsDataURL(file);
	}

	function clearImage() {
		selectedImage = null;
		imagePreview = null;
		imageBase64 = null;
		response = null;
		error = null;
	}

	async function runInference() {
		const client = vlmConnection.getClient();
		if (!client || !imageBase64 || !canInfer) return;

		isInferring = true;
		error = null;
		response = null;

		try {
			response = await client.infer({
				prompt,
				image: imageBase64,
				max_tokens: maxTokens,
				temperature
			});
		} catch (e) {
			error = e instanceof Error ? e.message : 'Inference failed';
		} finally {
			isInferring = false;
		}
	}
</script>

<svelte:head>
	<title>Yama - VLM Test</title>
</svelte:head>

<div class="vlm-test-container">
	<!-- Header -->
	<header class="header">
		<div class="header-left">
			<span class="logo">◆</span>
			<h1>VLM Test</h1>
			<Badge variant="inference">LLaVA-1.5-7B</Badge>
		</div>
		<div class="header-right">
			<ConnectionStatus status={$connectionStatus} size="md" />
		</div>
	</header>

	<main class="main-content">
		<!-- Server Configuration -->
		<Card>
			<h2 class="section-title">Server Configuration</h2>
			<div class="server-config">
				<div class="input-group">
					<label for="server-url">VLM Server URL</label>
					<input
						id="server-url"
						type="text"
						bind:value={serverUrlInput}
						class="text-input"
						placeholder="http://localhost:8000"
					/>
				</div>
				<Button variant="secondary" onclick={handleServerUrlChange}>
					Reconnect
				</Button>
				<Button variant="ghost" onclick={() => vlmConnection.refresh()}>
					Refresh
				</Button>
			</div>
			{#if $connectionStatus === 'disconnected'}
				<p class="status-message error">Cannot reach VLM server. Retrying automatically...</p>
			{:else if $connectionStatus === 'connecting'}
				<p class="status-message connecting">Establishing connection...</p>
			{:else if $connectionStatus === 'stale'}
				<p class="status-message stale">Connection may be unstable. Checking...</p>
			{:else if !$modelReady}
				<p class="status-message loading">Server connected, model is loading...</p>
			{/if}
		</Card>

		<div class="two-column">
			<!-- Left: Image & Prompt -->
			<div class="column">
				<!-- Image Upload -->
				<Card>
					<h2 class="section-title">Image</h2>
					{#if imagePreview}
						<div class="image-preview-container">
							<img src={imagePreview} alt="Selected" class="image-preview" />
							<button class="clear-image" onclick={clearImage} aria-label="Clear image">×</button>
						</div>
						<p class="image-info">
							{selectedImage?.name} ({((selectedImage?.size ?? 0) / 1024).toFixed(1)} KB)
						</p>
					{:else}
						<label class="image-dropzone">
							<input
								type="file"
								accept="image/*"
								class="visually-hidden"
								onchange={handleImageSelect}
							/>
							<div class="dropzone-content">
								<svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
									<rect x="3" y="3" width="18" height="18" rx="2" ry="2"/>
									<circle cx="8.5" cy="8.5" r="1.5"/>
									<path d="m21 15-5-5L5 21"/>
								</svg>
								<p><span class="cta">Click to upload</span> an image</p>
								<p class="hint">JPEG, PNG, WebP supported</p>
							</div>
						</label>
					{/if}
				</Card>

				<!-- Prompt -->
				<Card>
					<h2 class="section-title">Prompt</h2>
					<TextArea
						bind:value={prompt}
						placeholder="Describe what you want to know about the image..."
						rows={3}
					/>

					<div class="params-row">
						<div class="param">
							<label for="max-tokens">Max Tokens</label>
							<input
								id="max-tokens"
								type="number"
								bind:value={maxTokens}
								min={16}
								max={2048}
								class="number-input"
							/>
						</div>
						<div class="param">
							<label for="temperature">Temperature</label>
							<input
								id="temperature"
								type="number"
								bind:value={temperature}
								min={0}
								max={2}
								step={0.1}
								class="number-input"
							/>
						</div>
					</div>

					<div class="action-row">
						<Button variant="primary" disabled={!canInfer} loading={isInferring} onclick={runInference}>
							{isInferring ? 'Analyzing...' : 'Run Inference'}
						</Button>
						{#if !$modelReady && $connectionStatus === 'connected'}
							<span class="action-hint">Waiting for model to load...</span>
						{:else if !imageBase64}
							<span class="action-hint">Upload an image to continue</span>
						{/if}
					</div>
				</Card>
			</div>

			<!-- Right: Response & Metrics -->
			<div class="column">
				<!-- Response -->
				<Card>
					<h2 class="section-title">Response</h2>
					{#if isInferring}
						<div class="inference-loading">
							<Badge variant="inference" pulse>Processing...</Badge>
							<Progress value={0} max={100} variant="gradient" indeterminate />
						</div>
					{:else if error}
						<div class="error-box">
							<Badge variant="error">Error</Badge>
							<p>{error}</p>
						</div>
					{:else if response}
						<div class="response-content">
							<p class="response-text">{response.response}</p>
							<div class="response-stats">
								<span class="stat">
									<strong>{response.tokens_generated}</strong> tokens
								</span>
								<span class="stat">
									<strong>{response.tokens_per_second.toFixed(1)}</strong> tok/s
								</span>
								<span class="stat">
									<strong>{(response.generation_time_ms / 1000).toFixed(2)}</strong> sec
								</span>
							</div>
						</div>
					{:else}
						<p class="placeholder-text">Response will appear here after inference.</p>
					{/if}
				</Card>

				<!-- Metrics -->
				<Card>
					<div class="metrics-header">
						<h2 class="section-title">Server Metrics</h2>
						<ConnectionStatus status={$connectionStatus} size="sm" showLabel={false} />
					</div>
					{#if $vlmMetrics}
						<div class="metrics-grid">
							<div class="metric">
								<span class="metric-label">GPU Memory</span>
								<span class="metric-value">{$vlmMetrics.gpu_memory_allocated_gb.toFixed(2)} GB</span>
								<span class="metric-sub">/ {$vlmMetrics.gpu_memory_total_gb.toFixed(0)} GB total</span>
							</div>
							<div class="metric">
								<span class="metric-label">Model Load Time</span>
								<span class="metric-value">{$vlmMetrics.model_load_time_seconds.toFixed(2)}s</span>
							</div>
							<div class="metric">
								<span class="metric-label">Total Requests</span>
								<span class="metric-value">{$vlmMetrics.inference_requests_total}</span>
								<span class="metric-sub">
									{$vlmMetrics.inference_success_total} ok / {$vlmMetrics.inference_failed_total} failed
								</span>
							</div>
							<div class="metric">
								<span class="metric-label">Avg Latency</span>
								<span class="metric-value">{$vlmMetrics.inference_latency_avg_ms.toFixed(0)} ms</span>
							</div>
							<div class="metric">
								<span class="metric-label">Avg Throughput</span>
								<span class="metric-value">{$vlmMetrics.tokens_per_second_avg.toFixed(1)} tok/s</span>
							</div>
							<div class="metric">
								<span class="metric-label">Total Tokens</span>
								<span class="metric-value">{$vlmMetrics.tokens_generated_total}</span>
							</div>
						</div>
					{:else if $connectionStatus === 'connected' || $connectionStatus === 'connecting'}
						<div class="metrics-loading">
							<Progress value={0} max={100} variant="default" indeterminate size="sm" />
							<p class="placeholder-text">Loading metrics...</p>
						</div>
					{:else}
						<p class="placeholder-text">Connect to server to view metrics</p>
					{/if}
				</Card>
			</div>
		</div>
	</main>
</div>

<style>
	.vlm-test-container {
		display: flex;
		flex-direction: column;
		min-height: 100vh;
		background: var(--color-obsidian);
	}

	/* Header */
	.header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: var(--space-3) var(--space-6);
		background: var(--color-basalt);
		border-bottom: 1px solid var(--color-stone);
	}

	.header-left {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.logo {
		font-size: 24px;
		color: var(--color-amber);
	}

	.header h1 {
		font-size: var(--text-h3);
		font-weight: 600;
		color: var(--color-chalk);
		margin: 0;
	}

	/* Main Content */
	.main-content {
		padding: var(--space-6);
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}

	.section-title {
		font-size: var(--text-body);
		font-weight: 600;
		color: var(--color-chalk);
		margin: 0 0 var(--space-3) 0;
	}

	/* Server Config */
	.server-config {
		display: flex;
		gap: var(--space-3);
		align-items: flex-end;
	}

	.input-group {
		flex: 1;
	}

	.input-group label {
		display: block;
		font-size: var(--text-small);
		color: var(--color-silver);
		margin-bottom: var(--space-1);
	}

	.text-input {
		width: 100%;
		padding: var(--space-2) var(--space-3);
		background: var(--color-slate);
		border: 1px solid var(--color-stone);
		border-radius: var(--radius-md);
		color: var(--color-chalk);
		font-size: var(--text-body);
		font-family: var(--font-mono);
	}

	.text-input:focus {
		outline: none;
		border-color: var(--color-amber);
	}

	.status-message {
		font-size: var(--text-small);
		margin-top: var(--space-2);
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-md);
	}

	.status-message.error {
		color: var(--color-ember);
		background: rgba(239, 68, 68, 0.1);
	}

	.status-message.connecting {
		color: var(--color-jade);
		background: rgba(16, 185, 129, 0.1);
	}

	.status-message.stale {
		color: var(--color-amber);
		background: rgba(245, 158, 11, 0.1);
	}

	.status-message.loading {
		color: var(--color-violet);
		background: rgba(139, 92, 246, 0.1);
	}

	/* Two Column Layout */
	.two-column {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-4);
	}

	@media (max-width: 900px) {
		.two-column {
			grid-template-columns: 1fr;
		}
	}

	.column {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}

	/* Image Upload */
	.image-dropzone {
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: 200px;
		background: var(--color-basalt);
		border: 2px dashed var(--color-stone);
		border-radius: var(--radius-xl);
		cursor: pointer;
		transition: all var(--duration-fast) var(--ease-out);
	}

	.image-dropzone:hover {
		border-color: var(--color-silver);
		background: var(--color-slate);
	}

	.dropzone-content {
		display: flex;
		flex-direction: column;
		align-items: center;
		text-align: center;
		gap: var(--space-2);
		color: var(--color-silver);
	}

	.dropzone-content .cta {
		color: var(--color-amber);
		font-weight: 500;
	}

	.dropzone-content .hint {
		font-size: var(--text-small);
		color: var(--color-ash);
	}

	.image-preview-container {
		position: relative;
	}

	.image-preview {
		width: 100%;
		max-height: 300px;
		object-fit: contain;
		border-radius: var(--radius-lg);
		background: var(--color-basalt);
	}

	.clear-image {
		position: absolute;
		top: var(--space-2);
		right: var(--space-2);
		width: 28px;
		height: 28px;
		border-radius: 50%;
		background: var(--color-graphite);
		border: none;
		color: var(--color-chalk);
		font-size: 18px;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: background var(--duration-fast) var(--ease-out);
	}

	.clear-image:hover {
		background: var(--color-ember);
	}

	.image-info {
		font-size: var(--text-small);
		color: var(--color-silver);
		margin-top: var(--space-2);
	}

	/* Parameters */
	.params-row {
		display: flex;
		gap: var(--space-4);
		margin: var(--space-4) 0;
	}

	.param {
		flex: 1;
	}

	.param label {
		display: block;
		font-size: var(--text-small);
		color: var(--color-silver);
		margin-bottom: var(--space-1);
	}

	.number-input {
		width: 100%;
		padding: var(--space-2) var(--space-3);
		background: var(--color-slate);
		border: 1px solid var(--color-stone);
		border-radius: var(--radius-md);
		color: var(--color-chalk);
		font-size: var(--text-body);
	}

	.number-input:focus {
		outline: none;
		border-color: var(--color-amber);
	}

	.action-row {
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}

	.action-hint {
		font-size: var(--text-small);
		color: var(--color-ash);
		font-style: italic;
	}

	/* Response */
	.inference-loading {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		padding: var(--space-4);
	}

	.error-box {
		padding: var(--space-3);
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid var(--color-ember);
		border-radius: var(--radius-md);
	}

	.error-box p {
		color: var(--color-ember);
		margin: var(--space-2) 0 0 0;
		font-size: var(--text-small);
	}

	.response-content {
		background: var(--color-basalt);
		border-radius: var(--radius-lg);
		padding: var(--space-4);
	}

	.response-text {
		color: var(--color-chalk);
		white-space: pre-wrap;
		line-height: 1.6;
		margin: 0 0 var(--space-4) 0;
	}

	.response-stats {
		display: flex;
		gap: var(--space-4);
		padding-top: var(--space-3);
		border-top: 1px solid var(--color-stone);
	}

	.stat {
		font-size: var(--text-small);
		color: var(--color-silver);
	}

	.stat strong {
		color: var(--color-violet);
		font-weight: 600;
	}

	.placeholder-text {
		color: var(--color-ash);
		font-style: italic;
	}

	/* Metrics */
	.metrics-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: var(--space-3);
	}

	.metrics-header .section-title {
		margin: 0;
	}

	.metrics-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: var(--space-3);
	}

	.metric {
		background: var(--color-basalt);
		padding: var(--space-3);
		border-radius: var(--radius-md);
	}

	.metric-label {
		display: block;
		font-size: var(--text-small);
		color: var(--color-ash);
		margin-bottom: var(--space-1);
	}

	.metric-value {
		display: block;
		font-size: var(--text-h3);
		font-weight: 600;
		color: var(--color-chalk);
	}

	.metric-sub {
		display: block;
		font-size: var(--text-tiny);
		color: var(--color-silver);
		margin-top: var(--space-1);
	}

	.metrics-loading {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: var(--space-4);
	}

	/* Utilities */
	.visually-hidden {
		position: absolute;
		width: 1px;
		height: 1px;
		padding: 0;
		margin: -1px;
		overflow: hidden;
		clip: rect(0, 0, 0, 0);
		white-space: nowrap;
		border: 0;
	}
</style>
