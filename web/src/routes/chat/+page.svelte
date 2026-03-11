<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { ChatBubble, Badge, Progress, DropZone, Button } from '$lib/components';
	import { apiClient } from '$lib/api/client';
	import { isTauri, pickVideoFiles, videoFileToAttachment, getBackendStatus, type BackendStatus } from '$lib/tauri/commands';
	import type { ChatMessage, VideoAttachment, VlmModel, InferenceJob, FrameResult } from '$lib/types/chat';

	// State
	let messages = $state<ChatMessage[]>([]);
	let draftText = $state('');
	let draftAttachments = $state<VideoAttachment[]>([]);
	let models = $state<VlmModel[]>([]);
	let selectedModelIndex = $state(0);
	let backendStatus = $state<BackendStatus>({ ready: false, http_port: 8080, event_bus_port: 8765 });
	let isInferring = $state(false);
	let messagesEndRef: HTMLDivElement | undefined;
	let eventSource: EventSource | undefined;

	// Computed
	let canSend = $derived(draftText.trim().length > 0 || draftAttachments.length > 0);
	let selectedModel = $derived(models[selectedModelIndex]?.id ?? 'vlm-default');

	let statusInterval: ReturnType<typeof setInterval>;

	onMount(() => {
		// Initialize async
		(async () => {
			// Check backend status
			backendStatus = await getBackendStatus();

			// Add welcome message
			messages = [
				{
					id: crypto.randomUUID(),
					role: 'system',
					content: 'Drop a video file and describe what you want to analyze.',
					attachments: [],
					frame_results: [],
					timestamp: new Date()
				}
			];

			// Load available models
			try {
				const response = await apiClient.get<{ models: VlmModel[] }>('/api/inference/models');
				models = response.models;
			} catch (error) {
				console.error('Failed to load models:', error);
			}
		})();

		// Poll backend status periodically
		statusInterval = setInterval(async () => {
			backendStatus = await getBackendStatus();
		}, 5000);
	});

	onDestroy(() => {
		clearInterval(statusInterval);
		eventSource?.close();
	});

	function scrollToBottom() {
		messagesEndRef?.scrollIntoView({ behavior: 'smooth' });
	}

	async function handlePickFiles() {
		const files = await pickVideoFiles();
		for (const file of files) {
			draftAttachments = [...draftAttachments, videoFileToAttachment(file)];
		}
	}

	function handleDropFiles(files: FileList) {
		for (let i = 0; i < files.length; i++) {
			const file = files[i];
			if (file.type.startsWith('video/')) {
				draftAttachments = [
					...draftAttachments,
					{
						id: crypto.randomUUID(),
						path: '', // Browser doesn't expose full path
						filename: file.name,
						size: file.size,
						status: 'pending'
					}
				];
			}
		}
	}

	function removeAttachment(id: string) {
		draftAttachments = draftAttachments.filter((a) => a.id !== id);
	}

	async function sendMessage() {
		if (!canSend) return;

		const text = draftText;
		const attachments = [...draftAttachments];

		// Clear draft
		draftText = '';
		draftAttachments = [];

		// Add user message
		const userMessage: ChatMessage = {
			id: crypto.randomUUID(),
			role: 'user',
			content: text,
			attachments,
			frame_results: [],
			timestamp: new Date()
		};
		messages = [...messages, userMessage];
		scrollToBottom();

		// If no attachments, show prompt
		if (attachments.length === 0) {
			messages = [
				...messages,
				{
					id: crypto.randomUUID(),
					role: 'system',
					content: 'Please attach a video file to analyze.',
					attachments: [],
					frame_results: [],
					timestamp: new Date()
				}
			];
			scrollToBottom();
			return;
		}

		// Start inference for each attachment
		for (const attachment of attachments) {
			await startInference(attachment, text);
		}
	}

	async function startInference(attachment: VideoAttachment, prompt: string) {
		isInferring = true;

		try {
			// Upload video if needed (in browser mode)
			let uploadId: string | undefined;
			if (!isTauri() && attachment.path === '') {
				// TODO: Handle browser file uploads
				console.warn('Browser file upload not implemented yet');
				return;
			}

			// For Tauri mode, the path is already a local path
			// We need to register it via the API
			const uploadResponse = await apiClient.post<{ upload_id: string; success: boolean }>('/api/inference/upload', {
				path: attachment.path,
				filename: attachment.filename
			}).catch(() => null);

			if (!uploadResponse?.success) {
				// Try direct path registration (for local files in Tauri)
				uploadId = crypto.randomUUID();
			} else {
				uploadId = uploadResponse.upload_id;
			}

			// Start inference
			const startResponse = await apiClient.post<{ job_id: string; success: boolean }>('/api/inference/start', {
				source: 'upload',
				upload_id: uploadId,
				model: selectedModel,
				prompt
			});

			if (!startResponse.success || !startResponse.job_id) {
				throw new Error('Failed to start inference');
			}

			const jobId = startResponse.job_id;

			// Add assistant message (pending)
			const assistantMessage: ChatMessage = {
				id: crypto.randomUUID(),
				role: 'assistant',
				content: '',
				attachments: [],
				frame_results: [],
				progress: { current_frame: 0, total_frames: 0, percent: 0 },
				job_id: jobId,
				timestamp: new Date()
			};
			messages = [...messages, assistantMessage];
			scrollToBottom();

			// Poll for job status
			await pollJobStatus(jobId);
		} catch (error) {
			console.error('Inference error:', error);
			messages = [
				...messages,
				{
					id: crypto.randomUUID(),
					role: 'system',
					content: `Error: ${error instanceof Error ? error.message : 'Unknown error'}`,
					attachments: [],
					frame_results: [],
					timestamp: new Date()
				}
			];
		} finally {
			isInferring = false;
			scrollToBottom();
		}
	}

	async function pollJobStatus(jobId: string) {
		const maxPolls = 600; // 10 minutes max
		const pollInterval = 1000; // 1 second

		for (let i = 0; i < maxPolls; i++) {
			try {
				const job = await apiClient.get<InferenceJob>(`/api/inference/jobs/${jobId}`);

				// Update message with progress
				messages = messages.map((m) => {
					if (m.job_id === jobId) {
						return {
							...m,
							progress:
								job.status === 'completed' || job.status === 'failed'
									? undefined
									: {
											current_frame: job.current_frame,
											total_frames: job.total_frames,
											percent: job.progress_percent
										},
							frame_results: job.results,
							error: job.error,
							content: formatResults(job.results)
						};
					}
					return m;
				});
				scrollToBottom();

				if (job.status === 'completed' || job.status === 'failed') {
					break;
				}

				await new Promise((resolve) => setTimeout(resolve, pollInterval));
			} catch (error) {
				console.error('Error polling job status:', error);
				break;
			}
		}
	}

	function formatResults(results: FrameResult[]): string {
		if (results.length === 0) return 'Analyzing...';

		return results.map((r) => r.text).join('\n\n');
	}

	function formatBytes(bytes: number): string {
		if (bytes === 0) return '0 B';
		const k = 1024;
		const sizes = ['B', 'KB', 'MB', 'GB'];
		const i = Math.floor(Math.log(bytes) / Math.log(k));
		return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			sendMessage();
		}
	}
