# Jetson Platform Constraints

Non-negotiable requirements for `platform/jetson/` code.

## Video Decoding

- Use `nvv4l2decoder` for all codecs (H.264/H.265/VP9/AV1)
- Use `nvvidconv` for color conversion (not videoconvert)
- Output format: NV12, use DMA-BUF for zero-copy

## GPU Rendering

- Vulkan backend via wgpu or ash
- DMA-BUF import for video textures
- DRM/KMS for display output

## Wayland Compositor

- Smithay 0.3 for Wayland compositor
- libinput for input handling
- No X11 support

## Memory

- Discrete GPU memory
- DMA-BUF required for zero-copy
- Use DRM format modifiers

## Inference

- TensorRT for optimized inference
- CUDA for custom kernels
- Pre-compiled engine files (.engine)

## Required Patterns

```rust
// NVDEC decoder pipeline
let pipeline = "appsrc ! nvv4l2decoder ! nvvidconv ! video/x-raw,format=NV12 ! appsink";

// DMA-BUF import to Vulkan
let import_info = vk::ImportMemoryFdInfoKHR::builder()
    .handle_type(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT)
    .fd(dma_buf_fd);

// Jetson detection
fn is_jetson() -> bool {
    Path::new("/etc/nv_tegra_release").exists()
}
```

## Dependencies

```toml
[target.'cfg(target_os = "linux")'.dependencies]
ash = "0.37"
smithay = { version = "0.3", features = ["backend_drm"] }
input = "0.9"
```

## Environment

```bash
# Required for Vulkan
export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/nvidia_icd.json
```

## See Also

- Full guide: `docs/agent-guides/jetson-platform.md`
- Traits: `shared/platform-traits/src/`
