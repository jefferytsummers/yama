<script lang="ts">
	import type { WizardStep } from '$lib/types';

	interface Props {
		steps: WizardStep[];
		currentStep: number;
	}

	let { steps, currentStep }: Props = $props();
</script>

<div class="step-indicator">
	{#each steps as step, i}
		<div
			class="step"
			class:completed={i < currentStep}
			class:active={i === currentStep}
			class:upcoming={i > currentStep}
		>
			<div class="step-number">
				{#if i < currentStep}
					<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
						<polyline points="20 6 9 17 4 12" />
					</svg>
				{:else}
					{i + 1}
				{/if}
			</div>
			<span class="step-label">{step.label}</span>
		</div>
		{#if i < steps.length - 1}
			<div class="step-connector" class:completed={i < currentStep}></div>
		{/if}
	{/each}
</div>

<style>
	.step-indicator {
		display: flex;
		align-items: center;
		gap: var(--space-1);
	}

	.step {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.step-number {
		width: 28px;
		height: 28px;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: var(--text-small);
		font-weight: 600;
		border-radius: 50%;
		transition: all var(--duration-normal) var(--ease-out);
	}

	.step.upcoming .step-number {
		background: var(--color-slate);
		color: var(--color-ash);
		border: 1px solid var(--color-stone);
	}

	.step.active .step-number {
		background: var(--color-amber);
		color: var(--color-obsidian);
	}

	.step.completed .step-number {
		background: var(--color-jade);
		color: var(--color-chalk);
	}

	.step-label {
		font-size: var(--text-small);
		font-weight: 500;
		transition: color var(--duration-fast) var(--ease-out);
	}

	.step.upcoming .step-label {
		color: var(--color-ash);
	}

	.step.active .step-label {
		color: var(--color-chalk);
	}

	.step.completed .step-label {
		color: var(--color-silver);
	}

	.step-connector {
		flex: 1;
		height: 2px;
		min-width: 20px;
		max-width: 60px;
		background: var(--color-stone);
		transition: background var(--duration-normal) var(--ease-out);
	}

	.step-connector.completed {
		background: var(--color-jade);
	}

	@media (max-width: 640px) {
		.step-label {
			display: none;
		}

		.step-connector {
			min-width: 12px;
		}
	}
</style>
