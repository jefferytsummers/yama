<script lang="ts">
	const coreColors = [
		{ name: 'Obsidian', var: '--color-obsidian', hex: '#0D0D0F', desc: 'Primary background' },
		{ name: 'Basalt', var: '--color-basalt', hex: '#16161A', desc: 'Secondary background' },
		{ name: 'Slate', var: '--color-slate', hex: '#1E1E24', desc: 'Surface/cards' },
		{ name: 'Graphite', var: '--color-graphite', hex: '#2A2A32', desc: 'Elevated surfaces' },
		{ name: 'Stone', var: '--color-stone', hex: '#3D3D47', desc: 'Borders, dividers' }
	];

	const accentColors = [
		{ name: 'Amber', var: '--color-amber', hex: '#F59E0B', desc: 'Primary accent' },
		{ name: 'Ember', var: '--color-ember', hex: '#EF4444', desc: 'Errors' },
		{ name: 'Jade', var: '--color-jade', hex: '#10B981', desc: 'Success' },
		{ name: 'Azure', var: '--color-azure', hex: '#3B82F6', desc: 'Links, info' },
		{ name: 'Violet', var: '--color-violet', hex: '#8B5CF6', desc: 'AI/inference' }
	];

	const textColors = [
		{ name: 'Chalk', var: '--color-chalk', hex: '#FAFAFA', desc: 'Primary text' },
		{ name: 'Silver', var: '--color-silver', hex: '#A1A1AA', desc: 'Secondary text' },
		{ name: 'Ash', var: '--color-ash', hex: '#71717A', desc: 'Muted text' }
	];

	const statuses = ['running', 'stopped', 'starting', 'stopping', 'failed', 'inferring'];
</script>

<svelte:head>
	<title>Style Guide - Yama</title>
</svelte:head>

