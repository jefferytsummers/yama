"""
LLaVA Model for Triton Inference Server

Python backend implementation for serving LLaVA VLM on Jetson Thor.
"""

import json
import os
import time
import base64
import io
from typing import List

import numpy as np
import torch
from PIL import Image

import triton_python_backend_utils as pb_utils


class TritonPythonModel:
    """LLaVA model wrapper for Triton."""

    def initialize(self, args):
        """Initialize the model."""
        self.model_config = json.loads(args["model_config"])

        # Get model parameters
        params = self.model_config.get("parameters", {})
        self.model_path = params.get("model_path", {}).get(
            "string_value", "/data/models/llava/llava-1.5-7b"
        )
        self.dtype_str = params.get("dtype", {}).get("string_value", "float16")
        self.device_map = params.get("device_map", {}).get("string_value", "auto")
        self.low_cpu_mem = params.get("low_cpu_mem_usage", {}).get("string_value", "true") == "true"

        # Set dtype
        self.dtype = torch.float16 if self.dtype_str == "float16" else torch.float32

        # Load model
        pb_utils.Logger.log_info(f"Loading LLaVA model from {self.model_path}")
        start = time.time()

        from transformers import AutoProcessor, LlavaForConditionalGeneration

        self.model = LlavaForConditionalGeneration.from_pretrained(
            self.model_path,
            torch_dtype=self.dtype,
            device_map=self.device_map,
            low_cpu_mem_usage=self.low_cpu_mem,
        )
        self.processor = AutoProcessor.from_pretrained(self.model_path)

        load_time = time.time() - start
        pb_utils.Logger.log_info(f"Model loaded in {load_time:.2f}s")
        pb_utils.Logger.log_info(
            f"GPU Memory: {torch.cuda.memory_allocated()/1024**3:.2f} GB"
        )

    def execute(self, requests: List) -> List:
        """Execute inference on batch of requests."""
        responses = []

        for request in requests:
            try:
                response = self._process_request(request)
                responses.append(response)
            except Exception as e:
                pb_utils.Logger.log_error(f"Error processing request: {e}")
                error = pb_utils.TritonError(str(e))
                responses.append(pb_utils.InferenceResponse(error=error))

        return responses

    def _process_request(self, request) -> pb_utils.InferenceResponse:
        """Process a single inference request."""
        start_time = time.time()

        # Get inputs
        prompt_tensor = pb_utils.get_input_tensor_by_name(request, "prompt")
        image_tensor = pb_utils.get_input_tensor_by_name(request, "image")

        prompt = prompt_tensor.as_numpy()[0].decode("utf-8") if prompt_tensor else "Describe this image."
        image_b64 = image_tensor.as_numpy()[0].decode("utf-8") if image_tensor else None

        # Get optional parameters
        max_tokens_tensor = pb_utils.get_input_tensor_by_name(request, "max_tokens")
        temp_tensor = pb_utils.get_input_tensor_by_name(request, "temperature")

        max_tokens = int(max_tokens_tensor.as_numpy()[0]) if max_tokens_tensor else 256
        temperature = float(temp_tensor.as_numpy()[0]) if temp_tensor else 0.7

        # Decode image
        if image_b64:
            image_bytes = base64.b64decode(image_b64)
            image = Image.open(io.BytesIO(image_bytes)).convert("RGB")
        else:
            # Create blank image if none provided
            image = Image.new("RGB", (224, 224), color=(128, 128, 128))

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
        text_prompt = self.processor.apply_chat_template(conversation, add_generation_prompt=True)

        # Process inputs
        inputs = self.processor(
            text=text_prompt,
            images=image,
            return_tensors="pt"
        ).to("cuda", self.dtype)

        # Generate response
        with torch.inference_mode():
            output = self.model.generate(
                **inputs,
                max_new_tokens=max_tokens,
                do_sample=True,
                temperature=temperature,
                top_p=0.9,
            )

        # Decode response
        response_text = self.processor.decode(output[0], skip_special_tokens=True)

        # Extract assistant response
        if "ASSISTANT:" in response_text:
            response_text = response_text.split("ASSISTANT:")[-1].strip()

        tokens_generated = output.shape[1] - inputs["input_ids"].shape[1]
        inference_time_ms = (time.time() - start_time) * 1000

        # Create output tensors
        response_tensor = pb_utils.Tensor(
            "response",
            np.array([response_text.encode("utf-8")], dtype=np.object_)
        )
        tokens_tensor = pb_utils.Tensor(
            "tokens_generated",
            np.array([tokens_generated], dtype=np.int32)
        )
        time_tensor = pb_utils.Tensor(
            "inference_time_ms",
            np.array([inference_time_ms], dtype=np.float32)
        )

        return pb_utils.InferenceResponse(
            output_tensors=[response_tensor, tokens_tensor, time_tensor]
        )

    def finalize(self):
        """Clean up resources."""
        pb_utils.Logger.log_info("Finalizing LLaVA model")
        if hasattr(self, "model"):
            del self.model
        if hasattr(self, "processor"):
            del self.processor
        torch.cuda.empty_cache()
