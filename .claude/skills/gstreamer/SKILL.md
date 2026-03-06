---
name: gstreamer
description: GStreamer video pipeline development for hardware-accelerated decoding
compatibility:
  - platform/apple/video/
  - platform/jetson/video/
  - shared/platform-traits/src/decoder.rs
---

# GStreamer Video Pipelines

Hardware video decode using GStreamer with platform-specific acceleration.

## Version

GStreamer 0.23.x (Rust bindings via `gstreamer` crate)

## Platform Decoders

### Apple (VideoToolbox)

```rust
// Use vtdec_hw for hardware decode
let decoder_element = match codec {
    CodecType::H264 | CodecType::H265 => "vtdec_hw",
    CodecType::Vp9 => "vp9dec", // Software fallback
    _ => "avdec_h264",
};
```

Pipeline pattern:
```
appsrc name=src ! vtdec_hw ! videoconvert ! video/x-raw,format=NV12 ! appsink name=sink
```

### Jetson (NVDEC)

```rust
// Use nvv4l2decoder on Jetson
let decoder_element = match codec {
    CodecType::H264 | CodecType::H265 | CodecType::Vp9 | CodecType::Av1 => "nvv4l2decoder",
    _ => "avdec_h264",
};

// Use nvvidconv for efficient color conversion
let converter = "nvvidconv";
```

Pipeline pattern:
```
appsrc name=src ! nvv4l2decoder ! nvvidconv ! video/x-raw,format=NV12 ! appsink name=sink
```

## Supported Codecs

| Codec | Apple | Jetson | Notes |
|-------|-------|--------|-------|
| H.264 | vtdec_hw | nvv4l2decoder | Primary codec |
| H.265 | vtdec_hw | nvv4l2decoder | 10-bit support |
| VP9 | Software | nvv4l2decoder | WebM streams |
| AV1 | Software | nvv4l2decoder | Orin only |
| MJPEG | avdec_mjpeg | nvjpegdec | IP cameras |

## Output Formats

Prefer `NV12` for hardware paths. Conversion options:
- `NV12` - Recommended, native format
- `I420` - Compatible fallback
- `BGRA` - For direct rendering (slower)

## Key Files

- `platform/apple/video/src/decoder.rs` - VideoToolbox implementation
- `platform/jetson/video/src/decoder.rs` - NVDEC implementation
- `shared/platform-traits/src/decoder.rs` - VideoDecoder trait

## Patterns

### Pipeline Setup

```rust
// Always initialize GStreamer first
gst::init().context("Failed to initialize GStreamer")?;

// Parse pipeline from string
let pipeline = gst::parse::launch(&pipeline_str)?
    .downcast::<gst::Pipeline>()?;

// Set up appsink callbacks for frame delivery
let sink = pipeline.by_name("sink")?
    .downcast::<gstreamer_app::AppSink>()?;
```

### Frame Handling

```rust
sink.set_callbacks(
    gstreamer_app::AppSinkCallbacks::builder()
        .new_sample(move |appsink| {
            let sample = appsink.pull_sample()?;
            let buffer = sample.buffer()?;
            let caps = sample.caps()?;
            let video_info = gstreamer_video::VideoInfo::from_caps(caps)?;
            // Process frame...
            Ok(gst::FlowSuccess::Ok)
        })
        .build(),
);
```

### Buffer Management

```rust
// Create buffer from packet data
let mut buffer = gst::Buffer::from_slice(packet.data.clone());
{
    let buffer_ref = buffer.get_mut().unwrap();
    buffer_ref.set_pts(gst::ClockTime::from_useconds(packet.pts as u64));
    buffer_ref.set_dts(gst::ClockTime::from_useconds(packet.dts as u64));
    if packet.is_keyframe {
        buffer_ref.unset_flags(gst::BufferFlags::DELTA_UNIT);
    } else {
        buffer_ref.set_flags(gst::BufferFlags::DELTA_UNIT);
    }
}
```

## Zero-Copy

### Apple (IOSurface)

VideoToolbox outputs IOSurface-backed buffers. Convert to Metal texture:
```rust
// See platform/apple/compositor/src/surface.rs for IOSurface → Metal texture
```

### Jetson (DMA-BUF)

NVDEC outputs DMA-BUF file descriptors for zero-copy Vulkan import:
```rust
#[cfg(unix)]
let has_dma_buf = buffer.meta::<gstreamer::meta::ParentBufferMeta>().is_some();
```

## Troubleshooting

- `vtdec_hw` not found → Install GStreamer with VideoToolbox support
- `nvv4l2decoder` not found → Install Jetson multimedia plugins
- Pipeline stuck → Check appsrc caps match decoder expectations
- Memory leak → Ensure pipeline set to Null state on drop