<div class="style-guide">
	<header class="hero">
		<div class="hero-content">
			<div class="logo-large">
				<svg width="48" height="48" viewBox="0 0 24 24" fill="none">
					<path d="M12 2L2 19H22L12 2Z" fill="currentColor" />
					<path d="M12 6L6 17H18L12 6Z" fill="var(--color-obsidian)" />
				</svg>
			</div>
			<h1>Yama Design System</h1>
			<p class="tagline">Vision AI Compositor &mdash; "Obsidian Lens" Theme</p>
		</div>
	</header>

	<section class="section">
		<h2>Brand Concept</h2>
		<div class="concept-grid">
			<div class="concept-card">
				<div class="concept-icon">
					<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<circle cx="12" cy="12" r="10" />
						<circle cx="12" cy="12" r="4" />
					</svg>
				</div>
				<h3>Precision</h3>
				<p>Real-time, frame-accurate video analysis</p>
			</div>
			<div class="concept-card">
				<div class="concept-icon">
					<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<path d="M12 2L2 19H22L12 2Z" />
						<path d="M12 8L7 17H17L12 8Z" />
					</svg>
				</div>
				<h3>Depth</h3>
				<p>Multi-layered understanding of visual content</p>
			</div>
			<div class="concept-card">
				<div class="concept-icon">
					<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<rect x="3" y="3" width="18" height="18" rx="2" />
						<path d="M9 12h6M12 9v6" />
					</svg>
				</div>
				<h3>Stability</h3>
				<p>Rock-solid reliability like a mountain</p>
			</div>
			<div class="concept-card">
				<div class="concept-icon">
					<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<circle cx="12" cy="12" r="10" />
						<path d="M12 16v-4M12 8h.01" />
					</svg>
				</div>
				<h3>Clarity</h3>
				<p>Clear insights emerging from visual noise</p>
			</div>
		</div>
	</section>

	<section class="section">
		<h2>Color Palette</h2>

		<h3 class="subsection-title">Core Colors</h3>
		<div class="color-grid">
			{#each coreColors as color}
				<div class="color-card">
					<div class="color-swatch" style="background-color: var({color.var})"></div>
					<div class="color-info">
						<strong>{color.name}</strong>
						<code>{color.hex}</code>
						<span class="color-desc">{color.desc}</span>
					</div>
				</div>
			{/each}
		</div>

		<h3 class="subsection-title">Accent Colors</h3>
		<div class="color-grid">
			{#each accentColors as color}
				<div class="color-card">
					<div class="color-swatch" style="background-color: var({color.var})"></div>
					<div class="color-info">
						<strong>{color.name}</strong>
						<code>{color.hex}</code>
						<span class="color-desc">{color.desc}</span>
					</div>
				</div>
			{/each}
		</div>

		<h3 class="subsection-title">Text Colors</h3>
		<div class="color-grid">
			{#each textColors as color}
				<div class="color-card">
					<div class="color-swatch text-swatch" style="background-color: var(--color-surface)">
						<span style="color: var({color.var})">Aa</span>
					</div>
					<div class="color-info">
						<strong>{color.name}</strong>
						<code>{color.hex}</code>
						<span class="color-desc">{color.desc}</span>
					</div>
				</div>
			{/each}
		</div>
	</section>

	<section class="section">
		<h2>Typography</h2>

		<div class="type-specimen">
			<div class="type-sample">
				<span class="type-label">Display</span>
				<p class="type-display">Yama Vision AI</p>
			</div>
			<div class="type-sample">
				<span class="type-label">Heading 1</span>
				<h1 class="type-h1">Video Analysis Dashboard</h1>
			</div>
			<div class="type-sample">
				<span class="type-label">Heading 2</span>
				<h2 class="type-h2">System Overview</h2>
			</div>
			<div class="type-sample">
				<span class="type-label">Heading 3</span>
				<h3 class="type-h3">Service Status</h3>
			</div>
			<div class="type-sample">
				<span class="type-label">Body</span>
				<p class="type-body">
					Real-time video analysis powered by Vision Language Models.
					Monitor, analyze, and extract insights from video streams with frame-level precision.
				</p>
			</div>
			<div class="type-sample">
				<span class="type-label">Monospace</span>
				<code class="type-mono">container_id: 8f3a2b1c4d5e</code>
			</div>
		</div>
	</section>

	<section class="section">
		<h2>Buttons</h2>

		<div class="button-grid">
			<div class="button-example">
				<span class="example-label">Primary</span>
				<button>Start Service</button>
			</div>
			<div class="button-example">
				<span class="example-label">Secondary</span>
				<button class="secondary">Stop Service</button>
			</div>
			<div class="button-example">
				<span class="example-label">Destructive</span>
				<button class="destructive">Delete</button>
			</div>
			<div class="button-example">
				<span class="example-label">Ghost</span>
				<button class="ghost">Cancel</button>
			</div>
			<div class="button-example">
				<span class="example-label">Disabled</span>
				<button disabled>Disabled</button>
			</div>
			<div class="button-example">
				<span class="example-label">Loading</span>
				<button disabled>
					<span class="spinner"></span>
					Loading...
				</button>
			</div>
		</div>
	</section>

	<section class="section">
		<h2>Status Indicators</h2>

		<div class="status-grid">
			{#each statuses as status}
				<div class="status-example">
					<div class="status-dot-wrapper status-{status}">
						<span class="status-dot" class:running={status === 'running'} class:inferring={status === 'inferring'}></span>
					</div>
					<span class="status-label status-{status}">{status}</span>
				</div>
			{/each}
		</div>
	</section>

	<section class="section">
		<h2>Cards</h2>

		<div class="card-examples">
			<div class="card">
				<div class="card-header">
					<h3 class="card-title">Service Card</h3>
				</div>
				<p>Cards use the slate surface color with stone borders and subtle shadows.</p>
				<div style="display: flex; gap: var(--space-2); margin-top: var(--space-3);">
					<button>Action</button>
					<button class="secondary">Cancel</button>
				</div>
			</div>

			<div class="card">
				<div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-3);">
					<h3 style="margin: 0;">Video Source</h3>
					<span class="badge success">Live</span>
				</div>
				<div style="background: var(--color-bg); height: 120px; border-radius: var(--radius-md); display: flex; align-items: center; justify-content: center; color: var(--color-text-muted);">
					Video Preview
				</div>
			</div>
		</div>
	</section>

	<section class="section">
		<h2>Badges</h2>

		<div class="badge-grid">
			<span class="badge">Default</span>
			<span class="badge primary">Primary</span>
			<span class="badge success">Success</span>
			<span class="badge error">Error</span>
			<span class="badge info">Info</span>
			<span class="badge inference">Inference</span>
		</div>
	</section>

	<section class="section">
		<h2>Progress</h2>

		<div class="progress-examples">
			<div class="progress-example">
				<span class="progress-label">Default (4px)</span>
				<div class="progress">
					<div class="progress-bar" style="width: 65%"></div>
				</div>
			</div>
			<div class="progress-example">
				<span class="progress-label">Large (8px)</span>
				<div class="progress lg">
					<div class="progress-bar" style="width: 45%"></div>
				</div>
			</div>
		</div>
	</section>

	<section class="section">
		<h2>Forms</h2>

		<div class="form-examples">
			<div class="form-group">
				<label for="input-example">Text Input</label>
				<input id="input-example" type="text" placeholder="Enter value..." />
			</div>
			<div class="form-group">
				<label for="select-example">Select</label>
				<select id="select-example">
					<option>Option 1</option>
					<option>Option 2</option>
					<option>Option 3</option>
				</select>
			</div>
			<div class="form-group">
				<label for="textarea-example">Textarea</label>
				<textarea id="textarea-example" rows="3" placeholder="Enter prompt..."></textarea>
			</div>
		</div>
	</section>

	<section class="section">
		<h2>Spacing Scale</h2>

		<div class="spacing-examples">
			{#each [1, 2, 3, 4, 5, 6, 8, 10] as space}
				<div class="spacing-example">
					<code>--space-{space}</code>
					<div class="spacing-bar" style="width: calc(var(--space-{space}) * 4)"></div>
					<span class="spacing-value">{space * 4}px</span>
				</div>
			{/each}
		</div>
	</section>

	<section class="section">
		<h2>Border Radius</h2>

		<div class="radius-examples">
			<div class="radius-example">
				<div class="radius-box" style="border-radius: var(--radius-sm)"></div>
				<code>sm (4px)</code>
			</div>
			<div class="radius-example">
				<div class="radius-box" style="border-radius: var(--radius-md)"></div>
				<code>md (6px)</code>
			</div>
			<div class="radius-example">
				<div class="radius-box" style="border-radius: var(--radius-lg)"></div>
				<code>lg (8px)</code>
			</div>
			<div class="radius-example">
				<div class="radius-box" style="border-radius: var(--radius-xl)"></div>
				<code>xl (12px)</code>
			</div>
			<div class="radius-example">
				<div class="radius-box" style="border-radius: var(--radius-full)"></div>
				<code>full</code>
			</div>
		</div>
	</section>
</div>

<style>
	.style-guide {
		max-width: 1000px;
		margin: 0 auto;
	}

	.hero {
		text-align: center;
		padding: var(--space-12) 0;
		margin-bottom: var(--space-8);
		border-bottom: 1px solid var(--color-border);
	}

	.hero-content {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-4);
	}

	.logo-large {
		width: 80px;
		height: 80px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: linear-gradient(135deg, var(--color-amber), var(--color-ember));
		border-radius: var(--radius-xl);
		color: var(--color-obsidian);
		box-shadow: 0 4px 24px rgba(245, 158, 11, 0.4);
	}

	.hero h1 {
		font-size: var(--text-display);
		margin: 0;
		background: linear-gradient(90deg, var(--color-chalk), var(--color-amber));
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
	}

	.tagline {
		color: var(--color-text-muted);
		font-size: var(--text-h3);
		margin: 0;
	}

	.section {
		margin-bottom: var(--space-10);
	}

	.section h2 {
		margin-bottom: var(--space-6);
		padding-bottom: var(--space-3);
		border-bottom: 1px solid var(--color-border);
	}

	.subsection-title {
		font-size: var(--text-body);
		color: var(--color-text-muted);
		text-transform: uppercase;
		letter-spacing: 0.1em;
		margin: var(--space-6) 0 var(--space-3);
	}

	/* Concept grid */
	.concept-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: var(--space-4);
	}

	.concept-card {
		padding: var(--space-4);
		background-color: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		text-align: center;
	}

	.concept-icon {
		width: 48px;
		height: 48px;
		margin: 0 auto var(--space-3);
		display: flex;
		align-items: center;
		justify-content: center;
		background-color: color-mix(in srgb, var(--color-amber) 15%, transparent);
		border-radius: var(--radius-lg);
		color: var(--color-amber);
	}

	.concept-card h3 {
		margin: 0 0 var(--space-2);
		font-size: var(--text-h3);
	}

	.concept-card p {
		margin: 0;
		color: var(--color-text-muted);
		font-size: var(--text-small);
	}

	/* Color grid */
	.color-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
		gap: var(--space-3);
	}

	.color-card {
		background-color: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		overflow: hidden;
	}

	.color-swatch {
		height: 80px;
	}

	.color-swatch.text-swatch {
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 2rem;
		font-weight: 600;
	}

	.color-info {
		padding: var(--space-3);
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}

	.color-info strong {
		font-size: var(--text-body);
	}

	.color-info code {
		font-size: var(--text-small);
		background: none;
		padding: 0;
		color: var(--color-text-muted);
	}

	.color-desc {
		font-size: var(--text-tiny);
		color: var(--color-text-muted);
	}

	/* Typography */
	.type-specimen {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}

	.type-sample {
		padding: var(--space-4);
		background-color: var(--color-surface);
		border-radius: var(--radius-lg);
		border: 1px solid var(--color-border);
	}

	.type-label {
		display: block;
		font-size: var(--text-tiny);
		color: var(--color-text-muted);
		text-transform: uppercase;
		letter-spacing: 0.1em;
		margin-bottom: var(--space-2);
	}

	.type-display {
		font-size: var(--text-display);
		font-weight: 600;
		margin: 0;
		line-height: 1.2;
	}

	.type-h1, .type-h2, .type-h3 {
		margin: 0;
	}

	.type-body {
		margin: 0;
		max-width: 60ch;
	}

	.type-mono {
		font-size: var(--text-body);
		display: block;
	}

	/* Buttons */
	.button-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
		gap: var(--space-4);
	}

	.button-example {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.example-label {
		font-size: var(--text-tiny);
		color: var(--color-text-muted);
		text-transform: uppercase;
		letter-spacing: 0.1em;
	}

	.button-example button {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-2);
	}

	/* Status */
	.status-grid {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-4);
	}

	.status-example {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
		background-color: var(--color-surface);
		border-radius: var(--radius-md);
	}

	.status-dot-wrapper {
		width: 16px;
		height: 16px;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.status-label {
		font-size: var(--text-small);
		font-weight: 500;
		text-transform: capitalize;
	}

	/* Cards */
	.card-examples {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
		gap: var(--space-4);
	}

	/* Badges */
	.badge-grid {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-3);
	}

	/* Progress */
	.progress-examples {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}

	.progress-example {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.progress-label {
		font-size: var(--text-small);
		color: var(--color-text-muted);
	}

	/* Forms */
	.form-examples {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
		gap: var(--space-4);
	}

	.form-group {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.form-group label {
		font-size: var(--text-small);
		font-weight: 500;
		color: var(--color-text-secondary);
	}

	.form-group input,
	.form-group select,
	.form-group textarea {
		width: 100%;
	}

	/* Spacing */
	.spacing-examples {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.spacing-example {
		display: flex;
		align-items: center;
		gap: var(--space-4);
	}

	.spacing-example code {
		width: 100px;
		font-size: var(--text-small);
	}

	.spacing-bar {
		height: 20px;
		background: linear-gradient(90deg, var(--color-amber), var(--color-ember));
		border-radius: var(--radius-sm);
	}

	.spacing-value {
		font-size: var(--text-small);
		color: var(--color-text-muted);
		font-family: var(--font-mono);
	}

	/* Radius */
	.radius-examples {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-4);
	}

	.radius-example {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-2);
	}

	.radius-box {
		width: 60px;
		height: 60px;
		background-color: var(--color-amber);
	}

	.radius-example code {
		font-size: var(--text-small);
	}
</style>
