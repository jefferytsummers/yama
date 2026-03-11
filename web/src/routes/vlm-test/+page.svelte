<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { Button, Card, Badge, Progress, TextArea } from '$lib/components';
	import { createVlmClient, type VlmApiClient } from '$lib/api/vlm';
	import type { VlmMetrics, VlmInferenceResponse } from '$lib/types/vlm';

	// Configuration
	let serverUrl = $state('http://localhost:8000');
	let vlmClient: VlmApiClient | null = $state(null);

	// Health state
	let health = $state<'unknown' | 'ready' | 'loading' | 'error'>('unknown');
	let healthError = $state<string | null>(null);

	// Metrics
	let metrics = $state<VlmMetrics | null>(null);
	let metricsInterval: ReturnType<typeof setInterval>;

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
	let canInfer = $derived(health === 'ready' && imageBase64 !== null && prompt.trim().length > 0 && !isInferring);

	onMount(() => {
		connectToServer();
	});

	onDestroy(() => {
		clearInterval(metricsInterval);
	});

	function connectToServer() {
		vlmClient = createVlmClient(serverUrl);
		checkHealth();
		startMetricsPolling();
	}

	async function checkHealth() {
		if (!vlmClient) return;

		try {
			const result = await vlmClient.checkHealth();
			health = result.status;
			healthError = null;
		} catch (e) {
			health = 'error';
			healthError = e instanceof Error ? e.message : 'Unknown error';
		}
	}

	function startMetricsPolling() {
		// Clear existing interval
		clearInterval(metricsInterval);

		// Poll metrics every 2 seconds
		metricsInterval = setInterval(async () => {
			if (!vlmClient) return;
			try {
				metrics = await vlmClient.getMetrics();
			} catch {
				// Ignore metrics errors silently
			}
		}, 2000);

		// Immediate fetch
		if (vlmClient) {
			vlmClient.getMetrics().then(m => metrics = m).catch(() => {});
		}
	}

	function handleServerUrlChange() {
		clearInterval(metricsInterval);
		health = 'unknown';
		metrics = null;
		response = null;
		error = null;
		connectToServer();
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
		if (!vlmClient || !imageBase64 || !canInfer) return;

		isInferring = true;
		error = null;
		response = null;

		try {
			response = await vlmClient.infer({
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
			<div class="health-status" class:ready={health === 'ready'} class:error={health === 'error'} class:loading={health === 'loading'}>
				<span class="health-dot"></span>
				<span>{health === 'ready' ? 'Ready' : health === 'loading' ? 'Loading...' : health === 'error' ? 'Error' : 'Unknown'}</span>
			</div>
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
						bind:value={serverUrl}
						class="text-input"
						placeholder="http://localhost:8000"
					/>
				</div>
				<Button variant="secondary" onclick={handleServerUrlChange}>
					Connect
				</Button>
			</div>
			{#if healthError}
				<p class="error-text">{healthError}</p>
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

					<Button variant="primary" disabled={!canInfer} loading={isInferring} onclick={runInference}>
						{isInferring ? 'Analyzing...' : 'Run Inference'}
					</Button>
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
					<h2 class="section-title">Server Metrics</h2>
					{#if metrics}
						<div class="metrics-grid">
							<div class="metric">
								<span class="metric-label">GPU Memory</span>
								<span class="metric-value">{metrics.gpu_memory_allocated_gb.toFixed(2)} GB</span>
								<span class="metric-sub">/ {metrics.gpu_memory_total_gb.toFixed(0)} GB total</span>
							</div>
							<div class="metric">
								<span class="metric-label">Model Load Time</span>
								<span class="metric-value">{metrics.model_load_time_seconds.toFixed(2)}s</span>
							</div>
							<div class="metric">
								<span class="metric-label">Total Requests</span>
								<span class="metric-value">{metrics.inference_requests_total}</span>
								<span class="metric-sub">
									{metrics.inference_success_total} ok / {metrics.inference_failed_total} failed
								</span>
							</div>
							<div class="metric">
								<span class="metric-label">Avg Latency</span>
								<span class="metric-value">{metrics.inference_latency_avg_ms.toFixed(0)} ms</span>
							</div>
							<div class="metric">
								<span class="metric-label">Avg Throughput</span>
								<span class="metric-value">{metrics.tokens_per_second_avg.toFixed(1)} tok/s</span>
							</div>
							<div class="metric">
								<span class="metric-label">Total Tokens</span>
								<span class="metric-value">{metrics.tokens_generated_total}</span>
							</div>
						</div>
					{:else}
						<p class="placeholder-text">Metrics loading...</p>
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

	.health-status {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--text-small);
		color: var(--color-ash);
	}

	.health-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--color-ash);
	}

	.health-status.ready .health-dot {
		background: var(--color-jade);
		box-shadow: 0 0 8px var(--color-jade);
	}

	.health-status.ready {
		color: var(--color-jade);
	}

	.health-status.error .health-dot {
		background: var(--color-ember);
	}

	.health-status.error {
		color: var(--color-ember);
	}

	.health-status.loading .health-dot {
		background: var(--color-amber);
		animation: pulse 1.5s ease-in-out infinite;
	}

	.health-status.loading {
		color: var(--color-amber);
	}

	@keyframes pulse {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.5; }
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

	.error-text {
		color: var(--color-ember);
		font-size: var(--text-small);
		margin-top: var(--space-2);
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
