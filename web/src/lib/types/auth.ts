/**
 * Authentication types
 */

export interface User {
	id: string;
	email: string;
	name?: string;
	avatarUrl?: string;
	createdAt: string;
}

export interface SignupData {
	email: string;
	password: string;
	name?: string;
}

export interface LoginData {
	email: string;
	password: string;
}

export interface AuthState {
	isAuthenticated: boolean;
	user: User | null;
	isLoading: boolean;
}

export type OAuthProvider = 'google' | 'github';
