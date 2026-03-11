<script lang="ts">
	interface Props {
		type?: 'text' | 'email' | 'password' | 'search' | 'url' | 'number';
		value?: string;
		placeholder?: string;
		disabled?: boolean;
		error?: string | null;
		label?: string;
		id?: string;
		required?: boolean;
		oninput?: (e: Event) => void;
		onkeydown?: (e: KeyboardEvent) => void;
		onblur?: (e: FocusEvent) => void;
	}

	let {
		type = 'text',
		value = $bindable(''),
		placeholder = '',
		disabled = false,
		error = null,
		label = '',
		id = crypto.randomUUID(),
		required = false,
		oninput,
		onkeydown,
		onblur
	}: Props = $props();
</script>

<div class="input-wrapper">
	{#if label}
		<label for={id} class="input-label">{label}</label>
	{/if}
	<input
		{id}
		{type}
		bind:value
		{placeholder}
		{disabled}
		{required}
		class="input"
		class:has-error={!!error}
		{oninput}
		{onkeydown}
		{onblur}
	/>
	{#if error}
		<span class="input-error">{error}</span>
	{/if}
</div>

<style>
	.input-wrapper {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}

	.input-label {
		font-size: var(--text-small);
		font-weight: 500;
		color: var(--color-silver);
	}

	.input {
		height: 40px;
		padding: 0 var(--space-3);
		font-family: var(--font-sans);
		font-size: var(--text-body);
		color: var(--color-chalk);
		background: var(--color-basalt);
		border: 1px solid var(--color-stone);
		border-radius: var(--radius-md);
		transition:
			border-color var(--duration-fast) var(--ease-out),
			box-shadow var(--duration-fast) var(--ease-out);
	}

	.input::placeholder {
		color: var(--color-ash);
	}

	.input:hover:not(:disabled) {
		border-color: var(--color-silver);
	}

	.input:focus {
		outline: none;
		border-color: var(--color-amber);
		box-shadow: var(--shadow-glow);
	}

	.input:disabled {
		opacity: 0.5;
		cursor: not-allowed;
		background: var(--color-slate);
	}

	.input.has-error {
		border-color: var(--color-ember);
	}

	.input.has-error:focus {
		box-shadow: var(--shadow-glow-error);
	}

	.input-error {
		font-size: var(--text-small);
		color: var(--color-ember);
	}
</style>
