<script lang="ts">
	import { Input, Button } from '$lib/components';
	import OAuthButtons from './OAuthButtons.svelte';
	import type { OAuthProvider } from '$lib/types';

	interface Props {
		onSubmit: (data: { email: string; password: string; name: string }) => Promise<void>;
		onOAuth: (provider: OAuthProvider) => void;
		isLoading?: boolean;
		error?: string | null;
	}

	let { onSubmit, onOAuth, isLoading = false, error = null }: Props = $props();

	let email = $state('');
	let password = $state('');
	let confirmPassword = $state('');
	let name = $state('');

	let emailError = $state<string | null>(null);
	let passwordError = $state<string | null>(null);
	let confirmError = $state<string | null>(null);

	let isValid = $derived(
		email.length > 0 &&
			password.length >= 8 &&
			password === confirmPassword &&
			!emailError &&
			!passwordError &&
			!confirmError
	);

	function validateEmail() {
		if (!email) {
			emailError = null;
			return;
		}
		const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
		emailError = emailRegex.test(email) ? null : 'Please enter a valid email address';
	}

	function validatePassword() {
		if (!password) {
			passwordError = null;
			return;
		}
		if (password.length < 8) {
			passwordError = 'Password must be at least 8 characters';
		} else if (!/[A-Z]/.test(password)) {
			passwordError = 'Password must contain an uppercase letter';
		} else if (!/[0-9]/.test(password)) {
			passwordError = 'Password must contain a number';
		} else {
			passwordError = null;
		}
	}

	function validateConfirmPassword() {
		if (!confirmPassword) {
			confirmError = null;
			return;
		}
		confirmError = password === confirmPassword ? null : 'Passwords do not match';
	}

	async function handleSubmit(e: Event) {
		e.preventDefault();

		validateEmail();
		validatePassword();
		validateConfirmPassword();

		if (!isValid) return;

		await onSubmit({ email, password, name });
	}
</script>

<form class="signup-form" onsubmit={handleSubmit}>
	{#if error}
		<div class="form-error">
			<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
				<circle cx="12" cy="12" r="10" />
				<line x1="12" y1="8" x2="12" y2="12" />
				<line x1="12" y1="16" x2="12.01" y2="16" />
			</svg>
			{error}
		</div>
	{/if}

	<OAuthButtons onAuth={onOAuth} disabled={isLoading} />

	<div class="divider">
		<span>or continue with email</span>
	</div>

	<div class="form-fields">
		<Input
			type="text"
			placeholder="Full name (optional)"
			bind:value={name}
			disabled={isLoading}
		/>

		<Input
			type="email"
			placeholder="Email address"
			bind:value={email}
			error={emailError}
			onblur={validateEmail}
			disabled={isLoading}
			required
		/>

		<Input
			type="password"
			placeholder="Password"
			bind:value={password}
			error={passwordError}
			onblur={validatePassword}
			disabled={isLoading}
			required
		/>

		<Input
			type="password"
			placeholder="Confirm password"
			bind:value={confirmPassword}
			error={confirmError}
			onblur={validateConfirmPassword}
			disabled={isLoading}
			required
		/>
	</div>

	<Button
		type="submit"
		variant="primary"
		size="lg"
		fullWidth
		disabled={!isValid || isLoading}
		loading={isLoading}
	>
		{isLoading ? 'Creating account...' : 'Create Account'}
	</Button>

	<p class="terms">
		By creating an account, you agree to our
		<a href="/terms">Terms of Service</a>
		and
		<a href="/privacy">Privacy Policy</a>.
	</p>
</form>

<style>
	.signup-form {
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
		width: 100%;
		max-width: 400px;
	}

	.form-error {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-3) var(--space-4);
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid var(--color-ember);
		border-radius: var(--radius-md);
		color: var(--color-ember);
		font-size: var(--text-small);
	}

	.divider {
		display: flex;
		align-items: center;
		gap: var(--space-4);
	}

	.divider::before,
	.divider::after {
		content: '';
		flex: 1;
		height: 1px;
		background: var(--color-stone);
	}

	.divider span {
		font-size: var(--text-small);
		color: var(--color-ash);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.form-fields {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}

	.terms {
		font-size: var(--text-small);
		color: var(--color-ash);
		text-align: center;
		margin: 0;
	}

	.terms a {
		color: var(--color-amber);
		text-decoration: none;
	}

	.terms a:hover {
		text-decoration: underline;
	}
</style>
