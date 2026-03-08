# Phase 6: Video Streaming

**Duration:** 11 days
**Goal:** Low-latency video delivery to remote clients.

---

## Overview

This phase implements video streaming from Jetson to remote clients. WebRTC provides low-latency streaming for interactive use cases, while HLS serves as a fallback for broader compatibility. Video streams include optional detection overlays rendered server-side.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Jetson AI Server                                                       │
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  DeepStream Pipeline                                             │   │
│  │  (H.264 encoded output)                                         │   │
│  └───────────────────────────────────┬─────────────────────────────┘   │
│                                      │                                  │
│         ┌────────────────────────────┼────────────────────────────┐    │
│         ▼                            ▼                            ▼    │
│  ┌─────────────┐            ┌─────────────────┐         ┌─────────┐   │
│  │ WebRTC      │            │ Detection       │         │ HLS     │   │
│  │ Server      │            │ Overlay         │         │ Server  │   │
│  │ (low lat)   │            │ Renderer        │         │ (compat)│   │
│  └──────┬──────┘            └─────────────────┘         └────┬────┘   │
│         │                                                     │        │
│  ┌──────┴──────┐                                       ┌──────┴──────┐│
│  │ STUN/TURN   │                                       │ HTTP Server ││
│  │ Signaling   │                                       │ /hls/...    ││
│  └─────────────┘                                       └─────────────┘│
└─────────────────────────────────────────────────────────────────────────┘
           │                                                     │
           │ WebRTC (UDP)                                        │ HLS (HTTP)
           │ <200ms latency                                      │ 8-12s latency
           ▼                                                     ▼
┌─────────────────┐                                   ┌─────────────────┐
│   Mac Client    │                                   │   Web Browser   │
│   (egui)        │                                   │   (SvelteKit)   │
└─────────────────┘                                   └─────────────────┘
```

---

## Protocol Selection

| Client | Protocol | Rationale |
|--------|----------|-----------|
| Mac (egui) | WebRTC | Low latency, native support |
| Web (SvelteKit) | WebRTC + HLS fallback | Browser compatibility |
| Mobile | HLS | Battery efficiency |

---

## Milestone 6.1: WebRTC Server

**Duration:** 5 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 6.1.1 | STUN/TURN setup | NAT traversal | Connection from NAT works |
| 6.1.2 | SDP offer/answer | Via event bus signaling | Signaling completes |
| 6.1.3 | Video track from DeepStream | H.264 RTP packaging | Video plays |
| 6.1.4 | Detection overlay (optional) | Boxes in stream | Overlays visible |

### Protocol: WebRTC Signaling

```protobuf
// shared/protocol/proto/streaming.proto

syntax = "proto3";
package yama.streaming;

// WebRTC signaling messages (via event bus)
message WebRtcOffer {
    string session_id = 1;
    string source_id = 2;
    string sdp = 3;  // SDP offer
}

message WebRtcAnswer {
    string session_id = 1;
    string source_id = 2;
    string sdp = 3;  // SDP answer
}

message IceCandidate {
    string session_id = 1;
    string source_id = 2;
    string candidate = 3;
    string sdp_mid = 4;
    int32 sdp_mline_index = 5;
}

// Stream configuration
message StreamRequest {
    string source_id = 1;
    StreamProtocol protocol = 2;
    StreamQuality quality = 3;
    bool include_detections = 4;
}

message StreamResponse {
    string source_id = 1;
    oneof stream {
        WebRtcOffer webrtc_offer = 2;
        string hls_url = 3;
    }
}

enum StreamProtocol {
    STREAM_WEBRTC = 0;
    STREAM_HLS = 1;
}

enum StreamQuality {
    QUALITY_AUTO = 0;
    QUALITY_HIGH = 1;   // 1080p
    QUALITY_MEDIUM = 2; // 720p
    QUALITY_LOW = 3;    // 480p
}
```

### Code: WebRTC Server

```rust
// ai-server/src/streaming/webrtc.rs

use webrtc::api::APIBuilder;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;

pub struct WebRtcServer {
    api: API,
    config: RTCConfiguration,
    connections: RwLock<HashMap<String, PeerConnection>>,
    video_tracks: HashMap<String, Arc<TrackLocalStaticRTP>>,
    event_bus: EventBusClient,
}

