# Apple Platform Constraints

Non-negotiable requirements for `platform/apple/` code.

## Video Decoding

- Use `vtdec_hw` GStreamer element for H.264/H.265
- VP9/AV1 use software decode (no hardware support)
- Output format: NV12 preferred, BGRA8 for direct render

## GPU Rendering

- Metal backend via wgpu
- IOSurface for zero-copy video textures
- Handle Retina scale factor: `window.scale_factor()`

## Memory

- Unified memory architecture
- No explicit GPU↔CPU transfers needed
- IOSurface provides shared memory

## Inference

- TTRA backend with Metal MPS acceleration
- Supported formats: GGUF, MLX, SafeTensors
- OpenAI-compatible API at configurable URL

## Required Patterns

```rust
// VideoToolbox decoder element
let decoder = match codec {
    CodecType::H264 | CodecType::H265 => "vtdec_hw",
    _ => "avdec_h264",
};

// IOSurface texture import
let texture = device.new_texture_with_io_surface(surface, &desc);

// Retina handling
let scale = window.scale_factor();
let physical = logical_size.to_physical::<u32>(scale);
```

## Dependencies

```toml
[target.'cfg(target_os = "macos")'.dependencies]
metal = "0.28"
io-surface = "0.15"
```

## See Also

- Full guide: `docs/agent-guides/apple-platform.md`
- Traits: `shared/platform-traits/src/`
