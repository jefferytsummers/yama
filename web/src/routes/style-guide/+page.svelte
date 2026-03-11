<script lang="ts">
	import {
		Button,
		Card,
		Input,
		Badge,
		Progress,
		Skeleton,
		Modal,
		Tabs,
		Tooltip,
		Icon,
		DropZone,
		ChatBubble,
		ToolCallCard,
		ThemeToggle
	} from '$lib/components';

	// Demo state
	let inputValue = $state('');
	let inputWithError = $state('');
	let progressValue = $state(66);
	let modalOpen = $state(false);
	let activeTab = $state('overview');

	const tabs = [
		{ id: 'overview', label: 'Overview' },
		{ id: 'components', label: 'Components' },
		{ id: 'patterns', label: 'Patterns' }
	];

	// Color palette for display
	const coreColors = [
		{ name: 'Obsidian', var: '--color-obsidian', hex: '#0D0D0F', desc: 'Primary background' },
		{ name: 'Basalt', var: '--color-basalt', hex: '#16161A', desc: 'Secondary background' },
		{ name: 'Slate', var: '--color-slate', hex: '#1E1E24', desc: 'Surface/cards' },
		{ name: 'Graphite', var: '--color-graphite', hex: '#2A2A32', desc: 'Elevated surfaces' },
		{ name: 'Stone', var: '--color-stone', hex: '#3D3D47', desc: 'Borders, dividers' }
	];

	const accentColors = [
		{ name: 'Amber', var: '--color-amber', hex: '#F59E0B', desc: 'Primary accent, CTAs' },
		{ name: 'Ember', var: '--color-ember', hex: '#EF4444', desc: 'Errors, destructive' },
		{ name: 'Jade', var: '--color-jade', hex: '#10B981', desc: 'Success, running' },
		{ name: 'Azure', var: '--color-azure', hex: '#3B82F6', desc: 'Info, links' },
		{ name: 'Violet', var: '--color-violet', hex: '#8B5CF6', desc: 'AI/inference' }
	];

	const textColors = [
		{ name: 'Chalk', var: '--color-chalk', hex: '#FAFAFA', desc: 'Primary text' },
		{ name: 'Silver', var: '--color-silver', hex: '#A1A1AA', desc: 'Secondary text' },
		{ name: 'Pewter', var: '--color-pewter', hex: '#8E8E99', desc: 'Tertiary text' },
		{ name: 'Ash', var: '--color-ash', hex: '#71717A', desc: 'Decorative only' }
	];

	const spacingScale = [
		{ token: '1', value: '4px' },
		{ token: '2', value: '8px' },
		{ token: '3', value: '12px' },
		{ token: '4', value: '16px' },
		{ token: '5', value: '20px' },
		{ token: '6', value: '24px' },
		{ token: '8', value: '32px' },
		{ token: '10', value: '40px' },
		{ token: '12', value: '48px' }
	];

	function handleFileDrop(files: FileList) {
		console.log('Files dropped:', files);
	}
</script>

<svelte:head>
	<title>Style Guide | Yama</title>
</svelte:head>

