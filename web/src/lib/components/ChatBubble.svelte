<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		role: 'user' | 'assistant' | 'system';
		timestamp?: Date;
		streaming?: boolean;
		children: Snippet;
	}

	let {
		role,
		timestamp,
		streaming = false,
		children
	}: Props = $props();

	const formatTime = (date: Date) => {
		return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
	};
</script>

<div class="chat-bubble chat-bubble-{role}" class:streaming>
	<div class="bubble-content">
		{@render children()}
		{#if streaming}
			<span class="cursor">|</span>
		{/if}
	</div>
	{#if timestamp}
		<time class="bubble-time">{formatTime(timestamp)}</time>
	{/if}
</div>

<style>
	.chat-bubble {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		max-width: 80%;
		animation: slideUp var(--duration-normal) var(--ease-out);
	}

	.chat-bubble-user {
		align-self: flex-end;
	}

	.chat-bubble-assistant {
		align-self: flex-start;
	}

	.chat-bubble-system {
		align-self: center;
		max-width: 90%;
	}

	.bubble-content {
		padding: var(--space-3) var(--space-4);
		border-radius: var(--radius-lg);
		font-size: var(--text-body);
		line-height: var(--leading-relaxed);
		word-wrap: break-word;
	}

	.chat-bubble-user .bubble-content {
		background: var(--color-amber);
		color: var(--color-obsidian);
		border-bottom-right-radius: var(--radius-sm);
	}

	.chat-bubble-assistant .bubble-content {
		background: var(--color-slate);
		color: var(--color-chalk);
		border: 1px solid var(--color-stone);
		border-bottom-left-radius: var(--radius-sm);
	}

	.chat-bubble-system .bubble-content {
		background: var(--color-basalt);
		color: var(--color-silver);
		font-size: var(--text-small);
		text-align: center;
		border-radius: var(--radius-full);
	}

	.bubble-time {
		font-size: var(--text-tiny);
		color: var(--color-ash);
		padding: 0 var(--space-1);
	}

	.chat-bubble-user .bubble-time {
		text-align: right;
	}

	/* Streaming cursor */
	.cursor {
		animation: blink 1s step-end infinite;
	}

	@keyframes blink {
		0%,
		50% {
			opacity: 1;
		}
		51%,
		100% {
			opacity: 0;
		}
	}

	@keyframes slideUp {
		from {
			opacity: 0;
			transform: translateY(10px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}
</style>
