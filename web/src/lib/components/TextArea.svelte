<script lang="ts">
	interface Props {
		value?: string;
		placeholder?: string;
		disabled?: boolean;
		error?: string | null;
		label?: string;
		id?: string;
		rows?: number;
		maxLength?: number;
		showCount?: boolean;
		oninput?: (e: Event) => void;
		onblur?: (e: FocusEvent) => void;
	}

	let {
		value = $bindable(''),
		placeholder = '',
		disabled = false,
		error = null,
		label = '',
		id = crypto.randomUUID(),
		rows = 4,
		maxLength,
		showCount = false,
		oninput,
		onblur
	}: Props = $props();

	let charCount = $derived(value.length);
</script>

<div class="textarea-wrapper">
	{#if label}
		<label for={id} class="textarea-label">{label}</label>
	{/if}
	<textarea
		{id}
		bind:value
		{placeholder}
		{disabled}
		{rows}
		maxlength={maxLength}
		class="textarea"
		class:has-error={!!error}
		{oninput}
		{onblur}
	></textarea>
	<div class="textarea-footer">
		{#if error}
			<span class="textarea-error">{error}</span>
		{/if}
		{#if showCount && maxLength}
			<span class="char-count" class:near-limit={charCount > maxLength * 0.8}>
				{charCount}/{maxLength}
			</span>
		{/if}
	</div>
</div>

<style>
	.textarea-wrapper {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}

	.textarea-label {
		font-size: var(--text-small);
		font-weight: 500;
		color: var(--color-silver);
	}

	.textarea {
		padding: var(--space-3);
		font-family: var(--font-sans);
		font-size: var(--text-body);
		color: var(--color-chalk);
		background: var(--color-basalt);
		border: 1px solid var(--color-stone);
		border-radius: var(--radius-md);
		resize: vertical;
		min-height: 80px;
		transition:
			border-color var(--duration-fast) var(--ease-out),
			box-shadow var(--duration-fast) var(--ease-out);
	}

	.textarea::placeholder {
		color: var(--color-ash);
	}

	.textarea:hover:not(:disabled) {
		border-color: var(--color-silver);
	}

	.textarea:focus {
		outline: none;
		border-color: var(--color-amber);
		box-shadow: var(--shadow-glow);
	}

	.textarea:disabled {
		opacity: 0.5;
		cursor: not-allowed;
		background: var(--color-slate);
	}

	.textarea.has-error {
		border-color: var(--color-ember);
	}

	.textarea.has-error:focus {
		box-shadow: var(--shadow-glow-error);
	}

	.textarea-footer {
		display: flex;
		justify-content: space-between;
		align-items: center;
		min-height: 20px;
	}

	.textarea-error {
		font-size: var(--text-small);
		color: var(--color-ember);
	}

	.char-count {
		font-size: var(--text-small);
		color: var(--color-ash);
		margin-left: auto;
	}

	.char-count.near-limit {
		color: var(--color-amber);
	}
</style>
