/**
 * Yama API Client
 * Handles communication with the embedded HTTP server
 */

const getBaseUrl = (): string => {
	// In Tauri, the server runs on localhost
	// In dev mode, Vite proxies /api to the backend
	if (typeof window !== 'undefined' && window.__TAURI__) {
		// Tauri context - will be enhanced in M4.0b
		return 'http://localhost:8080';
	}
	// Browser dev mode - use relative URLs (proxied by Vite)
	return '';
};

export const apiClient = {
	baseUrl: getBaseUrl(),

	async get<T>(path: string): Promise<T> {
		const response = await fetch(`${this.baseUrl}${path}`);
		if (!response.ok) {
			throw new Error(`API error: ${response.status}`);
		}
		return response.json();
	},

	async post<T>(path: string, body?: unknown): Promise<T> {
		const response = await fetch(`${this.baseUrl}${path}`, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json'
			},
			body: body ? JSON.stringify(body) : undefined
		});
		if (!response.ok) {
			throw new Error(`API error: ${response.status}`);
		}
		return response.json();
	},

	async delete(path: string): Promise<void> {
		const response = await fetch(`${this.baseUrl}${path}`, {
			method: 'DELETE'
		});
		if (!response.ok) {
			throw new Error(`API error: ${response.status}`);
		}
	},

	/**
	 * Subscribe to SSE stream
	 */
	subscribe(path: string, handlers: {
		onMessage?: (event: MessageEvent) => void;
		onError?: (event: Event) => void;
		eventTypes?: Record<string, (data: unknown) => void>;
	}): EventSource {
		const source = new EventSource(`${this.baseUrl}${path}`);

		if (handlers.onMessage) {
			source.onmessage = handlers.onMessage;
		}

		if (handlers.onError) {
			source.onerror = handlers.onError;
		}

		if (handlers.eventTypes) {
			for (const [eventType, handler] of Object.entries(handlers.eventTypes)) {
				source.addEventListener(eventType, (e) => {
					const data = JSON.parse((e as MessageEvent).data);
					handler(data);
				});
			}
		}

		return source;
	}
};

export default apiClient;