impl WebRtcServer {
    pub fn new(config: WebRtcConfig, event_bus: EventBusClient) -> Result<Self> {
        let mut media_engine = MediaEngine::default();
        media_engine.register_default_codecs()?;

        let api = APIBuilder::new()
            .with_media_engine(media_engine)
            .build();

        let rtc_config = RTCConfiguration {
            ice_servers: vec![
                RTCIceServer {
                    urls: vec!["stun:stun.l.google.com:19302".to_string()],
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        Ok(Self {
            api,
            config: rtc_config,
            connections: RwLock::new(HashMap::new()),
            video_tracks: HashMap::new(),
            event_bus,
        })
    }

    /// Handle incoming stream request.
    pub async fn handle_stream_request(
        &self,
        request: StreamRequest,
    ) -> Result<StreamResponse> {
        // Create peer connection
        let peer_connection = self.api
            .new_peer_connection(self.config.clone())
            .await?;

        // Add video track
        let video_track = self.video_tracks.get(&request.source_id)
            .ok_or_else(|| anyhow!("Unknown source: {}", request.source_id))?;

        peer_connection.add_track(video_track.clone()).await?;

        // Set up ICE candidate handler
        let session_id = uuid::Uuid::new_v4().to_string();
        let event_bus = self.event_bus.clone();
        let source_id = request.source_id.clone();

        peer_connection.on_ice_candidate(Box::new(move |candidate| {
            if let Some(c) = candidate {
                let ice = IceCandidate {
                    session_id: session_id.clone(),
                    source_id: source_id.clone(),
                    candidate: c.to_json().unwrap().candidate,
                    sdp_mid: c.sdp_mid.clone(),
                    sdp_mline_index: c.sdp_mline_index as i32,
                };
                let _ = event_bus.publish_sync(envelope(
                    "streaming.ice_candidate",
                    "webrtc-server",
                    ice,
                ));
            }
            Box::pin(async {})
        }));

        // Create offer
        let offer = peer_connection.create_offer(None).await?;
        peer_connection.set_local_description(offer.clone()).await?;

        // Store connection
        {
            let mut conns = self.connections.write().await;
            conns.insert(session_id.clone(), peer_connection);
        }

        Ok(StreamResponse {
            source_id: request.source_id,
            stream: Some(Stream::WebrtcOffer(WebRtcOffer {
                session_id,
                source_id: request.source_id,
                sdp: offer.sdp,
            })),
        })
    }

    /// Handle SDP answer from client.
    pub async fn handle_answer(&self, answer: WebRtcAnswer) -> Result<()> {
        let conns = self.connections.read().await;
        let conn = conns.get(&answer.session_id)
            .ok_or_else(|| anyhow!("Unknown session"))?;

        let remote_desc = RTCSessionDescription::answer(answer.sdp)?;
        conn.set_remote_description(remote_desc).await?;

        Ok(())
    }

    /// Handle ICE candidate from client.
    pub async fn handle_ice_candidate(&self, candidate: IceCandidate) -> Result<()> {
        let conns = self.connections.read().await;
        let conn = conns.get(&candidate.session_id)
            .ok_or_else(|| anyhow!("Unknown session"))?;

        let ice = RTCIceCandidateInit {
            candidate: candidate.candidate,
            sdp_mid: Some(candidate.sdp_mid),
            sdp_mline_index: Some(candidate.sdp_mline_index as u16),
            ..Default::default()
        };
        conn.add_ice_candidate(ice).await?;

        Ok(())
    }

    /// Feed video frames to track.
    pub async fn send_video_frame(
        &self,
        source_id: &str,
        rtp_packet: &[u8],
    ) -> Result<()> {
        if let Some(track) = self.video_tracks.get(source_id) {
            track.write(rtp_packet).await?;
        }
        Ok(())
    }
}
```

### Verification

```bash
# WebRTC latency < 200ms
# Run latency test with timestamp overlay
```

### Deliverable

`ai-server/src/streaming/webrtc.rs`

---

## Milestone 6.2: HLS Fallback

**Duration:** 3 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 6.2.1 | Segment generation | 4-second TS segments | Segments generated |
| 6.2.2 | M3U8 playlist server | Live playlist updates | Playlist updates |
| 6.2.3 | Segment cleanup | Configurable retention | Old segments deleted |

### Code: HLS Server

```rust
// ai-server/src/streaming/hls.rs

pub struct HlsServer {
    output_dir: PathBuf,
    segment_duration: Duration,
    playlist_length: u32,
    segments: RwLock<HashMap<String, VecDeque<Segment>>>,
}

pub struct Segment {
    pub filename: String,
    pub duration: f64,
    pub created_at: Instant,
}

impl HlsServer {
    pub fn new(output_dir: PathBuf, config: HlsConfig) -> Self {
        Self {
            output_dir,
            segment_duration: Duration::from_secs(config.segment_duration),
            playlist_length: config.playlist_length,
            segments: RwLock::new(HashMap::new()),
        }
    }

    /// Add a new segment for a source.
    pub async fn add_segment(
        &self,
        source_id: &str,
        segment_data: &[u8],
        duration: f64,
    ) -> Result<()> {
        let filename = format!(
            "{}/{}_{}.ts",
            source_id,
            chrono::Utc::now().timestamp_millis(),
            rand::random::<u32>()
        );

        // Write segment file
        let path = self.output_dir.join(&filename);
        tokio::fs::create_dir_all(path.parent().unwrap()).await?;
        tokio::fs::write(&path, segment_data).await?;

        // Update segment list
        let segment = Segment {
            filename,
            duration,
            created_at: Instant::now(),
        };

        {
            let mut segments = self.segments.write().await;
            let queue = segments.entry(source_id.to_string()).or_default();
            queue.push_back(segment);

            // Remove old segments
            while queue.len() > self.playlist_length as usize {
                if let Some(old) = queue.pop_front() {
                    let old_path = self.output_dir.join(&old.filename);
                    let _ = tokio::fs::remove_file(old_path).await;
                }
            }
        }

        // Update playlist
        self.update_playlist(source_id).await?;

        Ok(())
    }

    /// Generate M3U8 playlist.
    async fn update_playlist(&self, source_id: &str) -> Result<()> {
        let segments = self.segments.read().await;
        let queue = segments.get(source_id).ok_or(anyhow!("Unknown source"))?;

        let mut playlist = String::new();
        playlist.push_str("#EXTM3U\n");
        playlist.push_str("#EXT-X-VERSION:3\n");
        playlist.push_str(&format!(
            "#EXT-X-TARGETDURATION:{}\n",
            self.segment_duration.as_secs()
        ));
        playlist.push_str("#EXT-X-MEDIA-SEQUENCE:0\n");

        for segment in queue.iter() {
            playlist.push_str(&format!("#EXTINF:{:.3},\n", segment.duration));
            playlist.push_str(&segment.filename);
            playlist.push('\n');
        }

        let playlist_path = self.output_dir.join(format!("{}/playlist.m3u8", source_id));
        tokio::fs::write(playlist_path, playlist).await?;

        Ok(())
    }

    /// Get playlist URL for a source.
    pub fn playlist_url(&self, source_id: &str) -> String {
        format!("/hls/{}/playlist.m3u8", source_id)
    }
}
```

### HTTP Routes

```rust
// ai-server/src/http_server/routes.rs

pub fn hls_routes(hls_server: Arc<HlsServer>) -> Router {
    Router::new()
        .route("/hls/:source_id/playlist.m3u8", get(get_playlist))
        .route("/hls/:source_id/:segment", get(get_segment))
        .with_state(hls_server)
}

async fn get_playlist(
    Path(source_id): Path<String>,
    State(hls): State<Arc<HlsServer>>,
) -> impl IntoResponse {
    let path = hls.output_dir.join(format!("{}/playlist.m3u8", source_id));
    match tokio::fs::read_to_string(path).await {
        Ok(content) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "application/vnd.apple.mpegurl"),
                (header::CACHE_CONTROL, "no-cache"),
            ],
            content,
        ).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn get_segment(
    Path((source_id, segment)): Path<(String, String)>,
    State(hls): State<Arc<HlsServer>>,
) -> impl IntoResponse {
    let path = hls.output_dir.join(format!("{}/{}", source_id, segment));
    match tokio::fs::read(path).await {
        Ok(data) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "video/mp2t")],
            data,
        ).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}
