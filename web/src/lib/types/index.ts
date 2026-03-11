// Auth types
export * from './auth';

// Wizard types
export * from './wizard';

// VLM types (direct Jetson Flask API)
export * from './vlm';

// API Types - to be expanded in Phase 4

export interface Session {
	id: string;
	presetId: string;
	projectId?: string;
	title: string;
	createdAt: string;
}

export interface Message {
	id: string;
	sessionId: string;
	role: 'user' | 'assistant' | 'system';
	content: string;
	toolCalls?: ToolCall[];
	createdAt: string;
}

export interface ToolCall {
	id: string;
	name: string;
	arguments: Record<string, unknown>;
	result?: string;
	status: 'pending' | 'running' | 'complete' | 'error';
	duration?: number;
}

export interface Preset {
	id: string;
	name: string;
	description: string;
	systemPrompt: string;
	tools: string[];
	model: {
		preferred: string;
		temperature: number;
		maxTokens: number;
	};
}

export interface Project {
	id: string;
	name: string;
	description?: string;
	videoCount: number;
	createdAt: string;
}

export interface Video {
	id: string;
	libraryId: string;
	path: string;
	durationMs: number;
	width: number;
	height: number;
	fps: number;
	codec: string;
}

export interface InferenceJob {
	id: string;
	videoId: string;
	status: 'pending' | 'extracting' | 'inferring' | 'completed' | 'failed';
	progress: number;
	frameCount: number;
	processedFrames: number;
	error?: string;
}

export interface Artifact {
	id: string;
	sessionId: string;
	type: 'video_clip' | 'report' | 'summary' | 'screenshot' | 'comparison';
	name: string;
	path: string;
	sizeBytes: number;
	metadata: Record<string, unknown>;
	createdAt: string;
}
