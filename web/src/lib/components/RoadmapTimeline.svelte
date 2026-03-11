<script lang="ts">
	const stages = [
		{
			label: 'NOW',
			title: 'Mac Development',
			description: 'Apple Silicon optimized',
			icon: 'mac',
			status: 'active'
		},
		{
			label: 'COMING',
			title: 'Edge Deployment',
			description: 'Jetson & x86 edge devices',
			icon: 'edge',
			status: 'upcoming'
		},
		{
			label: 'FUTURE',
			title: 'Fleet Management',
			description: 'Manage devices at scale',
			icon: 'fleet',
			status: 'roadmap'
		}
	];
</script>

<div class="roadmap-timeline">
	<div class="timeline-track"></div>
	<div class="stages">
		{#each stages as stage, i}
			<div class="stage {stage.status}">
				<div class="stage-badge">{stage.label}</div>
				<div class="stage-icon">
					{#if stage.icon === 'mac'}
						<!-- MacBook icon -->
						<svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
							<rect x="2" y="3" width="20" height="14" rx="2" />
							<path d="M2 14h20" />
							<path d="M8 21h8" />
							<path d="M12 17v4" />
						</svg>
					{:else if stage.icon === 'edge'}
						<!-- Edge device / Jetson module icon -->
						<svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
							<rect x="3" y="4" width="18" height="16" rx="2" />
							<rect x="7" y="8" width="4" height="4" />
							<rect x="13" y="8" width="4" height="4" />
							<path d="M7 16h10" />
							<circle cx="5" cy="2" r="0.5" fill="currentColor" />
							<circle cx="8" cy="2" r="0.5" fill="currentColor" />
							<circle cx="11" cy="2" r="0.5" fill="currentColor" />
						</svg>
					{:else}
						<!-- Fleet / multi-device icon -->
						<svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
							<rect x="1" y="5" width="8" height="6" rx="1" />
							<rect x="8" y="13" width="8" height="6" rx="1" />
							<rect x="15" y="5" width="8" height="6" rx="1" />
							<path d="M5 11v2h2" />
							<path d="M19 11v2h-3" />
						</svg>
					{/if}
				</div>
				<h4 class="stage-title">{stage.title}</h4>
				<p class="stage-description">{stage.description}</p>
			</div>
			{#if i < stages.length - 1}
				<div class="stage-connector">
					<svg width="48" height="16" viewBox="0 0 48 16">
						<defs>
							<linearGradient id="connector-gradient-{i}" x1="0%" y1="0%" x2="100%" y2="0%">
								<stop offset="0%" stop-color={i === 0 ? 'var(--color-primary)' : 'var(--color-border)'} />
								<stop offset="100%" stop-color="var(--color-border)" />
							</linearGradient>
						</defs>
						<path
							d="M0 8h40M36 4l6 4-6 4"
							stroke="url(#connector-gradient-{i})"
							stroke-width="2"
							fill="none"
							stroke-linecap="round"
							stroke-dasharray={i > 0 ? '4 4' : 'none'}
						/>
					</svg>
				</div>
			{/if}
		{/each}
	</div>
	<div class="architecture-note">
		<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
			<path d="M14.7 6.3a1 1 0 000 1.4l1.6 1.6a1 1 0 001.4 0l3.77-3.77a6 6 0 01-7.94 7.94l-6.91 6.91a2.12 2.12 0 01-3-3l6.91-6.91a6 6 0 017.94-7.94l-3.76 3.76z" />
		</svg>
		<span>Same Rust core runs on ARM64, x86, and Apple Silicon</span>
	</div>
</div>

<style>
	.roadmap-timeline {
		position: relative;
		padding: var(--space-6) 0;
	}

	.stages {
		display: flex;
		align-items: flex-start;
		justify-content: center;
		gap: var(--space-2);
	}

	.stage {
		display: flex;
		flex-direction: column;
		align-items: center;
		text-align: center;
		min-width: 140px;
		max-width: 180px;
	}

	.stage-badge {
		padding: var(--space-1) var(--space-2);
		font-size: var(--text-tiny);
		font-weight: 700;
		letter-spacing: 0.08em;
		border-radius: var(--radius-sm);
		margin-bottom: var(--space-3);
	}

	.stage.active .stage-badge {
		background: var(--color-primary);
		color: var(--color-on-primary);
	}

	.stage.upcoming .stage-badge {
		background: var(--color-bg-tertiary);
		color: var(--color-text-secondary);
		border: 1px solid var(--color-border);
	}

	.stage.roadmap .stage-badge {
		background: transparent;
		color: var(--color-text-muted);
		border: 1px dashed var(--color-border);
	}

	.stage-icon {
		width: 72px;
		height: 72px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: var(--radius-xl);
		margin-bottom: var(--space-3);
		transition: all var(--duration-normal) var(--ease-out);
	}

	.stage.active .stage-icon {
		background: color-mix(in srgb, var(--color-primary) 15%, transparent);
		border: 2px solid var(--color-primary);
		color: var(--color-primary);
		box-shadow: var(--shadow-glow);
	}

	.stage.upcoming .stage-icon {
		background: var(--color-bg-tertiary);
		border: 2px solid var(--color-border);
		color: var(--color-text-secondary);
	}

	.stage.roadmap .stage-icon {
		background: var(--color-bg-secondary);
		border: 2px dashed var(--color-border);
		color: var(--color-text-muted);
	}

	.stage-title {
		font-size: var(--text-body);
		font-weight: 600;
		color: var(--color-text-primary);
		margin: 0 0 var(--space-1);
	}

	.stage.roadmap .stage-title {
		color: var(--color-text-secondary);
	}

	.stage-description {
		font-size: var(--text-small);
		color: var(--color-text-tertiary);
		margin: 0;
		line-height: var(--leading-relaxed);
	}

	.stage.roadmap .stage-description {
		color: var(--color-text-muted);
	}

	.stage-connector {
		display: flex;
		align-items: center;
		padding-top: 50px;
	}

	.architecture-note {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-2);
		margin-top: var(--space-8);
		padding: var(--space-3) var(--space-4);
		background: var(--color-bg-tertiary);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		font-size: var(--text-small);
		color: var(--color-text-secondary);
	}

	.architecture-note svg {
		color: var(--color-primary);
		flex-shrink: 0;
	}

	@media (max-width: 700px) {
		.stages {
			flex-direction: column;
			align-items: center;
			gap: var(--space-1);
		}

		.stage {
			max-width: none;
			width: 100%;
		}

		.stage-connector {
			transform: rotate(90deg);
			padding-top: 0;
			margin: var(--space-2) 0;
		}

		.stage-icon {
			width: 56px;
			height: 56px;
		}

		.stage-icon svg {
			width: 24px;
			height: 24px;
		}
	}
</style>
