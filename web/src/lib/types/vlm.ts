/**
 * Types for direct VLM API interaction (Jetson Flask server)
 */

export interface VlmHealthResponse {
	status: 'ready' | 'loading' | 'error';
}

export interface VlmInferenceRequest {
	prompt: string;
	image: string; // base64 encoded
	max_tokens?: number;
	temperature?: number;
	fast_mode?: boolean; // Use greedy decoding for speed
}

export interface VlmInferenceResponse {
	response: string;
	tokens_generated: number;
	generation_time_ms: number;
	tokens_per_second: number;
}

export interface VlmMetrics {
	gpu_memory_allocated_gb: number;
	gpu_memory_reserved_gb: number;
	gpu_memory_total_gb: number;
	model_loaded: boolean;
	model_load_time_seconds: number;
	inference_requests_total: number;
	inference_success_total: number;
	inference_failed_total: number;
	tokens_generated_total: number;
	inference_latency_avg_ms: number;
	tokens_per_second_avg: number;
}

export interface VlmTestState {
	serverUrl: string;
	health: 'unknown' | 'ready' | 'loading' | 'error';
	metrics: VlmMetrics | null;
	selectedImage: File | null;
	imagePreview: string | null;
	prompt: string;
	maxTokens: number;
	temperature: number;
	isInferring: boolean;
	response: VlmInferenceResponse | null;
	error: string | null;
}
