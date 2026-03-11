/**
 * Tauri IPC commands for native functionality.
 *
 * In Tauri context, these call native Rust functions.
 * In browser context, they provide graceful fallbacks.
 */

import type { VideoAttachment } from '$lib/types/chat';

// Type for Tauri invoke function
type TauriInvoke = (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;

// Get Tauri invoke function if available (Tauri 2.0 API)
function getTauriInvoke(): TauriInvoke | null {
	if (typeof window !== 'undefined' && '__TAURI__' in window) {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		const tauri = (window as any).__TAURI__;
		// Tauri 2.0: invoke is at __TAURI__.core.invoke
		if (tauri?.core && typeof tauri.core.invoke === 'function') {
			return tauri.core.invoke as TauriInvoke;
		}
		// Tauri 1.x fallback: invoke is at __TAURI__.invoke
		if (tauri && typeof tauri.invoke === 'function') {
			return tauri.invoke as TauriInvoke;
		}
	}
	return null;
}

export interface VideoFile {
	path: string;
	filename: string;
	size: number;
}

export interface BackendStatus {
	ready: boolean;
	http_port: number;
	event_bus_port: number;
}

/**
 * Check if running in Tauri context.
 * Checks for __TAURI__ global (injected by Tauri webview).
 */
export function isTauri(): boolean {
	if (typeof window === 'undefined') return false;
	return '__TAURI__' in window || '__TAURI_INTERNALS__' in window;
}

/**
 * Open native file picker to select video files.
 *
 * In Tauri: Uses native OS file dialog.
 * In browser: Returns empty array (use DropZone component instead).
 */
export async function pickVideoFiles(): Promise<VideoFile[]> {
	const invoke = getTauriInvoke();
	if (invoke) {
		try {
			const files = (await invoke('pick_video_files')) as VideoFile[];
			return files || [];
		} catch (error) {
			console.error('Failed to pick video files:', error);
			return [];
		}
	}

	// Browser fallback - return empty, use DropZone instead
	console.warn('pickVideoFiles called outside Tauri context');
	return [];
}

/**
 * Convert VideoFile from Tauri to VideoAttachment for UI.
 */
export function videoFileToAttachment(file: VideoFile): VideoAttachment {
	return {
		id: crypto.randomUUID(),
		path: file.path,
		filename: file.filename,
		size: file.size,
		status: 'pending'
	};
}

/**
 * Get backend service status.
 *
 * In Tauri: Queries the embedded backend.
 * In browser: Checks HTTP server health.
 */
export async function getBackendStatus(): Promise<BackendStatus> {
	const invoke = getTauriInvoke();
	if (invoke) {
		try {
			const status = (await invoke('get_backend_status')) as BackendStatus;
			return status || { ready: false, http_port: 8080, event_bus_port: 8765 };
		} catch (error) {
			console.error('Failed to get backend status:', error);
			return { ready: false, http_port: 8080, event_bus_port: 8765 };
		}
	}

	// Browser fallback - check HTTP health endpoint
	try {
		const response = await fetch('/api/health');
		const ready = response.ok;
		return { ready, http_port: 8080, event_bus_port: 8765 };
	} catch {
		return { ready: false, http_port: 8080, event_bus_port: 8765 };
	}
}
