/**
 * Mock authentication store
 * Will be replaced with real auth in Phase 4+
 */

import { writable, derived, get } from 'svelte/store';
import type { User, SignupData, AuthState } from '$lib/types';

// Create the auth store
const createAuthStore = () => {
	const { subscribe, set, update } = writable<AuthState>({
		isAuthenticated: false,
		user: null,
		isLoading: false
	});

	return {
		subscribe,

		/**
		 * Mock signup - simulates account creation
		 */
		async signup(data: SignupData): Promise<User> {
			update((s) => ({ ...s, isLoading: true }));

			// Simulate API delay
			await new Promise((resolve) => setTimeout(resolve, 1200));

			const user: User = {
				id: crypto.randomUUID(),
				email: data.email,
				name: data.name,
				createdAt: new Date().toISOString()
			};

			set({
				isAuthenticated: true,
				user,
				isLoading: false
			});

			// Store in localStorage for persistence
			if (typeof window !== 'undefined') {
				localStorage.setItem('yama_user', JSON.stringify(user));
			}

			return user;
		},

		/**
		 * Mock login
		 */
		async login(email: string, _password: string): Promise<User> {
			update((s) => ({ ...s, isLoading: true }));

			await new Promise((resolve) => setTimeout(resolve, 800));

			const user: User = {
				id: crypto.randomUUID(),
				email,
				createdAt: new Date().toISOString()
			};

			set({
				isAuthenticated: true,
				user,
				isLoading: false
			});

			if (typeof window !== 'undefined') {
				localStorage.setItem('yama_user', JSON.stringify(user));
			}

			return user;
		},

		/**
		 * Logout
		 */
		logout(): void {
			set({
				isAuthenticated: false,
				user: null,
				isLoading: false
			});

			if (typeof window !== 'undefined') {
				localStorage.removeItem('yama_user');
			}
		},

		/**
		 * Initialize auth from localStorage
		 */
		init(): void {
			if (typeof window !== 'undefined') {
				const stored = localStorage.getItem('yama_user');
				if (stored) {
					try {
						const user = JSON.parse(stored) as User;
						set({
							isAuthenticated: true,
							user,
							isLoading: false
						});
					} catch {
						localStorage.removeItem('yama_user');
					}
				}
			}
		}
	};
};

export const auth = createAuthStore();

// Derived stores for convenience
export const isAuthenticated = derived(auth, ($auth) => $auth.isAuthenticated);
export const currentUser = derived(auth, ($auth) => $auth.user);
export const isLoading = derived(auth, ($auth) => $auth.isLoading);
