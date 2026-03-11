/**
 * VLM API Client for Jetson Flask Server
 * Direct communication with the LLaVA inference server
 */

import type { VlmHealthResponse, VlmInferenceRequest, VlmInferenceResponse, VlmMetrics } from '$lib/types/vlm';

export class VlmApiClient {
	constructor(private baseUrl: string = 'http://localhost:8000') {}

	/**
	 * Check if the VLM server is ready
	 */
	async checkHealth(): Promise<VlmHealthResponse> {
		const response = await fetch(`${this.baseUrl}/v2/health/ready`, {
			method: 'GET',
			headers: { 'Accept': 'application/json' }
		});

		if (!response.ok) {
			if (response.status === 503) {
				return { status: 'loading' };
			}
			throw new Error(`Health check failed: ${response.status}`);
		}

		return response.json();
	}

	/**
	 * Run inference on an image
	 */
	async infer(request: VlmInferenceRequest): Promise<VlmInferenceResponse> {
		const response = await fetch(`${this.baseUrl}/v2/models/llava/infer`, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
				'Accept': 'application/json'
			},
			body: JSON.stringify({
				prompt: request.prompt,
				image: request.image,
				max_tokens: request.max_tokens ?? 256,
				temperature: request.temperature ?? 0.7
			})
		});

		if (!response.ok) {
			const error = await response.json().catch(() => ({ error: 'Unknown error' }));
			throw new Error(error.error || `Inference failed: ${response.status}`);
		}

		return response.json();
	}

	/**
	 * Get server metrics
	 */
	async getMetrics(): Promise<VlmMetrics> {
		const response = await fetch(`${this.baseUrl}/metrics`, {
			method: 'GET',
			headers: { 'Accept': 'text/plain' }
		});

		if (!response.ok) {
			throw new Error(`Metrics request failed: ${response.status}`);
		}

		const text = await response.text();
		return parsePrometheusMetrics(text);
	}
}

/**
 * Parse Prometheus text format into structured metrics
 */
function parsePrometheusMetrics(text: string): VlmMetrics {
	const getValue = (name: string): number => {
		const match = text.match(new RegExp(`^${name}\\s+(\\d+(?:\\.\\d+)?)`, 'm'));
		return match ? parseFloat(match[1]) : 0;
	};

	const bytesToGb = (bytes: number): number => bytes / (1024 ** 3);

	return {
		gpu_memory_allocated_gb: bytesToGb(getValue('vlm_gpu_memory_allocated_bytes')),
		gpu_memory_reserved_gb: bytesToGb(getValue('vlm_gpu_memory_reserved_bytes')),
		gpu_memory_total_gb: bytesToGb(getValue('vlm_gpu_memory_total_bytes')),
		model_loaded: getValue('vlm_model_loaded') === 1,
		model_load_time_seconds: getValue('vlm_model_load_time_seconds'),
		inference_requests_total: getValue('vlm_inference_requests_total'),
		inference_success_total: getValue('vlm_inference_success_total'),
		inference_failed_total: getValue('vlm_inference_failed_total'),
		tokens_generated_total: getValue('vlm_tokens_generated_total'),
		inference_latency_avg_ms: getValue('vlm_inference_latency_avg_ms'),
		tokens_per_second_avg: getValue('vlm_tokens_per_second_avg')
	};
}

// Default client instance
export const vlmClient = new VlmApiClient();

// Factory for custom server URL
export function createVlmClient(baseUrl: string): VlmApiClient {
	return new VlmApiClient(baseUrl);
}
