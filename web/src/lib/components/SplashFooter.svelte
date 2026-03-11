<script lang="ts">
	const currentYear = new Date().getFullYear();
</script>

<footer class="footer">
	<div class="footer-content">
		<div class="footer-brand">
			<div class="footer-logo">
				<svg width="24" height="24" viewBox="0 0 64 64" fill="none">
					<path d="M32 8L8 52h48L32 8z" stroke="currentColor" stroke-width="3" fill="none" />
				</svg>
				<span>YAMA</span>
			</div>
			<p class="footer-tagline">Video intelligence for professionals</p>
		</div>

		<nav class="footer-links">
			<div class="footer-column">
				<h4>Product</h4>
				<a href="/features">Features</a>
				<a href="/pricing">Pricing</a>
				<a href="/changelog">Changelog</a>
			</div>
			<div class="footer-column">
				<h4>Resources</h4>
				<a href="/docs">Documentation</a>
				<a href="/api">API Reference</a>
				<a href="/support">Support</a>
			</div>
			<div class="footer-column">
				<h4>Company</h4>
				<a href="/about">About</a>
				<a href="/blog">Blog</a>
				<a href="/contact">Contact</a>
			</div>
		</nav>
	</div>

	<div class="footer-bottom">
		<p class="copyright">&copy; {currentYear} Yama. All rights reserved.</p>
		<div class="legal-links">
			<a href="/privacy">Privacy</a>
			<a href="/terms">Terms</a>
		</div>
	</div>
</footer>

<style>
	.footer {
		background: var(--color-bg-secondary);
		border-top: 1px solid var(--color-border);
		padding: var(--space-10) var(--space-6) var(--space-6);
	}

	.footer-content {
		display: flex;
		justify-content: space-between;
		gap: var(--space-10);
		max-width: 1200px;
		margin: 0 auto var(--space-8);
	}

	.footer-brand {
		max-width: 240px;
	}

	.footer-logo {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		color: var(--color-primary);
		font-size: var(--text-h3);
		font-weight: 700;
		letter-spacing: 0.05em;
		margin-bottom: var(--space-2);
	}

	.footer-tagline {
		font-size: var(--text-small);
		color: var(--color-text-tertiary);
		margin: 0;
	}

	.footer-links {
		display: flex;
		gap: var(--space-10);
	}

	.footer-column {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}

	.footer-column h4 {
		font-size: var(--text-small);
		font-weight: 600;
		color: var(--color-text-primary);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		margin: 0 0 var(--space-1);
	}

	.footer-column a {
		font-size: var(--text-small);
		color: var(--color-text-secondary);
		text-decoration: none;
		transition: color var(--duration-fast) var(--ease-out);
	}

	.footer-column a:hover {
		color: var(--color-primary);
	}

	.footer-bottom {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding-top: var(--space-6);
		border-top: 1px solid var(--color-border);
		max-width: 1200px;
		margin: 0 auto;
	}

	.copyright {
		font-size: var(--text-small);
		color: var(--color-text-muted);
		margin: 0;
	}

	.legal-links {
		display: flex;
		gap: var(--space-6);
	}

	.legal-links a {
		font-size: var(--text-small);
		color: var(--color-text-muted);
		text-decoration: none;
		transition: color var(--duration-fast) var(--ease-out);
	}

	.legal-links a:hover {
		color: var(--color-text-secondary);
	}

	@media (max-width: 768px) {
		.footer-content {
			flex-direction: column;
		}

		.footer-links {
			flex-wrap: wrap;
			gap: var(--space-6);
		}

		.footer-column {
			min-width: 120px;
		}

		.footer-bottom {
			flex-direction: column;
			gap: var(--space-4);
			text-align: center;
		}
	}
</style>
