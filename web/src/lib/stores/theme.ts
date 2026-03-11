import { writable, derived } from 'svelte/store';
import { browser } from '$app/environment';

export type Theme = 'light' | 'dark' | 'system';
export type ResolvedTheme = 'light' | 'dark';

// Get initial theme from localStorage or default to system
function getInitialTheme(): Theme {
	if (!browser) return 'system';
	const stored = localStorage.getItem('yama-theme');
	if (stored === 'light' || stored === 'dark' || stored === 'system') {
		return stored;
	}
	return 'system';
}

// Get system preference
function getSystemTheme(): ResolvedTheme {
	if (!browser) return 'dark';
	return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

// Create the theme store
function createThemeStore() {
	const { subscribe, set, update } = writable<Theme>(getInitialTheme());

	return {
		subscribe,
		set: (value: Theme) => {
			if (browser) {
				localStorage.setItem('yama-theme', value);
			}
			set(value);
		},
		toggle: () => {
			update((current) => {
				// Cycle through: system -> light -> dark -> system
				const next = current === 'system' ? 'light' : current === 'light' ? 'dark' : 'system';
				if (browser) {
					localStorage.setItem('yama-theme', next);
				}
				return next;
			});
		},
		setLight: () => {
			if (browser) localStorage.setItem('yama-theme', 'light');
			set('light');
		},
		setDark: () => {
			if (browser) localStorage.setItem('yama-theme', 'dark');
			set('dark');
		},
		setSystem: () => {
			if (browser) localStorage.setItem('yama-theme', 'system');
			set('system');
		}
	};
}

export const theme = createThemeStore();

// System theme preference (reactive)
export const systemTheme = writable<ResolvedTheme>(getSystemTheme());

// Resolved theme (what's actually applied)
export const resolvedTheme = derived(
	[theme, systemTheme],
	([$theme, $systemTheme]) => {
		if ($theme === 'system') {
			return $systemTheme;
		}
		return $theme;
	}
);

// Initialize system theme listener
export function initThemeListener() {
	if (!browser) return;

	const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');

	const handleChange = (e: MediaQueryListEvent | MediaQueryList) => {
		systemTheme.set(e.matches ? 'dark' : 'light');
	};

	// Set initial value
	handleChange(mediaQuery);

	// Listen for changes
	mediaQuery.addEventListener('change', handleChange);

	return () => {
		mediaQuery.removeEventListener('change', handleChange);
	};
}

// Apply theme to document
export function applyTheme(resolvedTheme: ResolvedTheme) {
	if (!browser) return;
	document.documentElement.setAttribute('data-theme', resolvedTheme);
}
