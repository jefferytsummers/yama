<script lang="ts">
	import { goto } from '$app/navigation';
	import {
		SplashHero,
		FeatureCard,
		HowItWorks,
		SplashFooter,
		ValueSection,
		RoadmapTimeline
	} from '$lib/components';
	import { isAuthenticated } from '$lib/stores';

	// Redirect authenticated users to dashboard
	$effect(() => {
		if ($isAuthenticated) {
			goto('/dashboard');
		}
	});

	function handleGetStarted() {
		goto('/signup');
	}

	function handleWatchDemo() {
		// TODO: Open demo modal or navigate to demo page
		console.log('Watch demo clicked');
	}

	// Pillars for the paradigm section
	const pillars = [
		{
			id: 'software',
			title: 'Battle-Tested Tooling',
			description: 'GStreamer, FFmpeg, VideoToolbox — decades of engineering in video processing. We don\'t reinvent wheels.',
			icon: 'wrench',
			examples: ['Hardware decode', 'Frame-accurate seeking', 'Format detection']
		},
		{
			id: 'models',
			title: 'Purpose-Fit Models',
			description: 'Curated model stacks tuned for your hardware. The right model for the task, not the biggest one.',
			icon: 'cpu',
			examples: ['Metal-optimized', 'Quantized for edge', 'Task-specific weights']
		},
		{
			id: 'synthesis',
			title: 'Strategic Synthesis',
			description: 'AI where it adds value, software where it\'s proven. A new paradigm of intelligent compute.',
			icon: 'merge',
			examples: ['AI for semantics', 'Software for precision', 'Combined for speed']
		}
	];
</script>

<svelte:head>
	<title>Yama - Video Intelligence for Professionals</title>
	<meta
		name="description"
		content="Frame-accurate video analysis with real-time AI inference. Local-first privacy with professional-grade tools."
	/>
</svelte:head>

