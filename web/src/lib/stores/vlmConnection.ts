/**
 * VLM Connection Manager
 *
 * Manages connection to VLM server with auto-reconnect and status tracking.
 *
 * Status states:
 * - connected (green): Server is ready and responding
 * - connecting (blinking green): Attempting to connect
 * - disconnected (red): Cannot reach server
 * - stale (yellow): Last response was too long ago
 */

import { writable, derived, get } from 'svelte/store';
import { createVlmClient, type VlmApiClient } from '$lib/api/vlm';
import type { VlmMetrics } from '$lib/types/vlm';

export type ConnectionStatus = 'connected' | 'connecting' | 'disconnected' | 'stale';

export interface ConnectionState {
	status: ConnectionStatus;
	serverUrl: string;
	lastHealthCheck: Date | null;
	lastSuccessfulCheck: Date | null;
	consecutiveFailures: number;
	metrics: VlmMetrics | null;
	error: string | null;
	modelReady: boolean;
}

const HEALTH_CHECK_INTERVAL = 3000; // 3 seconds
const STALE_THRESHOLD = 10000; // 10 seconds without response = stale
const MAX_BACKOFF = 30000; // Max 30 second backoff
const BASE_BACKOFF = 1000; // Start at 1 second

function createVlmConnectionStore() {
	const initialState: ConnectionState = {
		status: 'disconnected',
		serverUrl: 'http://localhost:8000',
		lastHealthCheck: null,
		lastSuccessfulCheck: null,
		consecutiveFailures: 0,
		metrics: null,
		error: null,
		modelReady: false
	};

	const { subscribe, set, update } = writable<ConnectionState>(initialState);

	let client: VlmApiClient | null = null;
	let healthCheckInterval: ReturnType<typeof setInterval> | null = null;
	let reconnectTimeout: ReturnType<typeof setTimeout> | null = null;
	let isRunning = false;

	function calculateBackoff(failures: number): number {
		// Exponential backoff: 1s, 2s, 4s, 8s, 16s, 30s (capped)
		return Math.min(BASE_BACKOFF * Math.pow(2, failures), MAX_BACKOFF);
	}

	async function performHealthCheck() {
		if (!client) return;

		const state = get({ subscribe });

		// Mark as connecting if we were disconnected
		if (state.status === 'disconnected') {
			update(s => ({ ...s, status: 'connecting', error: null }));
		}

		try {
			const [healthResult, metricsResult] = await Promise.all([
				client.checkHealth(),
				client.getMetrics().catch(() => null)
			]);

			const now = new Date();

			update(s => ({
				...s,
				status: 'connected',
				lastHealthCheck: now,
				lastSuccessfulCheck: now,
				consecutiveFailures: 0,
				metrics: metricsResult ?? s.metrics,
				error: null,
				modelReady: healthResult.status === 'ready'
			}));

		} catch (error) {
			const errorMessage = error instanceof Error ? error.message : 'Unknown error';

			update(s => {
				const failures = s.consecutiveFailures + 1;
				const timeSinceSuccess = s.lastSuccessfulCheck
					? Date.now() - s.lastSuccessfulCheck.getTime()
					: Infinity;

				// Determine status based on failure pattern
				let newStatus: ConnectionStatus;
				if (failures === 1 && timeSinceSuccess < STALE_THRESHOLD) {
					// First failure, recent success - might be transient
					newStatus = 'stale';
				} else if (failures < 3) {
					// Few failures - still trying
					newStatus = 'connecting';
				} else {
					// Multiple failures - disconnected
					newStatus = 'disconnected';
				}

				return {
					...s,
					status: newStatus,
					lastHealthCheck: new Date(),
					consecutiveFailures: failures,
					error: errorMessage,
					modelReady: false
				};
			});

			// Schedule reconnect with backoff
			const state = get({ subscribe });
			if (state.status === 'disconnected' && isRunning) {
				const backoff = calculateBackoff(state.consecutiveFailures);
				console.log(`VLM connection failed, retrying in ${backoff}ms...`);

				if (reconnectTimeout) clearTimeout(reconnectTimeout);
				reconnectTimeout = setTimeout(() => {
					if (isRunning) performHealthCheck();
				}, backoff);
			}
		}
	}

	function checkForStale() {
		update(s => {
			if (s.status !== 'connected') return s;

			const timeSinceSuccess = s.lastSuccessfulCheck
				? Date.now() - s.lastSuccessfulCheck.getTime()
				: Infinity;

			if (timeSinceSuccess > STALE_THRESHOLD) {
				return { ...s, status: 'stale' };
			}
			return s;
		});
	}

	return {
		subscribe,

		/**
		 * Connect to the VLM server and start health monitoring
		 */
		connect(serverUrl?: string) {
			const state = get({ subscribe });
			const url = serverUrl ?? state.serverUrl;

			// Update URL if changed
			if (url !== state.serverUrl) {
				update(s => ({ ...s, serverUrl: url }));
			}

			// Create client
			client = createVlmClient(url);
			isRunning = true;

			// Initial status
			update(s => ({ ...s, status: 'connecting', error: null }));

			// Start health check polling
			if (healthCheckInterval) clearInterval(healthCheckInterval);
			healthCheckInterval = setInterval(() => {
				performHealthCheck();
				checkForStale();
			}, HEALTH_CHECK_INTERVAL);

			// Immediate first check
			performHealthCheck();
		},

		/**
		 * Disconnect and stop monitoring
		 */
		disconnect() {
			isRunning = false;

			if (healthCheckInterval) {
				clearInterval(healthCheckInterval);
				healthCheckInterval = null;
			}

			if (reconnectTimeout) {
				clearTimeout(reconnectTimeout);
				reconnectTimeout = null;
			}

			update(s => ({
				...s,
				status: 'disconnected',
				error: null,
				consecutiveFailures: 0
			}));
		},

		/**
		 * Force an immediate health check
		 */
		async refresh() {
			await performHealthCheck();
		},

		/**
		 * Update server URL and reconnect
		 */
		setServerUrl(url: string) {
			this.disconnect();
			update(s => ({ ...s, serverUrl: url }));
			this.connect(url);
		},

		/**
		 * Get the underlying client for API calls
		 */
		getClient(): VlmApiClient | null {
			return client;
		}
	};
}

// Singleton store
export const vlmConnection = createVlmConnectionStore();

// Derived stores for convenience
export const connectionStatus = derived(vlmConnection, $conn => $conn.status);
export const isConnected = derived(vlmConnection, $conn => $conn.status === 'connected');
export const vlmMetrics = derived(vlmConnection, $conn => $conn.metrics);
export const modelReady = derived(vlmConnection, $conn => $conn.modelReady);
