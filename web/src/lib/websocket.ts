/**
 * WebSocket client for real-time event bus updates.
 *
 * Note: The Yama event bus uses binary Protobuf messages.
 * For the web UI, we may need a JSON bridge or a separate WebSocket endpoint.
 * This is a placeholder for future implementation.
 */

export interface EventMessage {
	topic: string;
	source: string;
	payload: unknown;
	timestamp: string;
}

export type EventHandler = (event: EventMessage) => void;

export class EventBusClient {
	private ws: WebSocket | null = null;
	private handlers: Map<string, Set<EventHandler>> = new Map();
	private reconnectAttempts = 0;
	private maxReconnectAttempts = 5;
	private reconnectDelay = 1000;

	constructor(private url: string = 'ws://localhost:8765') {}

	connect(): void {
		try {
			this.ws = new WebSocket(this.url);

			this.ws.onopen = () => {
				console.log('Event bus connected');
				this.reconnectAttempts = 0;
			};

			this.ws.onclose = () => {
				console.log('Event bus disconnected');
				this.scheduleReconnect();
			};

			this.ws.onerror = (error) => {
				console.error('Event bus error:', error);
			};

			this.ws.onmessage = (event) => {
				// TODO: Handle binary Protobuf messages
				// For now, assume JSON for testing
				try {
					const message = JSON.parse(event.data) as EventMessage;
					this.dispatch(message);
				} catch {
					console.warn('Failed to parse event bus message');
				}
			};
		} catch (error) {
			console.error('Failed to connect to event bus:', error);
			this.scheduleReconnect();
		}
	}

	disconnect(): void {
		if (this.ws) {
			this.ws.close();
			this.ws = null;
		}
	}

	subscribe(topic: string, handler: EventHandler): () => void {
		if (!this.handlers.has(topic)) {
			this.handlers.set(topic, new Set());
		}
		this.handlers.get(topic)!.add(handler);

		// Send subscription message to event bus
		// TODO: Implement proper Protobuf subscription

		// Return unsubscribe function
		return () => {
			this.handlers.get(topic)?.delete(handler);
		};
	}

	private dispatch(event: EventMessage): void {
		// Dispatch to exact topic handlers
		this.handlers.get(event.topic)?.forEach((handler) => handler(event));

		// Dispatch to wildcard handlers
		this.handlers.forEach((handlers, pattern) => {
			if (pattern.endsWith('*') && event.topic.startsWith(pattern.slice(0, -1))) {
				handlers.forEach((handler) => handler(event));
			}
		});
	}

	private scheduleReconnect(): void {
		if (this.reconnectAttempts >= this.maxReconnectAttempts) {
			console.error('Max reconnect attempts reached');
			return;
		}

		this.reconnectAttempts++;
		const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1);

		setTimeout(() => {
			console.log(`Reconnecting to event bus (attempt ${this.reconnectAttempts})`);
			this.connect();
		}, delay);
	}
}

// Singleton instance
export const eventBus = new EventBusClient();
