<script lang="ts">
	import type { VideoSourceInfo } from '$lib/api';

	interface Props {
		source: VideoSourceInfo;
	}

	let { source }: Props = $props();
</script>

<div class="video-container">
	<div class="video-placeholder">
		<!-- WebRTC video element placeholder -->
		<video class="video-element" autoplay playsinline muted>
			<!-- WebRTC stream will be attached here -->
		</video>

		<div class="placeholder-overlay">
			<div class="placeholder-icon">
				<svg
					xmlns="http://www.w3.org/2000/svg"
					width="48"
					height="48"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="1.5"
				>
					<rect x="2" y="4" width="20" height="16" rx="2" />
					<circle cx="12" cy="12" r="3" />
					<path d="M17 9h0" />
				</svg>
			</div>
			<p class="placeholder-text">
				{#if source.enabled}
					Connecting to {source.name}...
				{:else}
					{source.name} (disabled)
				{/if}
			</p>
			<p class="placeholder-url">{source.url}</p>
		</div>
	</div>

	<div class="video-info">
		<span class="video-name">{source.name}</span>
		<span class={`video-status status-${source.status === 'connected' ? 'running' : 'stopped'}`}>
			{source.status}
		</span>
	</div>
</div>

<style>
	.video-container {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.video-placeholder {
		position: relative;
		aspect-ratio: 16 / 9;
		background-color: var(--color-bg);
		border-radius: var(--radius);
		overflow: hidden;
	}

	.video-element {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: none; /* Hidden until WebRTC stream is attached */
	}

	.placeholder-overlay {
		position: absolute;
		inset: 0;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		color: var(--color-text-muted);
	}

	.placeholder-icon {
		opacity: 0.5;
	}

	.placeholder-text {
		font-size: 0.875rem;
	}

	.placeholder-url {
		font-size: 0.75rem;
		font-family: var(--font-mono);
		opacity: 0.6;
	}

	.video-info {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0 0.25rem;
	}

	.video-name {
		font-weight: 500;
	}

	.video-status {
		font-size: 0.75rem;
		text-transform: capitalize;
	}
</style>
