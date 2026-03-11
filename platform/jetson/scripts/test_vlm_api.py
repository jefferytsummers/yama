#!/usr/bin/env python3
"""
VLM API Integration Test Script

Tests the Flask VLM inference server API endpoints.
Matches the behavior expected by the Rust VlmClient.

Usage:
    python3 test_vlm_api.py [--url http://localhost:8000]
"""

import argparse
import base64
import io
import sys
import time
from dataclasses import dataclass
from typing import Optional

import requests
from PIL import Image


@dataclass
class TestResult:
    name: str
    passed: bool
    message: str
    duration_ms: float = 0


def create_test_image(width: int = 64, height: int = 64, color: tuple = (100, 150, 200)) -> bytes:
    """Create a simple test JPEG image."""
    img = Image.new('RGB', (width, height), color=color)
    buffer = io.BytesIO()
    img.save(buffer, format='JPEG')
    return buffer.getvalue()


def test_health_ready(base_url: str) -> TestResult:
    """Test /v2/health/ready endpoint."""
    start = time.time()
    try:
        resp = requests.get(f"{base_url}/v2/health/ready", timeout=10)
        duration = (time.time() - start) * 1000

        if resp.status_code == 200:
            data = resp.json()
            if data.get("status") == "ready":
                return TestResult("health_ready", True, "Server is ready", duration)
            return TestResult("health_ready", False, f"Unexpected status: {data}", duration)
        return TestResult("health_ready", False, f"HTTP {resp.status_code}", duration)
    except Exception as e:
        return TestResult("health_ready", False, str(e), 0)


def test_health_live(base_url: str) -> TestResult:
    """Test /v2/health/live endpoint."""
    start = time.time()
    try:
        resp = requests.get(f"{base_url}/v2/health/live", timeout=10)
        duration = (time.time() - start) * 1000

        if resp.status_code == 200:
            return TestResult("health_live", True, "Server is live", duration)
        return TestResult("health_live", False, f"HTTP {resp.status_code}", duration)
    except Exception as e:
        return TestResult("health_live", False, str(e), 0)


def test_metrics(base_url: str) -> TestResult:
    """Test /metrics endpoint."""
    start = time.time()
    try:
        resp = requests.get(f"{base_url}/metrics", timeout=10)
        duration = (time.time() - start) * 1000

        if resp.status_code == 200:
            text = resp.text
            # Check for new metric names
            required = ["vlm_gpu_memory_allocated_bytes", "vlm_model_loaded", "vlm_inference_requests_total"]
            if all(m in text for m in required):
                # Parse memory
                for line in text.split('\n'):
                    if line.startswith('vlm_gpu_memory_allocated_bytes') and not line.startswith('#'):
                        mem_bytes = int(line.split()[1])
                        mem_gb = mem_bytes / (1024**3)
                        return TestResult("metrics", True, f"GPU Memory: {mem_gb:.2f} GB", duration)
                return TestResult("metrics", True, "Metrics available", duration)
            return TestResult("metrics", False, f"Missing metrics. Got: {text[:200]}", duration)
        return TestResult("metrics", False, f"HTTP {resp.status_code}", duration)
    except Exception as e:
        return TestResult("metrics", False, str(e), 0)


def test_inference_simple(base_url: str) -> TestResult:
    """Test basic inference with a solid color image."""
    start = time.time()
    try:
        # Create blue test image
        img_data = create_test_image(64, 64, (50, 100, 200))
        img_b64 = base64.b64encode(img_data).decode()

        resp = requests.post(
            f"{base_url}/v2/models/llava/infer",
            json={
                "prompt": "What color is this image? Answer in one word.",
                "image": img_b64,
                "max_tokens": 20,
                "temperature": 0.1
            },
            timeout=120
        )
        duration = (time.time() - start) * 1000

        if resp.status_code == 200:
            data = resp.json()
            response_text = data.get("response", "")
            tokens = data.get("tokens_generated", 0)
            gen_time = data.get("generation_time_ms", 0)
            tok_per_sec = data.get("tokens_per_second", 0)

            return TestResult(
                "inference_simple",
                True,
                f"'{response_text[:50]}...' ({tokens} tokens, {tok_per_sec:.1f} tok/s)",
                duration
            )
        return TestResult("inference_simple", False, f"HTTP {resp.status_code}: {resp.text}", duration)
    except Exception as e:
        return TestResult("inference_simple", False, str(e), 0)


def test_inference_detailed(base_url: str) -> TestResult:
    """Test detailed inference with a more complex prompt."""
    start = time.time()
    try:
        # Create gradient test image
        img = Image.new('RGB', (256, 256))
        for x in range(256):
            for y in range(256):
                img.putpixel((x, y), (x, y, (x + y) // 2))

        buffer = io.BytesIO()
        img.save(buffer, format='JPEG', quality=85)
        img_b64 = base64.b64encode(buffer.getvalue()).decode()

        resp = requests.post(
            f"{base_url}/v2/models/llava/infer",
            json={
                "prompt": "Describe this image in detail.",
                "image": img_b64,
                "max_tokens": 100,
                "temperature": 0.7
            },
            timeout=180
        )
        duration = (time.time() - start) * 1000

        if resp.status_code == 200:
            data = resp.json()
            response_text = data.get("response", "")
            tokens = data.get("tokens_generated", 0)
            tok_per_sec = data.get("tokens_per_second", 0)

            return TestResult(
                "inference_detailed",
                True,
                f"{tokens} tokens at {tok_per_sec:.1f} tok/s",
                duration
            )
        return TestResult("inference_detailed", False, f"HTTP {resp.status_code}", duration)
    except Exception as e:
        return TestResult("inference_detailed", False, str(e), 0)


def test_inference_error_no_image(base_url: str) -> TestResult:
    """Test error handling when no image provided."""
    start = time.time()
    try:
        resp = requests.post(
            f"{base_url}/v2/models/llava/infer",
            json={
                "prompt": "Describe this image."
            },
            timeout=30
        )
        duration = (time.time() - start) * 1000

        if resp.status_code == 400:
            return TestResult("error_no_image", True, "Correctly returned 400", duration)
        return TestResult("error_no_image", False, f"Expected 400, got {resp.status_code}", duration)
    except Exception as e:
        return TestResult("error_no_image", False, str(e), 0)


def run_tests(base_url: str) -> list[TestResult]:
    """Run all tests and return results."""
    tests = [
        test_health_ready,
        test_health_live,
        test_metrics,
        test_inference_simple,
        test_inference_detailed,
        test_inference_error_no_image,
    ]

    results = []
    for test_fn in tests:
        print(f"Running {test_fn.__name__}...", end=" ", flush=True)
        result = test_fn(base_url)
        status = "PASS" if result.passed else "FAIL"
        print(f"{status} ({result.duration_ms:.0f}ms) - {result.message}")
        results.append(result)

    return results


def main():
    parser = argparse.ArgumentParser(description="Test VLM API")
    parser.add_argument("--url", default="http://localhost:8000", help="VLM server URL")
    args = parser.parse_args()

    print(f"\n{'='*60}")
    print(f"VLM API Integration Tests")
    print(f"Server: {args.url}")
    print(f"{'='*60}\n")

    results = run_tests(args.url)

    print(f"\n{'='*60}")
    passed = sum(1 for r in results if r.passed)
    total = len(results)
    print(f"Results: {passed}/{total} tests passed")

    if passed == total:
        print("All tests PASSED!")
        sys.exit(0)
    else:
        failed = [r for r in results if not r.passed]
        print(f"Failed tests: {', '.join(r.name for r in failed)}")
        sys.exit(1)


if __name__ == "__main__":
    main()
