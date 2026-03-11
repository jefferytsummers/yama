<script lang="ts">
	interface Props {
		accept?: string;
		multiple?: boolean;
		disabled?: boolean;
		ondrop?: (files: FileList) => void;
	}

	let {
		accept = 'video/*',
		multiple = true,
		disabled = false,
		ondrop
	}: Props = $props();

	let isDragOver = $state(false);
	let inputEl: HTMLInputElement;

	function handleDragOver(e: DragEvent) {
		e.preventDefault();
		if (!disabled) {
			isDragOver = true;
		}
	}

	function handleDragLeave(e: DragEvent) {
		e.preventDefault();
		isDragOver = false;
	}

	function handleDrop(e: DragEvent) {
		e.preventDefault();
		isDragOver = false;
		if (!disabled && e.dataTransfer?.files.length) {
			ondrop?.(e.dataTransfer.files);
		}
	}

	function handleClick() {
		if (!disabled) {
			inputEl?.click();
		}
	}

	function handleInputChange(e: Event) {
		const target = e.target as HTMLInputElement;
		if (target.files?.length) {
			ondrop?.(target.files);
			target.value = ''; // Reset input
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' || e.key === ' ') {
			e.preventDefault();
			handleClick();
		}
	}
</script>

<div
	class="dropzone"
	class:drag-over={isDragOver}
	class:disabled
	role="button"
	tabindex={disabled ? -1 : 0}
	ondragover={handleDragOver}
	ondragleave={handleDragLeave}
	ondrop={handleDrop}
	onclick={handleClick}
	onkeydown={handleKeydown}
>
	<input
		bind:this={inputEl}
		type="file"
		{accept}
		{multiple}
		{disabled}
		class="visually-hidden"
		onchange={handleInputChange}
	/>

	<div class="dropzone-content">
		<div class="dropzone-icon">
			<svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
				<path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4M17 8l-5-5-5 5M12 3v12" />
			</svg>
		</div>
		<p class="dropzone-text">
			{#if isDragOver}
				Drop files here
			{:else}
				<span class="dropzone-cta">Click to upload</span> or drag and drop
			{/if}
		</p>
		<p class="dropzone-hint">
			{accept === 'video/*' ? 'MP4, MOV, AVI, MKV supported' : accept}
		</p>
	</div>
</div>

<style>
	.dropzone {
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: 200px;
		padding: var(--space-8);
		background: var(--color-basalt);
		border: 2px dashed var(--color-stone);
		border-radius: var(--radius-xl);
		cursor: pointer;
		transition:
			border-color var(--duration-fast) var(--ease-out),
			background-color var(--duration-fast) var(--ease-out);
	}

	.dropzone:hover:not(.disabled) {
		border-color: var(--color-silver);
		background: var(--color-slate);
	}

	.dropzone:focus-visible {
		outline: none;
		border-color: var(--color-amber);
		box-shadow: var(--shadow-glow);
	}

	.dropzone.drag-over {
		border-color: var(--color-amber);
		background: rgba(245, 158, 11, 0.05);
	}

	.dropzone.disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.dropzone-content {
		display: flex;
		flex-direction: column;
		align-items: center;
		text-align: center;
		gap: var(--space-3);
	}

	.dropzone-icon {
		color: var(--color-silver);
		transition: color var(--duration-fast) var(--ease-out);
	}

	.drag-over .dropzone-icon {
		color: var(--color-amber);
	}

	.dropzone-text {
		font-size: var(--text-body);
		color: var(--color-silver);
	}

	.dropzone-cta {
		color: var(--color-amber);
		font-weight: 500;
	}

	.dropzone-hint {
		font-size: var(--text-small);
		color: var(--color-ash);
	}

	.visually-hidden {
		position: absolute;
		width: 1px;
		height: 1px;
		padding: 0;
		margin: -1px;
		overflow: hidden;
		clip: rect(0, 0, 0, 0);
		white-space: nowrap;
		border: 0;
	}
</style>
