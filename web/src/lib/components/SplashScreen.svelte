<script lang="ts">
	import { onMount } from 'svelte';

	let { onReady }: { onReady?: () => void } = $props();

	let progress = $state(0);
	let isComplete = $state(false);
	let isGlowing = $state(false);
	let isHiding = $state(false);

	// Easing function for smoother fill
	function easeOutCubic(t: number): number {
		return 1 - Math.pow(1 - t, 3);
	}

	onMount(() => {

		// Animate progress from 0 to 100 with easing
		const startTime = performance.now();
		const fillDuration = 1200; // Longer fill animation
		const glowDuration = 800; // Time to show glow effect
		const fadeOutDuration = 400; // Fade out time

		function animate(currentTime: number) {
			const elapsed = currentTime - startTime;
			const t = Math.min(elapsed / fillDuration, 1);
			progress = easeOutCubic(t) * 100;

			if (progress < 100) {
				requestAnimationFrame(animate);
			} else {
				// Fill complete - start glow phase
				isComplete = true;
				isGlowing = true;

				// Hold on glow effect
				setTimeout(() => {
					// Start fade out
					isHiding = true;

					// Wait for fade to complete before calling ready
					setTimeout(() => {
						onReady?.();
					}, fadeOutDuration);
				}, glowDuration);
			}
		}

		requestAnimationFrame(animate);
	});

	// Calculate clip path for fill effect (bottom to top)
	let clipPath = $derived(`inset(${100 - progress}% 0 0 0)`);
</script>

<div class="splash-screen" class:hiding={isHiding}>
	<div class="splash-content">
		<div class="logo-container">
			<!-- Outlined triangle (always visible) -->
			<svg
				class="logo-outline"
				width="120"
				height="120"
				viewBox="0 0 64 64"
				fill="none"
				xmlns="http://www.w3.org/2000/svg"
			>
				<path
					d="M32 8L8 56h48L32 8z"
					stroke="var(--color-amber)"
					stroke-width="2"
					fill="none"
				/>
			</svg>

			<!-- Filled triangle (clips based on progress) -->
			<svg
				class="logo-fill"
				width="120"
				height="120"
				viewBox="0 0 64 64"
				fill="none"
				xmlns="http://www.w3.org/2000/svg"
				style="clip-path: {clipPath}"
			>
				<defs>
					<linearGradient id="fillGradient" x1="0%" y1="100%" x2="0%" y2="0%">
						<stop offset="0%" stop-color="var(--color-ember)" />
						<stop offset="100%" stop-color="var(--color-amber)" />
					</linearGradient>
				</defs>
				<path
					d="M32 8L8 56h48L32 8z"
					fill="url(#fillGradient)"
				/>
			</svg>

			<!-- Inner glow effect when complete -->
			{#if isGlowing}
				<div class="logo-glow"></div>
			{/if}
		</div>

		<div class="brand-name" class:visible={progress > 50}>
			YAMA
		</div>
	</div>
</div>

<style>
	.splash-screen {
		position: fixed;
		inset: 0;
		z-index: 9999;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--color-obsidian);
		transition: opacity 0.4s ease-out, transform 0.4s ease-out;
	}

	.splash-screen.hiding {
		opacity: 0;
		transform: scale(1.05);
		pointer-events: none;
	}

	.splash-content {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-6);
	}

	.logo-container {
		position: relative;
		width: 120px;
		height: 120px;
	}

	.logo-outline,
	.logo-fill {
		position: absolute;
		top: 0;
		left: 0;
	}

	.logo-outline {
		opacity: 0.4;
	}

	.logo-fill {
		transition: clip-path 0.05s linear;
	}

	.logo-glow {
		position: absolute;
		inset: -30px;
		background: radial-gradient(
			circle at center,
			color-mix(in srgb, var(--color-amber) 30%, transparent) 0%,
			color-mix(in srgb, var(--color-ember) 15%, transparent) 40%,
			transparent 70%
		);
		animation: glow-pulse 0.6s ease-out forwards, glow-breathe 1.5s ease-in-out 0.6s infinite;
	}

	@keyframes glow-pulse {
		0% {
			opacity: 0;
			transform: scale(0.8);
		}
		100% {
			opacity: 1;
			transform: scale(1);
		}
	}

	@keyframes glow-breathe {
		0%, 100% {
			opacity: 0.8;
			transform: scale(1);
		}
		50% {
			opacity: 1;
			transform: scale(1.1);
		}
	}

	.brand-name {
		font-family: var(--font-sans);
		font-size: 1.5rem;
		font-weight: 700;
		letter-spacing: 0.2em;
		color: var(--color-amber);
		opacity: 0;
		transform: translateY(10px);
		transition: all 0.5s ease-out;
	}

	.brand-name.visible {
		opacity: 1;
		transform: translateY(0);
	}
</style>
