/**
 * REST API client for Yama Host UI.
 */

const API_BASE = '/api';

export interface ServiceInfo {
	name: string;
	status: string;
	container_id: string | null;
}

export interface ConfigResponse {
	compositor: {
		backend: string;
		renderer: string;
		fps: number;
		vsync: boolean;
		width: number;
		height: number;
	};
	event_bus: {
		websocket_bind: string;
		unix_socket: string;
	};
	orchestrator: {
		runtime: string;
		health_interval: number;
		max_restarts: number;
		service_count: number;
	};
	http_server: {
		bind: string;
		port: number;
		cors_enabled: boolean;
	};
}

export interface MetricsResponse {
	cpu_usage_percent: number;
	memory_used_mb: number;
	memory_total_mb: number;
	gpu_usage_percent: number | null;
	gpu_memory_used_mb: number | null;
	uptime_seconds: number;
}

export interface VideoSourceInfo {
	name: string;
	url: string;
	enabled: boolean;
	status: string;
}

export interface ApiResponse {
	success: boolean;
	message: string;
}

export interface HealthResponse {
	status: string;
	version: string;
}

export interface SubsystemStatus {
	name: string;
	status: string;
	details: string;
}

export interface HostStatusResponse {
	version: string;
	platform: string;
	arch: string;
	event_bus: SubsystemStatus;
	http_server: SubsystemStatus;
	orchestrator: SubsystemStatus;
	compositor: SubsystemStatus;
}

async function fetchJson<T>(url: string, options?: RequestInit): Promise<T> {
	const response = await fetch(url, {
		headers: {
			'Content-Type': 'application/json',
			...options?.headers
		},
		...options
	});

	if (!response.ok) {
		throw new Error(`HTTP error: ${response.status} ${response.statusText}`);
	}

	return response.json();
}

export const api = {
	async health(): Promise<HealthResponse> {
		return fetchJson<HealthResponse>(`${API_BASE}/health`);
	},

	async getStatus(): Promise<HostStatusResponse> {
		return fetchJson<HostStatusResponse>(`${API_BASE}/status`);
	},

	async getServices(): Promise<ServiceInfo[]> {
		return fetchJson<ServiceInfo[]>(`${API_BASE}/services`);
	},

	async startService(id: string): Promise<ApiResponse> {
		return fetchJson<ApiResponse>(`${API_BASE}/services/${id}/start`, {
			method: 'POST'
		});
	},

	async stopService(id: string): Promise<ApiResponse> {
		return fetchJson<ApiResponse>(`${API_BASE}/services/${id}/stop`, {
			method: 'POST'
		});
	},

	async getConfig(): Promise<ConfigResponse> {
		return fetchJson<ConfigResponse>(`${API_BASE}/config`);
	},

	async getMetrics(): Promise<MetricsResponse> {
		return fetchJson<MetricsResponse>(`${API_BASE}/metrics`);
	},

	async getVideoSources(): Promise<VideoSourceInfo[]> {
		return fetchJson<VideoSourceInfo[]>(`${API_BASE}/video-sources`);
	}
};
