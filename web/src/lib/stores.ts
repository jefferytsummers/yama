/**
 * Svelte stores for application state.
 */

import { writable, derived, type Readable } from 'svelte/store';
import { api, type ServiceInfo, type MetricsResponse, type ConfigResponse, type VideoSourceInfo, type HostStatusResponse } from './api';

// Services store
function createServicesStore() {
	const { subscribe, set, update } = writable<ServiceInfo[]>([]);
	let pollInterval: ReturnType<typeof setInterval> | null = null;

	return {
		subscribe,
		async refresh() {
			try {
				const services = await api.getServices();
				set(services);
			} catch (error) {
				console.error('Failed to fetch services:', error);
			}
		},
		startPolling(intervalMs = 5000) {
			this.refresh();
			pollInterval = setInterval(() => this.refresh(), intervalMs);
		},
		stopPolling() {
			if (pollInterval) {
				clearInterval(pollInterval);
				pollInterval = null;
			}
		},
		async startService(id: string) {
			try {
				const result = await api.startService(id);
				if (result.success) {
					await this.refresh();
				}
				return result;
			} catch (error) {
				console.error('Failed to start service:', error);
				throw error;
			}
		},
		async stopService(id: string) {
			try {
				const result = await api.stopService(id);
				if (result.success) {
					await this.refresh();
				}
				return result;
			} catch (error) {
				console.error('Failed to stop service:', error);
				throw error;
			}
		}
	};
}

export const services = createServicesStore();

// Metrics store
function createMetricsStore() {
	const { subscribe, set } = writable<MetricsResponse | null>(null);
	let pollInterval: ReturnType<typeof setInterval> | null = null;

	return {
		subscribe,
		async refresh() {
			try {
				const metrics = await api.getMetrics();
				set(metrics);
			} catch (error) {
				console.error('Failed to fetch metrics:', error);
			}
		},
		startPolling(intervalMs = 2000) {
			this.refresh();
			pollInterval = setInterval(() => this.refresh(), intervalMs);
		},
		stopPolling() {
			if (pollInterval) {
				clearInterval(pollInterval);
				pollInterval = null;
			}
		}
	};
}

export const metrics = createMetricsStore();

// Config store
function createConfigStore() {
	const { subscribe, set } = writable<ConfigResponse | null>(null);

	return {
		subscribe,
		async refresh() {
			try {
				const config = await api.getConfig();
				set(config);
			} catch (error) {
				console.error('Failed to fetch config:', error);
			}
		}
	};
}

export const config = createConfigStore();

// Video sources store
function createVideoSourcesStore() {
	const { subscribe, set } = writable<VideoSourceInfo[]>([]);

	return {
		subscribe,
		async refresh() {
			try {
				const sources = await api.getVideoSources();
				set(sources);
			} catch (error) {
				console.error('Failed to fetch video sources:', error);
			}
		}
	};
}

export const videoSources = createVideoSourcesStore();

// Connection status store
export const connectionStatus = writable<'connected' | 'disconnected' | 'connecting'>('disconnected');

// Host status store (for console)
function createHostStatusStore() {
	const { subscribe, set } = writable<HostStatusResponse | null>(null);
	let pollInterval: ReturnType<typeof setInterval> | null = null;

	return {
		subscribe,
		async refresh() {
			try {
				const status = await api.getStatus();
				set(status);
			} catch (error) {
				console.error('Failed to fetch host status:', error);
			}
		},
		startPolling(intervalMs = 2000) {
			this.refresh();
			pollInterval = setInterval(() => this.refresh(), intervalMs);
		},
		stopPolling() {
			if (pollInterval) {
				clearInterval(pollInterval);
				pollInterval = null;
			}
		}
	};
}

export const hostStatus = createHostStatusStore();

// Derived store: count of running services
export const runningServicesCount: Readable<number> = derived(services, ($services) =>
	$services.filter((s) => s.status === 'running').length
);

// Derived store: total services count
export const totalServicesCount: Readable<number> = derived(services, ($services) => $services.length);