<div class="splash-page">
	<SplashHero onGetStarted={handleGetStarted} onWatchDemo={handleWatchDemo} />

	<section class="features">
		<h2 class="section-title">Built for Video Professionals</h2>
		<p class="section-subtitle">Powerful tools that understand your content</p>

		<div class="feature-grid">
			<FeatureCard
				title="Frame-Accurate Analysis"
				description="Every frame matters. Get precise timestamps, scene detection, and object tracking at the frame level."
			>
				{#snippet icon()}
					<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<rect x="2" y="2" width="20" height="20" rx="2" />
						<line x1="7" y1="2" x2="7" y2="22" />
						<line x1="17" y1="2" x2="17" y2="22" />
						<line x1="2" y1="12" x2="22" y2="12" />
					</svg>
				{/snippet}
			</FeatureCard>

			<FeatureCard
				title="Local-First Privacy"
				description="Your videos never leave your machine. All processing happens locally using on-device AI."
			>
				{#snippet icon()}
					<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<rect x="3" y="11" width="18" height="11" rx="2" />
						<path d="M7 11V7a5 5 0 0110 0v4" />
						<circle cx="12" cy="16" r="1" fill="currentColor" />
					</svg>
				{/snippet}
			</FeatureCard>

			<FeatureCard
				title="Real-Time Inference"
				description="Stream analysis as it happens. Watch AI insights appear frame-by-frame during playback."
			>
				{#snippet icon()}
					<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z" />
					</svg>
				{/snippet}
			</FeatureCard>

			<FeatureCard
				title="Conversational Search"
				description="Ask questions in natural language. Find moments, compare scenes, generate reports instantly."
			>
				{#snippet icon()}
					<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<path d="M21 15a2 2 0 01-2 2H7l-4 4V5a2 2 0 012-2h14a2 2 0 012 2z" />
						<circle cx="9" cy="10" r="1" fill="currentColor" />
						<circle cx="12" cy="10" r="1" fill="currentColor" />
						<circle cx="15" cy="10" r="1" fill="currentColor" />
					</svg>
				{/snippet}
			</FeatureCard>

			<FeatureCard
				title="Multi-Model Support"
				description="Choose from leading vision models. Switch between local and cloud inference based on your needs."
			>
				{#snippet icon()}
					<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<circle cx="12" cy="12" r="3" />
						<path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" />
					</svg>
				{/snippet}
			</FeatureCard>

			<FeatureCard
				title="Agent-Powered Tools"
				description="Automated workflows for clip extraction, transcription, visual search, and report generation."
			>
				{#snippet icon()}
					<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<path d="M14.7 6.3a1 1 0 000 1.4l1.6 1.6a1 1 0 001.4 0l3.77-3.77a6 6 0 01-7.94 7.94l-6.91 6.91a2.12 2.12 0 01-3-3l6.91-6.91a6 6 0 017.94-7.94l-3.76 3.76z" />
					</svg>
				{/snippet}
			</FeatureCard>
		</div>
	</section>

	<!-- Section 1: Your Models, Your Hardware -->
	<ValueSection
		title="Your Models, Your Hardware"
		subtitle="Local-first AI that respects your privacy"
		variant="default"
		layout="split"
	>
		{#snippet visual()}
			<div class="hardware-visual">
				<div class="device-stack">
					<div class="device mac">
						<svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
							<rect x="2" y="3" width="20" height="14" rx="2" />
							<path d="M2 14h20" />
							<path d="M8 21h8" />
							<path d="M12 17v4" />
						</svg>
						<span class="device-label">Mac + Metal</span>
					</div>
					<div class="device-connector"></div>
					<div class="device edge">
						<svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
							<rect x="3" y="4" width="18" height="16" rx="2" />
							<rect x="7" y="8" width="4" height="4" />
							<rect x="13" y="8" width="4" height="4" />
							<path d="M7 16h10" />
						</svg>
						<span class="device-label">Edge + CUDA</span>
					</div>
				</div>
				<div class="model-flow">
					<div class="model-icon">
						<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
							<circle cx="12" cy="12" r="3" />
							<path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" />
						</svg>
					</div>
					<div class="flow-arrow"></div>
					<div class="model-icon">
						<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
							<path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4M7 10l5 5 5-5M12 15V3" />
						</svg>
					</div>
				</div>
			</div>
		{/snippet}

		<ul class="feature-list">
			<li>
				<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<polyline points="20 6 9 17 4 12" />
				</svg>
				<span>Run entirely on-device — Mac with Metal, edge with CUDA</span>
			</li>
			<li>
				<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<polyline points="20 6 9 17 4 12" />
				</svg>
				<span>No cloud dependency, no data leaves your machine</span>
			</li>
			<li>
				<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<polyline points="20 6 9 17 4 12" />
				</svg>
				<span>Optimized stacks from 500MB to 5GB+ model bundles</span>
			</li>
			<li>
				<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<polyline points="20 6 9 17 4 12" />
				</svg>
				<span>Hardware-accelerated decode: VideoToolbox, NVDEC</span>
			</li>
		</ul>
	</ValueSection>

	<HowItWorks />

	<!-- Section 2: The Paradigm -->
	<ValueSection
		title="Engineered Intelligence"
		subtitle="The right tool for every layer of the stack"
		variant="dark"
		layout="centered"
	>
		<div class="paradigm-visual">
			<div class="paradigm-equation">
				<div class="equation-term software">
					<span class="term-label">Proven Software</span>
				</div>
				<span class="equation-operator">+</span>
				<div class="equation-term ai">
					<span class="term-label">Purpose-Fit AI</span>
				</div>
				<span class="equation-operator">=</span>
				<div class="equation-term result">
					<span class="term-label">Intelligent Systems</span>
				</div>
			</div>
		</div>

		<div class="pillars-grid">
			{#each pillars as pillar}
				<div class="pillar-card">
					<div class="pillar-icon">
						{#if pillar.icon === 'wrench'}
							<svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
								<path d="M14.7 6.3a1 1 0 000 1.4l1.6 1.6a1 1 0 001.4 0l3.77-3.77a6 6 0 01-7.94 7.94l-6.91 6.91a2.12 2.12 0 01-3-3l6.91-6.91a6 6 0 017.94-7.94l-3.76 3.76z" />
							</svg>
						{:else if pillar.icon === 'cpu'}
							<svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
								<rect x="4" y="4" width="16" height="16" rx="2" />
								<rect x="9" y="9" width="6" height="6" />
								<path d="M9 1v3M15 1v3M9 20v3M15 20v3M20 9h3M20 14h3M1 9h3M1 14h3" />
							</svg>
						{:else}
							<svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
								<path d="M6 3v12" />
								<path d="M18 9v12" />
								<circle cx="6" cy="18" r="3" />
								<circle cx="18" cy="6" r="3" />
								<path d="M6 15a6 6 0 006-6h0a6 6 0 016 6" />
							</svg>
						{/if}
					</div>
					<h4 class="pillar-title">{pillar.title}</h4>
					<p class="pillar-description">{pillar.description}</p>
					<ul class="pillar-examples">
						{#each pillar.examples as example}
							<li>{example}</li>
						{/each}
					</ul>
				</div>
			{/each}
		</div>
	</ValueSection>

	<!-- Section 3: Built for the Edge -->
	<ValueSection
		title="Built for the Edge"
		subtitle="From laptop prototype to production fleet"
		variant="gradient"
		layout="centered"
	>
		<RoadmapTimeline />
	</ValueSection>

	<section class="cta-section">
		<div class="cta-content">
			<h2 class="cta-title">Ready to analyze your videos?</h2>
			<p class="cta-description">
				Get started in minutes. No credit card required for the free tier.
			</p>
			<button class="btn-primary large" onclick={handleGetStarted}>
				<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<path d="M12 3l1.912 5.813a2 2 0 001.275 1.275L21 12l-5.813 1.912a2 2 0 00-1.275 1.275L12 21l-1.912-5.813a2 2 0 00-1.275-1.275L3 12l5.813-1.912a2 2 0 001.275-1.275L12 3z" />
				</svg>
				Get Started Free
			</button>
		</div>
	</section>

	<SplashFooter />
</div>

<style>
	.splash-page {
		display: flex;
		flex-direction: column;
		min-height: 100vh;
		background: var(--color-bg-primary);
	}

	/* Features Section */
	.features {
		padding: var(--space-12) var(--space-6);
		text-align: center;
		background: var(--color-bg-secondary);
	}

	.section-title {
		font-size: var(--text-h1);
		font-weight: 600;
		color: var(--color-text-primary);
		margin: 0 0 var(--space-2);
	}

	.section-subtitle {
		font-size: var(--text-body);
		color: var(--color-text-secondary);
		margin: 0 0 var(--space-10);
	}

	.feature-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
		gap: var(--space-6);
		max-width: 1200px;
		margin: 0 auto;
	}

	/* CTA Section */
	.cta-section {
		padding: var(--space-12) var(--space-6);
		background: var(--gradient-depth);
	}

	.cta-content {
		max-width: 600px;
		margin: 0 auto;
		text-align: center;
	}

	.cta-title {
		font-size: var(--text-h1);
		font-weight: 600;
		color: var(--color-text-primary);
		margin: 0 0 var(--space-3);
	}

	.cta-description {
		font-size: var(--text-body);
		color: var(--color-text-secondary);
		margin: 0 0 var(--space-8);
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
		transform: translateY(-2px);
	}

	.btn-primary.large {
		padding: var(--space-4) var(--space-8);
		font-size: var(--text-h3);
	}

	@media (max-width: 768px) {
		.features {
			padding: var(--space-10) var(--space-4);
		}

		.section-title {
			font-size: var(--text-h2);
		}

		.cta-section {
			padding: var(--space-10) var(--space-4);
		}

		.cta-title {
			font-size: var(--text-h2);
		}
	}

	/* Section 1: Your Models, Your Hardware */
	.feature-list {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}

	.feature-list li {
		display: flex;
		align-items: flex-start;
		gap: var(--space-3);
		font-size: var(--text-body);
		color: var(--color-text-secondary);
		line-height: var(--leading-relaxed);
	}

	.feature-list li svg {
		flex-shrink: 0;
		color: var(--color-success);
		margin-top: 2px;
	}

	.hardware-visual {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-6);
		padding: var(--space-8);
		background: var(--color-bg-tertiary);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-xl);
	}

	.device-stack {
		display: flex;
		align-items: center;
		gap: var(--space-6);
	}

	.device {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-4);
		background: var(--color-bg-secondary);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		transition: all var(--duration-normal) var(--ease-out);
	}

	.device.mac {
		color: var(--color-primary);
		border-color: color-mix(in srgb, var(--color-primary) 30%, transparent);
		box-shadow: 0 0 20px color-mix(in srgb, var(--color-primary) 10%, transparent);
	}

	.device.edge {
		color: var(--color-success);
		border-color: color-mix(in srgb, var(--color-success) 30%, transparent);
		box-shadow: 0 0 20px color-mix(in srgb, var(--color-success) 10%, transparent);
	}

	.device-label {
		font-size: var(--text-small);
		font-weight: 600;
		color: var(--color-text-primary);
	}

	.device-connector {
		width: 40px;
		height: 2px;
		background: linear-gradient(90deg, var(--color-primary), var(--color-success));
		border-radius: var(--radius-full);
	}

	.model-flow {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-3) var(--space-4);
		background: var(--color-bg-secondary);
		border-radius: var(--radius-lg);
	}

	.model-icon {
		width: 40px;
		height: 40px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--color-bg-elevated);
		border-radius: var(--radius-md);
		color: var(--color-inference);
	}

	.flow-arrow {
		width: 24px;
		height: 2px;
		background: var(--color-border);
		position: relative;
	}

	.flow-arrow::after {
		content: '';
		position: absolute;
		right: 0;
		top: 50%;
		transform: translateY(-50%);
		border: 4px solid transparent;
		border-left-color: var(--color-border);
	}

	/* Section 2: Paradigm / Pillars */
	.paradigm-visual {
		margin-bottom: var(--space-10);
	}

	.paradigm-equation {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-4);
		flex-wrap: wrap;
	}

	.equation-term {
		padding: var(--space-3) var(--space-5);
		border-radius: var(--radius-lg);
		font-weight: 600;
	}

	.equation-term.software {
		background: color-mix(in srgb, var(--color-info) 15%, transparent);
		border: 1px solid color-mix(in srgb, var(--color-info) 30%, transparent);
		color: var(--color-info);
	}

	.equation-term.ai {
		background: color-mix(in srgb, var(--color-inference) 15%, transparent);
		border: 1px solid color-mix(in srgb, var(--color-inference) 30%, transparent);
		color: var(--color-inference);
	}

	.equation-term.result {
		background: color-mix(in srgb, var(--color-primary) 15%, transparent);
		border: 1px solid color-mix(in srgb, var(--color-primary) 30%, transparent);
		color: var(--color-primary);
	}

	.term-label {
		font-size: var(--text-body);
	}

	.equation-operator {
		font-size: var(--text-h2);
		font-weight: 300;
		color: var(--color-border);
	}

	.pillars-grid {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: var(--space-6);
		max-width: 1000px;
		margin: 0 auto;
	}

	.pillar-card {
		padding: var(--space-6);
		background: var(--color-bg-tertiary);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-xl);
		text-align: left;
		transition: all var(--duration-normal) var(--ease-out);
	}

	.pillar-card:hover {
		background: var(--color-bg-hover);
		transform: translateY(-2px);
	}

	.pillar-card:nth-child(1) .pillar-icon {
		color: var(--color-info);
		background: color-mix(in srgb, var(--color-info) 10%, transparent);
		border-color: color-mix(in srgb, var(--color-info) 20%, transparent);
	}

	.pillar-card:nth-child(2) .pillar-icon {
		color: var(--color-inference);
		background: color-mix(in srgb, var(--color-inference) 10%, transparent);
		border-color: color-mix(in srgb, var(--color-inference) 20%, transparent);
	}

	.pillar-card:nth-child(3) .pillar-icon {
		color: var(--color-primary);
		background: color-mix(in srgb, var(--color-primary) 10%, transparent);
		border-color: color-mix(in srgb, var(--color-primary) 20%, transparent);
	}

	.pillar-icon {
		width: 56px;
		height: 56px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: var(--radius-lg);
		border: 1px solid;
		margin-bottom: var(--space-4);
	}

	.pillar-title {
		font-size: var(--text-h3);
		font-weight: 600;
		color: var(--color-text-primary);
		margin: 0 0 var(--space-2);
	}

	.pillar-description {
		font-size: var(--text-small);
		color: var(--color-text-secondary);
		margin: 0 0 var(--space-4);
		line-height: var(--leading-relaxed);
	}

	.pillar-examples {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
	}

	.pillar-examples li {
		padding: var(--space-1) var(--space-2);
		background: var(--color-bg-secondary);
		border-radius: var(--radius-sm);
		font-size: var(--text-tiny);
		color: var(--color-text-tertiary);
	}

	@media (max-width: 900px) {
		.pillars-grid {
			grid-template-columns: 1fr;
			gap: var(--space-4);
		}

		.paradigm-equation {
			gap: var(--space-2);
		}

		.equation-term {
			padding: var(--space-2) var(--space-3);
		}

		.term-label {
			font-size: var(--text-small);
		}

		.equation-operator {
			font-size: var(--text-h3);
		}
	}

	@media (max-width: 768px) {
		.device-stack {
			flex-direction: column;
		}

		.device-connector {
			width: 2px;
			height: 24px;
			background: linear-gradient(180deg, var(--color-primary), var(--color-success));
		}

		.paradigm-equation {
			flex-direction: column;
			gap: var(--space-3);
		}

		.equation-operator {
			transform: rotate(90deg);
		}
	}
</style>
