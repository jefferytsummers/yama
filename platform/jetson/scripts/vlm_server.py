#!/usr/bin/env python3
"""
VLM Inference Server for Yama on Jetson Thor

Flask-based HTTP API for LLaVA inference.

Endpoints:
  - GET /v2/health/ready - Health check
  - GET /v2/health/live - Liveness check
  - POST /v2/models/llava/infer - Run inference

Environment:
  - VLM_MODEL_PATH: Path to model directory (default: /data/models/llava/llava-1.5-7b-hf)
  - VLM_DTYPE: Data type (default: float16)
  - VLM_MAX_NEW_TOKENS: Max tokens to generate (default: 512)
"""

import os
import base64
import io
import time
import threading
from dataclasses import dataclass, field
from typing import List

import torch
from flask import Flask, request, jsonify
from transformers import AutoProcessor, LlavaForConditionalGeneration
from PIL import Image

app = Flask(__name__)

# Configuration
MODEL_PATH = os.environ.get("VLM_MODEL_PATH", "/data/models/llava/llava-1.5-7b-hf")
DTYPE = torch.float16 if os.environ.get("VLM_DTYPE", "float16") == "float16" else torch.float32
MAX_NEW_TOKENS = int(os.environ.get("VLM_MAX_NEW_TOKENS", "512"))

# Global model and processor
model = None
processor = None
model_load_time = 0.0


@dataclass
class InferenceMetrics:
    """Track inference statistics."""
    total_requests: int = 0
    successful_requests: int = 0
    failed_requests: int = 0
    total_tokens: int = 0
    total_time_ms: float = 0.0
    latencies_ms: List[float] = field(default_factory=list)
    lock: threading.Lock = field(default_factory=threading.Lock)

    def record_inference(self, tokens: int, time_ms: float, success: bool = True):
        with self.lock:
            self.total_requests += 1
            if success:
                self.successful_requests += 1
                self.total_tokens += tokens
                self.total_time_ms += time_ms
                # Keep last 100 latencies for percentile calculation
                self.latencies_ms.append(time_ms)
                if len(self.latencies_ms) > 100:
                    self.latencies_ms.pop(0)
            else:
                self.failed_requests += 1

    def avg_latency_ms(self) -> float:
        with self.lock:
            if not self.latencies_ms:
                return 0.0
            return sum(self.latencies_ms) / len(self.latencies_ms)

    def avg_tokens_per_second(self) -> float:
        with self.lock:
            if self.total_time_ms == 0:
                return 0.0
            return (self.total_tokens / self.total_time_ms) * 1000


# Global metrics
metrics = InferenceMetrics()


def load_model():
    """Load the LLaVA model."""
    global model, processor, model_load_time

    print(f"Loading model from {MODEL_PATH}...")
    print(f"GPU Memory before load: {torch.cuda.memory_allocated()/1024**3:.2f} GB")

    start = time.time()
    model = LlavaForConditionalGeneration.from_pretrained(
        MODEL_PATH,
        torch_dtype=DTYPE,
        device_map="auto",
        low_cpu_mem_usage=True,
    )
    processor = AutoProcessor.from_pretrained(MODEL_PATH)
    model_load_time = time.time() - start

    print(f"Model loaded in {model_load_time:.2f}s")
    print(f"GPU Memory after load: {torch.cuda.memory_allocated()/1024**3:.2f} GB")


@app.route("/v2/health/ready", methods=["GET"])
def health_ready():
    """Health check endpoint."""
    if model is not None and processor is not None:
        return jsonify({"status": "ready"})
    return jsonify({"status": "loading"}), 503


@app.route("/v2/health/live", methods=["GET"])
def health_live():
    """Liveness check endpoint."""
    return jsonify({"status": "live"})


