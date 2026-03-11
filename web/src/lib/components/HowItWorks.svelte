<script lang="ts">
	const steps = [
		{
			number: 1,
			title: 'Import',
			description: 'Drop your videos or connect streams',
			icon: 'upload'
		},
		{
			number: 2,
			title: 'Analyze',
			description: 'AI processes every frame locally',
			icon: 'cpu'
		},
		{
			number: 3,
			title: 'Ask',
			description: 'Search and chat with your content',
			icon: 'message'
		}
	];
</script>

<section class="how-it-works">
	<h2 class="section-title">How It Works</h2>
	<p class="section-subtitle">From video to insight in three steps</p>

	<div class="steps">
		{#each steps as step, i}
			<div class="step">
				<div class="step-number">{step.number}</div>
				<div class="step-icon">
					{#if step.icon === 'upload'}
						<svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
							<path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4M17 8l-5-5-5 5M12 3v12" />
						</svg>
					{:else if step.icon === 'cpu'}
						<svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
							<rect x="4" y="4" width="16" height="16" rx="2" />
							<rect x="9" y="9" width="6" height="6" />
							<path d="M9 1v3M15 1v3M9 20v3M15 20v3M20 9h3M20 14h3M1 9h3M1 14h3" />
						</svg>
					{:else}
						<svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
							<path d="M21 15a2 2 0 01-2 2H7l-4 4V5a2 2 0 012-2h14a2 2 0 012 2z" />
						</svg>
					{/if}
				</div>
				<h3 class="step-title">{step.title}</h3>
				<p class="step-description">{step.description}</p>
			</div>
			{#if i < steps.length - 1}
				<div class="step-connector">
					<svg width="40" height="12" viewBox="0 0 40 12">
						<path
							d="M0 6h32M28 2l6 4-6 4"
							stroke="var(--color-border)"
							stroke-width="2"
							fill="none"
							stroke-linecap="round"
						/>
					</svg>
				</div>
			{/if}
		{/each}
	</div>
</section>

<style>
	.how-it-works {
		padding: var(--space-12) var(--space-6);
		text-align: center;
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

	.steps {
		display: flex;
		align-items: flex-start;
		justify-content: center;
		gap: var(--space-4);
		max-width: 900px;
		margin: 0 auto;
	}

	.step {
		display: flex;
		flex-direction: column;
		align-items: center;
		flex: 1;
		max-width: 200px;
	}

	.step-number {
		width: 32px;
		height: 32px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--color-primary);
		color: var(--color-on-primary);
		font-size: var(--text-body);
		font-weight: 700;
		border-radius: 50%;
		margin-bottom: var(--space-4);
	}

	.step-icon {
		width: 64px;
		height: 64px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--color-bg-secondary);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-xl);
		color: var(--color-text-secondary);
		margin-bottom: var(--space-4);
	}

	.step-title {
		font-size: var(--text-h3);
		font-weight: 600;
		color: var(--color-text-primary);
		margin: 0 0 var(--space-2);
	}

	.step-description {
		font-size: var(--text-small);
		color: var(--color-text-tertiary);
		margin: 0;
		line-height: var(--leading-relaxed);
	}

	.step-connector {
		display: flex;
		align-items: center;
		padding-top: 56px;
	}

	@media (max-width: 768px) {
		.steps {
			flex-direction: column;
			gap: var(--space-6);
		}

		.step {
			max-width: none;
		}

		.step-connector {
			transform: rotate(90deg);
			padding-top: 0;
		}
	}
</style>