```

### Verification

```bash
ffplay http://jetson:8080/hls/cam1/playlist.m3u8
# Plays with ~8-12s latency
```

### Deliverable

`ai-server/src/streaming/hls.rs`

---

## Milestone 6.3: Client Video Receiver

**Duration:** 3 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 6.3.1 | WebRTC client (Mac) | Connects, receives video | Video displays |
| 6.3.2 | Decode to egui texture | GPU texture | Smooth playback |
| 6.3.3 | HLS fallback | Auto-switch on failure | Fallback works |
| 6.3.4 | Detection overlay | Render boxes | Overlays visible |

### Code: Video Receiver (Mac)

```rust
// platform/apple/host/src/remote/video_receiver.rs

pub struct VideoReceiver {
    event_bus: EventBusClient,
    peer_connection: Option<RTCPeerConnection>,
    texture_handle: Arc<RwLock<Option<egui::TextureHandle>>>,
    decoder: VideoDecoder,
}

impl VideoReceiver {
    pub async fn connect(&mut self, source_id: &str) -> Result<()> {
        // Request WebRTC stream
        let request = StreamRequest {
            source_id: source_id.to_string(),
            protocol: StreamProtocol::Webrtc as i32,
            quality: StreamQuality::Auto as i32,
            include_detections: true,
        };

        self.event_bus.publish(envelope(
            "streaming.request",
            "mac-client",
            request,
        )).await?;

        // Wait for offer
        let offer = self.wait_for_offer(source_id).await?;

        // Create peer connection
        let api = APIBuilder::new().build();
        let config = RTCConfiguration::default();
        let pc = api.new_peer_connection(config).await?;

        // Set up track handler
        let texture = self.texture_handle.clone();
        let decoder = self.decoder.clone();

        pc.on_track(Box::new(move |track, _, _| {
            let texture = texture.clone();
            let decoder = decoder.clone();

            Box::pin(async move {
                // Read RTP packets and decode
                loop {
                    let (rtp, _) = track.read_rtp().await?;
                    if let Ok(frame) = decoder.decode(&rtp.payload).await {
                        // Update texture
                        let mut tex = texture.write().await;
                        // ... update egui texture
                    }
                }
            })
        }));

        // Set remote description (offer)
        pc.set_remote_description(RTCSessionDescription::offer(offer.sdp)?).await?;

        // Create answer
        let answer = pc.create_answer(None).await?;
        pc.set_local_description(answer.clone()).await?;

        // Send answer
        self.event_bus.publish(envelope(
            "streaming.answer",
            "mac-client",
            WebRtcAnswer {
                session_id: offer.session_id,
                source_id: source_id.to_string(),
                sdp: answer.sdp,
            },
        )).await?;

        self.peer_connection = Some(pc);

        Ok(())
    }

