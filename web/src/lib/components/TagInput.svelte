<script lang="ts">
	interface Props {
		tags?: string[];
		placeholder?: string;
		maxTags?: number;
		label?: string;
		disabled?: boolean;
	}

	let {
		tags = $bindable([]),
		placeholder = 'Add tag...',
		maxTags = 10,
		label = '',
		disabled = false
	}: Props = $props();

	let inputValue = $state('');
	let inputEl = $state<HTMLInputElement | null>(null);

	function addTag() {
		const tag = inputValue.trim().toLowerCase();
		if (tag && !tags.includes(tag) && tags.length < maxTags) {
			tags = [...tags, tag];
			inputValue = '';
		}
	}

	function removeTag(index: number) {
		tags = tags.filter((_, i) => i !== index);
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') {
			e.preventDefault();
			addTag();
		} else if (e.key === 'Backspace' && !inputValue && tags.length > 0) {
			removeTag(tags.length - 1);
		}
	}

	function handleContainerClick() {
		inputEl?.focus();
	}
</script>

<div class="tag-input-wrapper">
	{#if label}
		<!-- svelte-ignore a11y_label_has_associated_control -->
		<span class="tag-label">{label}</span>
	{/if}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="tag-input-container"
		class:disabled
		onclick={handleContainerClick}
	>
		{#each tags as tag, i}
			<span class="tag">
				{tag}
				{#if !disabled}
					<button
						type="button"
						class="tag-remove"
						onclick={() => removeTag(i)}
						aria-label="Remove tag {tag}"
					>
						<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
							<line x1="18" y1="6" x2="6" y2="18" />
							<line x1="6" y1="6" x2="18" y2="18" />
						</svg>
					</button>
				{/if}
			</span>
		{/each}
		{#if tags.length < maxTags && !disabled}
			<input
				bind:this={inputEl}
				type="text"
				bind:value={inputValue}
				{placeholder}
				class="tag-input"
				onkeydown={handleKeydown}
				onblur={addTag}
			/>
		{/if}
	</div>
	{#if maxTags}
		<span class="tag-count">{tags.length}/{maxTags} tags</span>
	{/if}
</div>

<style>
	.tag-input-wrapper {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}

	.tag-label {
		font-size: var(--text-small);
		font-weight: 500;
		color: var(--color-silver);
	}

	.tag-input-container {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
		min-height: 44px;
		background: var(--color-basalt);
		border: 1px solid var(--color-stone);
		border-radius: var(--radius-md);
		cursor: text;
		transition: border-color var(--duration-fast) var(--ease-out);
	}

	.tag-input-container:hover:not(.disabled) {
		border-color: var(--color-silver);
	}

	.tag-input-container:focus-within {
		border-color: var(--color-amber);
		box-shadow: var(--shadow-glow);
	}

	.tag-input-container.disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.tag {
		display: inline-flex;
		align-items: center;
		gap: var(--space-1);
		padding: var(--space-1) var(--space-2);
		background: var(--color-slate);
		border: 1px solid var(--color-stone);
		border-radius: var(--radius-sm);
		font-size: var(--text-small);
		color: var(--color-silver);
	}

	.tag-remove {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 0;
		background: none;
		border: none;
		color: var(--color-ash);
		cursor: pointer;
		transition: color var(--duration-fast) var(--ease-out);
	}

	.tag-remove:hover {
		color: var(--color-ember);
	}

	.tag-input {
		flex: 1;
		min-width: 100px;
		padding: var(--space-1) 0;
		font-family: var(--font-sans);
		font-size: var(--text-body);
		color: var(--color-chalk);
		background: transparent;
		border: none;
		outline: none;
	}

	.tag-input::placeholder {
		color: var(--color-ash);
	}

	.tag-count {
		font-size: var(--text-small);
		color: var(--color-ash);
	}
</style>