</script>

<svelte:head>
	<title>Yama - Chat</title>
</svelte:head>

<div class="chat-container">
	<!-- Header -->
	<header class="chat-header">
		<div class="header-left">
			<span class="logo">◆</span>
			<h1>Yama</h1>
			<Badge variant="inference">Chat</Badge>
		</div>

		<div class="header-right">
			{#if isInferring}
				<Badge variant="inference" pulse>Analyzing...</Badge>
			{/if}

			<select class="model-select" bind:value={selectedModelIndex}>
				{#each models as model, i}
					<option value={i}>{model.name}</option>
				{/each}
				{#if models.length === 0}
					<option value={0}>Default Model</option>
				{/if}
			</select>
		</div>
	</header>

	<!-- Messages -->
	<main class="chat-messages">
		{#each messages as message (message.id)}
			<ChatBubble role={message.role} timestamp={message.timestamp} streaming={!!message.progress}>
				{#if message.content}
					<p class="message-text">{message.content}</p>
				{/if}

				{#if message.progress}
					<div class="message-progress">
						<Progress
							value={message.progress.percent}
							max={100}
							variant="gradient"
							size="sm"
							showLabel
						/>
						<span class="progress-detail">
							Frame {message.progress.current_frame}/{message.progress.total_frames}
						</span>
					</div>
				{/if}

				{#if message.attachments.length > 0}
					<div class="message-attachments">
						{#each message.attachments as attachment}
							<div class="attachment-chip">
								<span class="attachment-icon">🎬</span>
								<span class="attachment-name">{attachment.filename}</span>
								<span class="attachment-size">{formatBytes(attachment.size)}</span>
							</div>
						{/each}
					</div>
				{/if}

				{#if message.error}
					<p class="message-error">{message.error}</p>
				{/if}
			</ChatBubble>
		{/each}
		<div bind:this={messagesEndRef}></div>
	</main>

	<!-- Input Area -->
	<footer class="chat-input-area">
		{#if draftAttachments.length > 0}
			<div class="draft-attachments">
				{#each draftAttachments as attachment (attachment.id)}
					<div class="draft-chip">
						<span>🎬 {attachment.filename}</span>
						<button
							class="remove-btn"
							onclick={() => removeAttachment(attachment.id)}
							aria-label="Remove attachment"
						>
							×
						</button>
					</div>
				{/each}
			</div>
		{/if}

		<div class="input-row">
			{#if isTauri()}
				<button class="attach-btn" onclick={handlePickFiles} aria-label="Attach video">
					<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<path d="M21.44 11.05l-9.19 9.19a6 6 0 01-8.49-8.49l9.19-9.19a4 4 0 015.66 5.66l-9.2 9.19a2 2 0 01-2.83-2.83l8.49-8.48" />
					</svg>
				</button>
			{:else}
				<DropZone ondrop={handleDropFiles} accept="video/*" />
			{/if}

			<textarea
				class="message-input"
				placeholder="Describe what you want to analyze..."
				bind:value={draftText}
				onkeydown={handleKeydown}
				disabled={isInferring}
				rows={1}
			></textarea>

			<Button variant="primary" disabled={!canSend || isInferring} onclick={sendMessage}>
				Send
			</Button>
		</div>
	</footer>

	<!-- Status Bar -->
	<div class="status-bar">
		<div class="status-indicator" class:ready={backendStatus.ready}>
			<span class="status-dot"></span>
			<span>{backendStatus.ready ? 'Connected' : 'Connecting...'}</span>
		</div>
		{#if isTauri()}
			<span class="status-badge">Desktop App</span>
		{:else}
			<span class="status-badge">Web UI</span>
		{/if}
	</div>
</div>

<style>
	.chat-container {
		display: flex;
		flex-direction: column;
		height: 100vh;
		background: var(--color-obsidian);
	}

	/* Header */
	.chat-header {
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

	.chat-header h1 {
		font-size: var(--text-h3);
		font-weight: 600;
		color: var(--color-chalk);
		margin: 0;
	}

	.header-right {
		display: flex;
		align-items: center;
		gap: var(--space-4);
	}

	.model-select {
		background: var(--color-graphite);
		color: var(--color-chalk);
		border: 1px solid var(--color-stone);
		border-radius: var(--radius-md);
		padding: var(--space-2) var(--space-3);
		font-size: var(--text-small);
		cursor: pointer;
	}

	.model-select:focus {
		outline: none;
		border-color: var(--color-amber);
	}

	/* Messages */
	.chat-messages {
		flex: 1;
		overflow-y: auto;
		padding: var(--space-6);
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}

	.message-text {
		margin: 0;
		white-space: pre-wrap;
	}

	.message-progress {
		margin-top: var(--space-2);
	}

	.progress-detail {
		font-size: var(--text-tiny);
		color: var(--color-ash);
		margin-top: var(--space-1);
		display: block;
	}

	.message-attachments {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
		margin-top: var(--space-2);
	}

	.attachment-chip {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		padding: var(--space-1) var(--space-2);
		background: var(--color-graphite);
		border-radius: var(--radius-sm);
		font-size: var(--text-small);
	}

	.attachment-icon {
		font-size: 14px;
	}

	.attachment-name {
		color: var(--color-chalk);
	}

	.attachment-size {
		color: var(--color-ash);
	}

	.message-error {
		color: var(--color-ember);
		font-size: var(--text-small);
		margin-top: var(--space-2);
	}

	/* Input Area */
	.chat-input-area {
		padding: var(--space-4) var(--space-6);
		background: var(--color-basalt);
		border-top: 1px solid var(--color-stone);
	}

	.draft-attachments {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
		margin-bottom: var(--space-3);
	}

	.draft-chip {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-1) var(--space-2);
		background: var(--color-graphite);
		border-radius: var(--radius-md);
		font-size: var(--text-small);
		color: var(--color-chalk);
	}

	.remove-btn {
		background: none;
		border: none;
		color: var(--color-silver);
		cursor: pointer;
		padding: 0 var(--space-1);
		font-size: 16px;
		line-height: 1;
	}

	.remove-btn:hover {
		color: var(--color-ember);
	}

	.input-row {
		display: flex;
		gap: var(--space-3);
		align-items: flex-end;
	}

	.attach-btn {
		flex-shrink: 0;
		width: 44px;
		height: 44px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--color-graphite);
		border: 1px solid var(--color-stone);
		border-radius: var(--radius-md);
		color: var(--color-silver);
		cursor: pointer;
		transition: all var(--duration-fast) var(--ease-out);
	}

	.attach-btn:hover {
		background: var(--color-slate);
		color: var(--color-amber);
	}

	.message-input {
		flex: 1;
		min-height: 44px;
		max-height: 200px;
		padding: var(--space-3);
		background: var(--color-slate);
		border: 1px solid var(--color-stone);
		border-radius: var(--radius-lg);
		color: var(--color-chalk);
		font-size: var(--text-body);
		resize: none;
		font-family: inherit;
	}

	.message-input::placeholder {
		color: var(--color-ash);
	}

	.message-input:focus {
		outline: none;
		border-color: var(--color-amber);
	}

	.message-input:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	/* Status Bar */
	.status-bar {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: var(--space-2) var(--space-6);
		background: var(--color-obsidian);
		border-top: 1px solid var(--color-stone);
	}

	.status-indicator {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--text-small);
		color: var(--color-ash);
	}

	.status-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--color-ash);
	}

	.status-indicator.ready .status-dot {
		background: var(--color-jade);
	}

	.status-indicator.ready {
		color: var(--color-silver);
	}

	.status-badge {
		font-size: var(--text-tiny);
		color: var(--color-ash);
		padding: var(--space-1) var(--space-2);
		background: var(--color-basalt);
		border-radius: var(--radius-sm);
	}
</style>
