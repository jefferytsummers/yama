<script lang="ts">
	import ThemeToggle from './ThemeToggle.svelte';

	interface Props {
		onGetStarted: () => void;
		onWatchDemo?: () => void;
	}

	let { onGetStarted, onWatchDemo }: Props = $props();
</script>

<section class="hero">
	<div class="hero-header">
		<ThemeToggle size="md" />
	</div>

	<div class="hero-content">
		<div class="logo-mark">
			<svg width="80" height="80" viewBox="0 0 64 64" fill="none">
				<path d="M32 8L8 52h48L32 8z" stroke="currentColor" stroke-width="2" fill="none" />
				<path d="M32 18L14 48h36L32 18z" fill="currentColor" opacity="0.2" />
			</svg>
		</div>

		<h1 class="hero-title">YAMA</h1>
		<p class="hero-tagline">Video Intelligence for Professionals</p>

		<p class="hero-description">
			Frame-accurate analysis&nbsp;&nbsp;•&nbsp;&nbsp;Real-time AI inference&nbsp;&nbsp;•&nbsp;&nbsp;Local-first
			privacy
		</p>

		<div class="hero-actions">
			<button class="btn-primary" onclick={onGetStarted}>
				<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<path d="M12 3l1.912 5.813a2 2 0 001.275 1.275L21 12l-5.813 1.912a2 2 0 00-1.275 1.275L12 21l-1.912-5.813a2 2 0 00-1.275-1.275L3 12l5.813-1.912a2 2 0 001.275-1.275L12 3z" />
				</svg>
				Start Free
			</button>
			{#if onWatchDemo}
				<button class="btn-secondary" onclick={onWatchDemo}>
					Watch Demo
					<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<path d="M5 12h14M12 5l7 7-7 7" />
					</svg>
				</button>
			{/if}
		</div>
	</div>

	<div class="hero-visual">
		<div class="screenshot-frame">
			<div class="screenshot-header">
				<span class="dot red"></span>
				<span class="dot yellow"></span>
				<span class="dot green"></span>
			</div>
			<div class="screenshot-content">
				<div class="mock-sidebar">
					<div class="mock-item active"></div>
					<div class="mock-item"></div>
					<div class="mock-item"></div>
				</div>
				<div class="mock-main">
					<div class="mock-video">
						<svg width="48" height="48" viewBox="0 0 24 24" fill="none">
							<polygon points="5 3 19 12 5 21 5 3" fill="currentColor" opacity="0.8" />
						</svg>
					</div>
					<div class="mock-chat">
						<div class="mock-bubble user"></div>
						<div class="mock-bubble assistant"></div>
						<div class="mock-bubble assistant short"></div>
					</div>
				</div>
			</div>
		</div>
	</div>
</section>

<style>
	.hero {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-10);
		padding: var(--space-12) var(--space-6);
		text-align: center;
		position: relative;
	}

	.hero-header {
		position: absolute;
		top: var(--space-4);
		right: var(--space-4);
	}

	.hero-content {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-4);
		max-width: 600px;
	}

	.logo-mark {
		color: var(--color-primary);
		margin-bottom: var(--space-2);
	}

	.hero-title {
		font-size: 48px;
		font-weight: 700;
		letter-spacing: 0.1em;
		color: var(--color-text-primary);
		margin: 0;
	}

	.hero-tagline {
		font-size: var(--text-h2);
		color: var(--color-text-secondary);
		margin: 0;
	}

	.hero-description {
		font-size: var(--text-body);
		color: var(--color-text-tertiary);
		margin-top: var(--space-2);
	}

	.hero-actions {
		display: flex;
		gap: var(--space-4);
		margin-top: var(--space-6);
	}

	.btn-primary {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-3) var(--space-6);
		font-family: var(--font-sans);
		font-size: var(--text-body);
		font-weight: 600;
		color: var(--color-on-primary);
		background: var(--color-primary);
		border: none;
		border-radius: var(--radius-lg);
		cursor: pointer;
		transition: all var(--duration-fast) var(--ease-out);
	}

	.btn-primary:hover {
		background: var(--color-primary-hover);
		transform: translateY(-1px);
	}

	.btn-secondary {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-3) var(--space-5);
		font-family: var(--font-sans);
		font-size: var(--text-body);
		font-weight: 500;
		color: var(--color-text-secondary);
		background: transparent;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		cursor: pointer;
		transition: all var(--duration-fast) var(--ease-out);
	}

	.btn-secondary:hover {
		color: var(--color-text-primary);
		background: var(--color-bg-tertiary);
		border-color: var(--color-text-muted);
	}

	/* Screenshot mockup */
	.hero-visual {
		width: 100%;
		max-width: 900px;
		perspective: 1000px;
	}

	.screenshot-frame {
		background: var(--color-bg-secondary);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-xl);
		overflow: hidden;
		box-shadow: var(--shadow-medium);
		transform: rotateX(2deg);
	}

	.screenshot-header {
		display: flex;
		gap: 6px;
		padding: var(--space-3);
		background: var(--color-bg-tertiary);
		border-bottom: 1px solid var(--color-border);
	}

	.dot {
		width: 10px;
		height: 10px;
		border-radius: 50%;
	}
	.dot.red {
		background: var(--color-destructive);
	}
	.dot.yellow {
		background: var(--color-primary);
	}
	.dot.green {
		background: var(--color-success);
	}

	.screenshot-content {
		display: flex;
		height: 400px;
	}

	.mock-sidebar {
		width: 200px;
		background: var(--color-bg-tertiary);
		border-right: 1px solid var(--color-border);
		padding: var(--space-4);
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.mock-item {
		height: 36px;
		background: var(--color-bg-elevated);
		border-radius: var(--radius-md);
	}

	.mock-item.active {
		background: color-mix(in srgb, var(--color-primary) 20%, transparent);
		border: 1px solid var(--color-primary);
	}

	.mock-main {
		flex: 1;
		display: flex;
		padding: var(--space-4);
		gap: var(--space-4);
	}

	.mock-video {
		flex: 1;
		background: var(--color-bg-primary);
		border-radius: var(--radius-lg);
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--color-primary);
	}

	.mock-chat {
		width: 280px;
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		padding: var(--space-3);
		background: var(--color-bg-tertiary);
		border-radius: var(--radius-lg);
	}

	.mock-bubble {
		border-radius: var(--radius-lg);
		padding: var(--space-4);
	}

	.mock-bubble.user {
		background: var(--color-primary);
		align-self: flex-end;
		width: 70%;
		height: 40px;
	}

	.mock-bubble.assistant {
		background: var(--color-bg-elevated);
		align-self: flex-start;
		width: 85%;
		height: 60px;
	}

	.mock-bubble.short {
		height: 36px;
		width: 60%;
	}

	@media (max-width: 768px) {
		.hero-title {
			font-size: 36px;
		}

		.hero-tagline {
			font-size: var(--text-h3);
		}

		.hero-actions {
			flex-direction: column;
			width: 100%;
		}

		.btn-primary,
		.btn-secondary {
			width: 100%;
			justify-content: center;
		}

		.screenshot-content {
			height: 300px;
		}

		.mock-sidebar {
			display: none;
		}

		.mock-chat {
			width: 200px;
		}
	}
</style>