@app.route("/v2/models/llava/infer", methods=["POST"])
def infer():
    """Run inference on an image."""
    if model is None or processor is None:
        return jsonify({"error": "Model not loaded"}), 503

    data = request.json
    prompt = data.get("prompt", "Describe this image.")
    image_data = data.get("image")  # base64 encoded
    max_tokens = data.get("max_tokens", MAX_NEW_TOKENS)
    temperature = data.get("temperature", 0.7)

    if not image_data:
        return jsonify({"error": "No image provided"}), 400

    try:
        # Decode image
        image_bytes = base64.b64decode(image_data)
        image = Image.open(io.BytesIO(image_bytes)).convert("RGB")

        # Format prompt using HuggingFace chat template
        conversation = [
            {
                "role": "user",
                "content": [
                    {"type": "image"},
                    {"type": "text", "text": prompt},
                ],
            },
        ]
        text_prompt = processor.apply_chat_template(conversation, add_generation_prompt=True)

        # Process inputs
        inputs = processor(text=text_prompt, images=image, return_tensors="pt").to("cuda", DTYPE)

        # Generate
        start = time.time()
        with torch.inference_mode():
            output = model.generate(
                **inputs,
                max_new_tokens=max_tokens,
                do_sample=True,
                temperature=temperature,
                top_p=0.9,
            )
        gen_time = time.time() - start

        # Decode
        response = processor.decode(output[0], skip_special_tokens=True)
        tokens_generated = output.shape[1] - inputs["input_ids"].shape[1]

        # Extract assistant response
        if "ASSISTANT:" in response:
            response = response.split("ASSISTANT:")[-1].strip()

        gen_time_ms = gen_time * 1000
        metrics.record_inference(tokens_generated, gen_time_ms, success=True)

        return jsonify({
            "response": response,
            "tokens_generated": tokens_generated,
            "generation_time_ms": gen_time_ms,
            "tokens_per_second": tokens_generated / gen_time if gen_time > 0 else 0,
        })

    except Exception as e:
        metrics.record_inference(0, 0, success=False)
        return jsonify({"error": str(e)}), 500


@app.route("/metrics", methods=["GET"])
def get_metrics():
    """Prometheus-style metrics endpoint."""
    gpu_mem_allocated = torch.cuda.memory_allocated() if torch.cuda.is_available() else 0
    gpu_mem_reserved = torch.cuda.memory_reserved() if torch.cuda.is_available() else 0
    gpu_mem_total = torch.cuda.get_device_properties(0).total_memory if torch.cuda.is_available() else 0

    metrics_text = f"""# HELP vlm_gpu_memory_allocated_bytes GPU memory allocated in bytes
# TYPE vlm_gpu_memory_allocated_bytes gauge
vlm_gpu_memory_allocated_bytes {gpu_mem_allocated}

# HELP vlm_gpu_memory_reserved_bytes GPU memory reserved in bytes
# TYPE vlm_gpu_memory_reserved_bytes gauge
vlm_gpu_memory_reserved_bytes {gpu_mem_reserved}

# HELP vlm_gpu_memory_total_bytes Total GPU memory in bytes
# TYPE vlm_gpu_memory_total_bytes gauge
vlm_gpu_memory_total_bytes {gpu_mem_total}

# HELP vlm_model_loaded Model loaded status (1=loaded, 0=not loaded)
# TYPE vlm_model_loaded gauge
vlm_model_loaded {1 if model is not None else 0}

# HELP vlm_model_load_time_seconds Time to load model
# TYPE vlm_model_load_time_seconds gauge
vlm_model_load_time_seconds {model_load_time:.3f}

# HELP vlm_inference_requests_total Total number of inference requests
# TYPE vlm_inference_requests_total counter
vlm_inference_requests_total {metrics.total_requests}

# HELP vlm_inference_success_total Successful inference requests
# TYPE vlm_inference_success_total counter
vlm_inference_success_total {metrics.successful_requests}

# HELP vlm_inference_failed_total Failed inference requests
# TYPE vlm_inference_failed_total counter
vlm_inference_failed_total {metrics.failed_requests}

# HELP vlm_tokens_generated_total Total tokens generated
# TYPE vlm_tokens_generated_total counter
vlm_tokens_generated_total {metrics.total_tokens}

# HELP vlm_inference_latency_avg_ms Average inference latency in milliseconds
# TYPE vlm_inference_latency_avg_ms gauge
vlm_inference_latency_avg_ms {metrics.avg_latency_ms():.2f}

# HELP vlm_tokens_per_second_avg Average tokens per second
# TYPE vlm_tokens_per_second_avg gauge
vlm_tokens_per_second_avg {metrics.avg_tokens_per_second():.2f}
"""
    return metrics_text, 200, {"Content-Type": "text/plain"}


if __name__ == "__main__":
    load_model()
    app.run(host="0.0.0.0", port=8000, threaded=True)