<div class="style-guide">
	<header class="header">
		<a href="/" class="back-link">
			<Icon name="arrowLeft" size={20} />
			<span>Back</span>
		</a>
		<div class="header-content">
			<h1>Yama Style Guide</h1>
			<p class="header-subtitle">Obsidian Lens Design System</p>
		</div>
		<div class="header-actions">
			<ThemeToggle size="md" />
		</div>
	</header>

	<nav class="sidebar">
		<ul class="nav-list">
			<li><a href="#colors" class="nav-link">Colors</a></li>
			<li><a href="#typography" class="nav-link">Typography</a></li>
			<li><a href="#spacing" class="nav-link">Spacing</a></li>
			<li><a href="#buttons" class="nav-link">Buttons</a></li>
			<li><a href="#cards" class="nav-link">Cards</a></li>
			<li><a href="#inputs" class="nav-link">Inputs</a></li>
			<li><a href="#badges" class="nav-link">Badges</a></li>
			<li><a href="#progress" class="nav-link">Progress</a></li>
			<li><a href="#modals" class="nav-link">Modals</a></li>
			<li><a href="#tabs" class="nav-link">Tabs</a></li>
			<li><a href="#tooltips" class="nav-link">Tooltips</a></li>
			<li><a href="#dropzone" class="nav-link">Drop Zone</a></li>
			<li><a href="#chat" class="nav-link">Chat</a></li>
			<li><a href="#tools" class="nav-link">Tool Calls</a></li>
			<li><a href="#skeleton" class="nav-link">Skeleton</a></li>
			<li><a href="#icons" class="nav-link">Icons</a></li>
		</ul>
	</nav>

	<main class="content">
		<!-- Colors Section -->
		<section id="colors" class="section">
			<h2 class="section-title">Color Palette</h2>
			<p class="section-desc">
				The Obsidian Lens palette creates depth through carefully graduated dark tones,
				with volcanic amber as the primary accent.
			</p>

			<h3 class="subsection-title">Core Backgrounds</h3>
			<div class="color-grid">
				{#each coreColors as color}
					<div class="color-swatch">
						<div class="swatch" style="background: var({color.var})"></div>
						<div class="swatch-info">
							<span class="swatch-name">{color.name}</span>
							<code class="swatch-hex">{color.hex}</code>
							<span class="swatch-desc">{color.desc}</span>
						</div>
					</div>
				{/each}
			</div>

			<h3 class="subsection-title">Accent Colors</h3>
			<div class="color-grid">
				{#each accentColors as color}
					<div class="color-swatch">
						<div class="swatch" style="background: var({color.var})"></div>
						<div class="swatch-info">
							<span class="swatch-name">{color.name}</span>
							<code class="swatch-hex">{color.hex}</code>
							<span class="swatch-desc">{color.desc}</span>
						</div>
					</div>
				{/each}
			</div>

			<h3 class="subsection-title">Text Colors</h3>
			<div class="color-grid">
				{#each textColors as color}
					<div class="color-swatch">
						<div class="swatch swatch-text" style="background: var(--color-slate)">
							<span style="color: var({color.var})">{color.name}</span>
						</div>
						<div class="swatch-info">
							<span class="swatch-name">{color.name}</span>
							<code class="swatch-hex">{color.hex}</code>
							<span class="swatch-desc">{color.desc}</span>
						</div>
					</div>
				{/each}
			</div>
		</section>

		<!-- Typography Section -->
		<section id="typography" class="section">
			<h2 class="section-title">Typography</h2>
			<p class="section-desc">
				Geist font family with SF Pro fallback. Monospace for code and data.
			</p>

			<div class="type-scale">
				<div class="type-sample">
					<span class="type-label">Display (32px)</span>
					<span style="font-size: var(--text-display); font-weight: 600">Video Analysis Complete</span>
				</div>
				<div class="type-sample">
					<span class="type-label">H1 (24px)</span>
					<h1 style="margin: 0">Page Title</h1>
				</div>
				<div class="type-sample">
					<span class="type-label">H2 (20px)</span>
					<h2 style="margin: 0">Section Header</h2>
				</div>
				<div class="type-sample">
					<span class="type-label">H3 (16px)</span>
					<h3 style="margin: 0">Card Title</h3>
				</div>
				<div class="type-sample">
					<span class="type-label">Body (14px)</span>
					<p style="margin: 0; color: var(--color-chalk)">Default body text for content and descriptions.</p>
				</div>
				<div class="type-sample">
					<span class="type-label">Small (12px)</span>
					<span style="font-size: var(--text-small)">Labels and captions</span>
				</div>
				<div class="type-sample">
					<span class="type-label">Mono</span>
					<code style="background: none; padding: 0">frame_00042.jpg • 1920×1080 • 00:01:24</code>
				</div>
			</div>
		</section>

		<!-- Spacing Section -->
		<section id="spacing" class="section">
			<h2 class="section-title">Spacing Scale</h2>
			<p class="section-desc">4px base unit for consistent rhythm.</p>

			<div class="spacing-scale">
				{#each spacingScale as space}
					<div class="spacing-item">
						<div class="spacing-box" style="width: var(--space-{space.token}); height: var(--space-{space.token})"></div>
						<code>--space-{space.token}</code>
						<span class="spacing-value">{space.value}</span>
					</div>
				{/each}
			</div>
		</section>

		<!-- Buttons Section -->
		<section id="buttons" class="section">
			<h2 class="section-title">Buttons</h2>
			<p class="section-desc">Action triggers with clear visual hierarchy.</p>

			<div class="component-row">
				<div class="component-group">
					<h3 class="subsection-title">Variants</h3>
					<div class="button-row">
						<Button variant="primary">Primary</Button>
						<Button variant="secondary">Secondary</Button>
						<Button variant="ghost">Ghost</Button>
						<Button variant="destructive">Destructive</Button>
					</div>
				</div>

				<div class="component-group">
					<h3 class="subsection-title">Sizes</h3>
					<div class="button-row">
						<Button size="sm">Small</Button>
						<Button size="md">Medium</Button>
						<Button size="lg">Large</Button>
					</div>
				</div>

				<div class="component-group">
					<h3 class="subsection-title">States</h3>
					<div class="button-row">
						<Button disabled>Disabled</Button>
						<Button loading>Loading</Button>
					</div>
				</div>
			</div>
		</section>

		<!-- Cards Section -->
		<section id="cards" class="section">
			<h2 class="section-title">Cards</h2>
			<p class="section-desc">Surface containers for grouping content.</p>

			<div class="card-grid">
				<Card>
					<h3>Default Card</h3>
					<p>Standard surface with border.</p>
				</Card>
				<Card elevated>
					<h3>Elevated Card</h3>
					<p>Raised surface with shadow.</p>
				</Card>
				<Card interactive onclick={() => console.log('clicked')}>
					<h3>Interactive Card</h3>
					<p>Clickable with hover state.</p>
				</Card>
			</div>
		</section>

		<!-- Inputs Section -->
		<section id="inputs" class="section">
			<h2 class="section-title">Form Inputs</h2>
			<p class="section-desc">Text input with validation states.</p>

			<div class="input-grid">
				<Input placeholder="Default input" bind:value={inputValue} />
				<Input label="With Label" placeholder="Enter value..." />
				<Input value="Filled input" disabled />
				<Input
					bind:value={inputWithError}
					error={inputWithError.length < 3 ? 'Minimum 3 characters' : ''}
					placeholder="Type to validate..."
				/>
			</div>
		</section>

		<!-- Badges Section -->
		<section id="badges" class="section">
			<h2 class="section-title">Status Badges</h2>
			<p class="section-desc">Indicate state with semantic colors.</p>

			<div class="badge-row">
				<Badge variant="success" glow pulse>Running</Badge>
				<Badge variant="warning" pulse>Starting</Badge>
				<Badge variant="error" glow>Error</Badge>
				<Badge variant="neutral">Stopped</Badge>
				<Badge variant="info">Info</Badge>
				<Badge variant="inference" glow pulse>Analyzing</Badge>
			</div>

			<div class="badge-row" style="margin-top: var(--space-4)">
				<Badge variant="success" size="sm">Small</Badge>
				<Badge variant="success" size="md">Medium</Badge>
			</div>
		</section>

		<!-- Progress Section -->
		<section id="progress" class="section">
			<h2 class="section-title">Progress Indicators</h2>
			<p class="section-desc">Track completion and loading states.</p>

			<div class="progress-demos">
				<div class="progress-item">
					<span class="progress-label-inline">Default ({progressValue}%)</span>
					<Progress value={progressValue} showLabel />
				</div>
				<div class="progress-item">
					<span class="progress-label-inline">Gradient</span>
					<Progress value={progressValue} variant="gradient" />
				</div>
				<div class="progress-item">
					<span class="progress-label-inline">Indeterminate</span>
					<Progress indeterminate />
				</div>
				<div class="progress-item">
					<span class="progress-label-inline">Sizes</span>
					<div class="progress-sizes">
						<Progress value={50} size="sm" />
						<Progress value={50} size="md" />
						<Progress value={50} size="lg" />
					</div>
				</div>
			</div>

			<div class="progress-control">
				<input
					type="range"
					min="0"
					max="100"
					bind:value={progressValue}
					class="slider"
				/>
			</div>
		</section>

		<!-- Modals Section -->
		<section id="modals" class="section">
			<h2 class="section-title">Modals</h2>
			<p class="section-desc">Overlay dialogs for focused interactions.</p>

			<Button onclick={() => (modalOpen = true)}>Open Modal</Button>

			<Modal bind:open={modalOpen} title="Confirm Action" onclose={() => (modalOpen = false)}>
				{#snippet children()}
					<p>Are you sure you want to proceed? This action cannot be undone.</p>
				{/snippet}
				{#snippet footer()}
					<Button variant="secondary" onclick={() => (modalOpen = false)}>Cancel</Button>
					<Button variant="primary" onclick={() => (modalOpen = false)}>Confirm</Button>
				{/snippet}
			</Modal>
		</section>

		<!-- Tabs Section -->
		<section id="tabs" class="section">
			<h2 class="section-title">Tabs</h2>
			<p class="section-desc">Navigate between related views.</p>

			<Tabs {tabs} bind:activeTab>
				{#snippet children(tab)}
					<Card>
						{#if tab === 'overview'}
							<h3>Overview Content</h3>
							<p>General information and summary.</p>
						{:else if tab === 'components'}
							<h3>Components Content</h3>
							<p>Detailed component documentation.</p>
						{:else}
							<h3>Patterns Content</h3>
							<p>Common usage patterns and examples.</p>
						{/if}
					</Card>
				{/snippet}
			</Tabs>
		</section>

		<!-- Tooltips Section -->
		<section id="tooltips" class="section">
			<h2 class="section-title">Tooltips</h2>
			<p class="section-desc">Contextual help on hover.</p>

			<div class="tooltip-row">
				<Tooltip content="Above the element" position="top">
					<Button variant="secondary">Top</Button>
				</Tooltip>
				<Tooltip content="Below the element" position="bottom">
					<Button variant="secondary">Bottom</Button>
				</Tooltip>
				<Tooltip content="Left side" position="left">
					<Button variant="secondary">Left</Button>
				</Tooltip>
				<Tooltip content="Right side" position="right">
					<Button variant="secondary">Right</Button>
				</Tooltip>
			</div>
		</section>

		<!-- DropZone Section -->
		<section id="dropzone" class="section">
			<h2 class="section-title">Drop Zone</h2>
			<p class="section-desc">Drag & drop file upload area.</p>

			<DropZone ondrop={handleFileDrop} />
		</section>

		<!-- Chat Section -->
		<section id="chat" class="section">
			<h2 class="section-title">Chat Components</h2>
			<p class="section-desc">Conversational UI elements.</p>

			<div class="chat-demo">
				<ChatBubble role="user" timestamp={new Date()}>
					Analyze the footage for any safety violations in the warehouse area.
				</ChatBubble>
				<ChatBubble role="assistant" timestamp={new Date()}>
					I'll search through the video frames to identify any potential safety violations in the warehouse area. Let me process the footage now...
				</ChatBubble>
				<ChatBubble role="assistant" streaming>
					Found 3 instances where workers are not wearing required PPE
				</ChatBubble>
				<ChatBubble role="system">
					Analysis complete • 847 frames processed
				</ChatBubble>
			</div>
		</section>

		<!-- Tool Calls Section -->
		<section id="tools" class="section">
			<h2 class="section-title">Tool Call Cards</h2>
			<p class="section-desc">Display tool execution in chat context.</p>

			<div class="tool-demos">
				<ToolCallCard name="search_videos" status="complete" duration={234} arguments={{ query: 'safety violations', limit: 10 }} result="Found 3 matching segments" />
				<ToolCallCard name="extract_clip" status="running" arguments={{ start: '00:01:24', end: '00:01:45' }} />
				<ToolCallCard name="analyze_frame" status="pending" />
				<ToolCallCard name="generate_report" status="error" arguments={{ format: 'pdf' }} result="Connection timeout" />
			</div>
		</section>

		<!-- Skeleton Section -->
		<section id="skeleton" class="section">
			<h2 class="section-title">Loading Skeletons</h2>
			<p class="section-desc">Placeholder content during loading states.</p>

			<div class="skeleton-demos">
				<Card>
					<div class="skeleton-card-demo">
						<Skeleton variant="circular" width="48px" height="48px" />
						<div class="skeleton-text-group">
							<Skeleton variant="text" width="60%" />
							<Skeleton variant="text" lines={2} />
						</div>
					</div>
				</Card>
				<Card>
					<Skeleton variant="rectangular" height="120px" />
				</Card>
			</div>
		</section>

		<!-- Icons Section -->
		<section id="icons" class="section">
			<h2 class="section-title">Icons</h2>
			<p class="section-desc">Consistent iconography throughout the interface.</p>

			<div class="icon-grid">
				{#each ['plus', 'folder', 'video', 'search', 'upload', 'download', 'check', 'x', 'settings', 'trash', 'sparkles', 'zap', 'messageSquare', 'cpu', 'eye', 'play', 'pause', 'menu', 'home', 'user', 'clock', 'film', 'image', 'file', 'fileText', 'externalLink', 'copy', 'refresh', 'info', 'alertCircle', 'sun', 'moon', 'send'] as iconName}
					<Tooltip content={iconName}>
						<div class="icon-item">
							<Icon name={iconName as any} />
						</div>
					</Tooltip>
				{/each}
			</div>
		</section>
	</main>
</div>

<style>
	.style-guide {
		display: grid;
		grid-template-columns: 200px 1fr;
		grid-template-rows: auto 1fr;
		min-height: 100vh;
	}

	/* Header */
	.header {
		grid-column: 1 / -1;
		display: flex;
		align-items: center;
		gap: var(--space-6);
		padding: var(--space-4) var(--space-6);
		background: var(--color-bg-secondary);
		border-bottom: 1px solid var(--color-border);
	}

	.back-link {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		color: var(--color-text-secondary);
		font-size: var(--text-small);
		text-decoration: none;
		transition: color var(--duration-fast) var(--ease-out);
	}

	.back-link:hover {
		color: var(--color-text-primary);
	}

	.header-content h1 {
		font-size: var(--text-h1);
		margin: 0;
	}

	.header-subtitle {
		font-size: var(--text-small);
		color: var(--color-text-secondary);
		margin: 0;
	}

	.header-actions {
		margin-left: auto;
	}

	/* Sidebar */
	.sidebar {
		position: sticky;
		top: 0;
		height: calc(100vh - 65px);
		overflow-y: auto;
		padding: var(--space-4);
		background: var(--color-bg-secondary);
		border-right: 1px solid var(--color-border);
	}

	.nav-list {
		list-style: none;
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}

	.nav-link {
		display: block;
		padding: var(--space-2) var(--space-3);
		color: var(--color-text-secondary);
		font-size: var(--text-small);
		text-decoration: none;
		border-radius: var(--radius-md);
		transition:
			background-color var(--duration-fast) var(--ease-out),
			color var(--duration-fast) var(--ease-out);
	}

	.nav-link:hover {
		background: var(--color-bg-tertiary);
		color: var(--color-text-primary);
	}

	/* Content */
	.content {
		padding: var(--space-8);
		overflow-y: auto;
	}

	.section {
		margin-bottom: var(--space-12);
		scroll-margin-top: var(--space-4);
	}

	.section-title {
		font-size: var(--text-h1);
		margin-bottom: var(--space-2);
		padding-bottom: var(--space-2);
		border-bottom: 1px solid var(--color-border);
	}

	.section-desc {
		color: var(--color-text-secondary);
		margin-bottom: var(--space-6);
		max-width: 600px;
	}

	.subsection-title {
		font-size: var(--text-h3);
		color: var(--color-text-secondary);
		margin: var(--space-6) 0 var(--space-3);
	}

	/* Colors */
	.color-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
		gap: var(--space-4);
	}

	.color-swatch {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.swatch {
		height: 80px;
		border-radius: var(--radius-lg);
		border: 1px solid var(--color-border);
	}

	.swatch-text {
		display: flex;
		align-items: center;
		justify-content: center;
		font-weight: 600;
	}

	.swatch-info {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.swatch-name {
		font-weight: 600;
		color: var(--color-text-primary);
	}

	.swatch-hex {
		font-family: var(--font-mono);
		font-size: var(--text-small);
		color: var(--color-text-secondary);
		background: none;
		padding: 0;
	}

	.swatch-desc {
		font-size: var(--text-small);
		color: var(--color-text-muted);
	}

	/* Typography */
	.type-scale {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}

	.type-sample {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		padding: var(--space-3);
		background: var(--color-bg-tertiary);
		border-radius: var(--radius-lg);
	}

	.type-label {
		font-size: var(--text-small);
		color: var(--color-text-muted);
	}

	/* Spacing */
	.spacing-scale {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-4);
	}

	.spacing-item {
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}

	.spacing-box {
		background: var(--color-primary);
		border-radius: var(--radius-sm);
		min-width: 16px;
		min-height: 16px;
	}

	.spacing-value {
		font-size: var(--text-small);
		color: var(--color-text-muted);
	}

	/* Buttons */
	.component-row {
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
	}

	.component-group {
		display: flex;
		flex-direction: column;
	}

	.button-row {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-3);
	}

	/* Cards */
	.card-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
		gap: var(--space-4);
	}

	/* Inputs */
	.input-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
		gap: var(--space-4);
	}

	/* Badges */
	.badge-row {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-3);
	}

	/* Progress */
	.progress-demos {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		max-width: 400px;
	}

	.progress-item {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.progress-label-inline {
		font-size: var(--text-small);
		color: var(--color-text-secondary);
	}

	.progress-sizes {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.progress-control {
		margin-top: var(--space-4);
		max-width: 400px;
	}

	.slider {
		width: 100%;
		accent-color: var(--color-primary);
	}

	/* Tooltips */
	.tooltip-row {
		display: flex;
		gap: var(--space-4);
	}

	/* Chat */
	.chat-demo {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		max-width: 600px;
		padding: var(--space-4);
		background: var(--color-bg-secondary);
		border-radius: var(--radius-xl);
	}

	/* Tool demos */
	.tool-demos {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		max-width: 500px;
	}

	/* Skeleton demos */
	.skeleton-demos {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
		gap: var(--space-4);
	}

	.skeleton-card-demo {
		display: flex;
		gap: var(--space-3);
	}

	.skeleton-text-group {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	/* Icons */
	.icon-grid {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
	}

	.icon-item {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 44px;
		height: 44px;
		background: var(--color-bg-tertiary);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-text-secondary);
		transition:
			color var(--duration-fast) var(--ease-out),
			border-color var(--duration-fast) var(--ease-out);
	}

	.icon-item:hover {
		color: var(--color-primary);
		border-color: var(--color-primary);
	}
</style>
