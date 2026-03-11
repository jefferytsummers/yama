/**
 * Chat types for the video analysis chat interface.
 */

export type MessageRole = 'user' | 'assistant' | 'system';

export interface VideoAttachment {
	id: string;
	path: string;
	filename: string;
	size: number;
	status: 'pending' | 'ready' | 'processing' | 'completed' | 'failed';
	duration_ms?: number;
	dimensions?: [number, number];
}

export interface FrameResult {
	frame_number: number;
	timestamp_ms: number;
	text: string;
}

export interface InferenceProgress {
	current_frame: number;
	total_frames: number;
	percent: number;
}

export interface ChatMessage {
	id: string;
	role: MessageRole;
	content: string;
	attachments: VideoAttachment[];
	frame_results: FrameResult[];
	progress?: InferenceProgress;
	job_id?: string;
	error?: string;
	timestamp: Date;
}

export interface InferenceJob {
	job_id: string;
	status: 'pending' | 'extracting' | 'inferring' | 'completed' | 'failed';
	progress_percent: number;
	current_frame: number;
	total_frames: number;
	results: FrameResult[];
	error?: string;
	model: string;
	prompt: string;
}

export interface VlmModel {
	id: string;
	name: string;
	description: string;
	context_length: number;
}