    pub fn render(&self, ui: &mut egui::Ui) {
        if let Ok(tex) = self.texture_handle.try_read() {
            if let Some(handle) = tex.as_ref() {
                ui.image(handle, handle.size_vec2());
            }
        }
    }
}
```

### Verification

```bash
cargo run -p yama-host-apple -- --config client.toml
# Video < 500ms latency, boxes overlay
```

### Deliverable

`platform/apple/host/src/remote/video_receiver.rs`

---

## Dependencies

- Phase 2 (Remote Event Bus) - for signaling
- Phase 3 (DeepStream) - for encoded video

## Blocks

- Phase 8 (E2E Integration) - video streaming required

---

## Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| WebRTC latency | <200ms | webrtc-stats |
| HLS latency | 8-12s | Segment timing |
| Bitrate (1080p) | 4 Mbps | Network stats |
| Bitrate (720p) | 2 Mbps | Network stats |

---

## Checklist

- [ ] 6.1.1 STUN/TURN setup
- [ ] 6.1.2 SDP offer/answer
- [ ] 6.1.3 Video track from DeepStream
- [ ] 6.1.4 Detection overlay (optional)
- [ ] 6.2.1 Segment generation
- [ ] 6.2.2 M3U8 playlist server
- [ ] 6.2.3 Segment cleanup
- [ ] 6.3.1 WebRTC client (Mac)
- [ ] 6.3.2 Decode to egui texture
- [ ] 6.3.3 HLS fallback
- [ ] 6.3.4 Detection overlay
