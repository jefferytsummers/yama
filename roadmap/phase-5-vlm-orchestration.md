# Phase 5: Multi-Stage VLM Orchestration

**Duration:** 26 days
**Goal:** Intelligent multi-pass video analysis with model routing based on video characteristics.

---

## Overview

This is the core intelligence phase. The system first characterizes each video stream (scene type, lighting, motion, content), then selects appropriate detection and VLM models. Multiple models run in parallel or sequentially based on triggers, and results are aggregated into a searchable context store.

---

## Architecture: Meta-VLM Model Router

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    MULTI-STAGE VLM PIPELINE                             │
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  STAGE 1: VIDEO CHARACTERIZATION                                │   │
│  │                                                                  │   │
│  │  Input: Sample frames (first 5 seconds + periodic samples)      │   │
│  │                                                                  │   │
│  │  Outputs:                                                        │   │
│  │  • Scene Type: indoor/outdoor/mixed                             │   │
│  │  • Environment: office/warehouse/retail/street/home             │   │
│  │  • Lighting: day/night/artificial/mixed                         │   │
│  │  • Motion Level: static/slow/fast/chaotic                       │   │
│  │  • Content Hints: people/vehicles/text/machinery                │   │
│  │  • Quality: resolution, noise, blur assessment                  │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                              │                                          │
│                              ▼                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  STAGE 2: MODEL ROUTER                                          │   │
│  │                                                                  │   │
│  │  Detection Models: YOLOv8n, YOLO-World, RT-DETR, License Plate  │   │
│  │  VLM Models: Qwen2-VL-3B (fast), Qwen2-VL-7B (detailed)        │   │
│  │  Specialized: OCR, Pose Estimation, Action Recognition          │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                              │                                          │
│                              ▼                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  STAGE 3: PARALLEL ANALYSIS EXECUTION                           │   │
│  │                                                                  │   │
│  │  • Run selected detection models continuously                   │   │
│  │  • Trigger VLM analysis based on events/schedule                │   │
│  │  • Run specialized models on-demand                             │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                              │                                          │
│                              ▼                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  STAGE 4: RESULT SYNTHESIS                                       │   │
│  │                                                                  │   │
│  │  • Merge detections from multiple models                        │   │
│  │  • Correlate VLM descriptions with detections                   │   │
│  │  • Build temporal narrative                                      │   │
│  │  • Store in searchable context                                   │   │
│  └─────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Milestone 5.1: Video Characterization VLM

**Duration:** 4 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 5.1.1 | Define `VideoCharacteristics` proto | Comprehensive video properties | Compiles, covers all properties |
| 5.1.2 | Create characterization prompt | Structured JSON output | Returns structured JSON |
| 5.1.3 | Implement sample frame extraction | 5-10 frames from first 5s + periodic | Frames extracted |
| 5.1.4 | Parse VLM response to proto | Structured output | Parsing works |
| 5.1.5 | Cache characteristics per source | Avoid re-analysis | Cache hit rate >90% |

### Protocol: Video Characteristics

```protobuf
// shared/protocol/proto/vlm.proto (extended)

// Video characteristics from meta-VLM analysis
message VideoCharacteristics {
    string video_id = 1;

    // Scene properties
    SceneType scene_type = 2;
    Environment environment = 3;
    LightingCondition lighting = 4;

    // Motion analysis
    MotionLevel motion_level = 5;
    CameraType camera_type = 6;

    // Content hints (what's likely in the video)
    repeated ContentHint content_hints = 7;
    float people_likelihood = 8;
    float vehicle_likelihood = 9;
    float text_likelihood = 10;

    // Quality assessment
    QualityAssessment quality = 11;

    // Recommended analysis strategy
    AnalysisStrategy recommended_strategy = 12;
}

enum SceneType {
    SCENE_INDOOR = 0;
    SCENE_OUTDOOR = 1;
    SCENE_MIXED = 2;
}

enum Environment {
    ENV_OFFICE = 0;
    ENV_WAREHOUSE = 1;
    ENV_RETAIL = 2;
    ENV_STREET = 3;
    ENV_PARKING = 4;
    ENV_HOME = 5;
    ENV_INDUSTRIAL = 6;
    ENV_TRANSIT = 7;
    ENV_OTHER = 8;
}

enum LightingCondition {
    LIGHTING_DAYLIGHT = 0;
    LIGHTING_NIGHT = 1;
    LIGHTING_ARTIFICIAL = 2;
    LIGHTING_MIXED = 3;
    LIGHTING_LOW_LIGHT = 4;
}

enum MotionLevel {
    MOTION_STATIC = 0;
    MOTION_SLOW = 1;
    MOTION_MODERATE = 2;
    MOTION_FAST = 3;
    MOTION_CHAOTIC = 4;
}

enum CameraType {
    CAMERA_FIXED = 0;
    CAMERA_PTZ = 1;
    CAMERA_HANDHELD = 2;
    CAMERA_DRONE = 3;
    CAMERA_VEHICLE = 4;
}

message ContentHint {
    string category = 1;
    float confidence = 2;
    string detail = 3;
}

message QualityAssessment {
    Resolution resolution = 1;
    float noise_level = 2;
    float blur_level = 3;
    float compression_artifacts = 4;
    bool suitable_for_detection = 5;
    bool suitable_for_recognition = 6;
}

message AnalysisStrategy {
    string primary_detector = 1;
    repeated string specialized_detectors = 2;
    string primary_vlm = 3;
    repeated TriggerCondition vlm_triggers = 4;
    repeated string optional_models = 5;
    uint32 detection_interval_frames = 6;
    uint32 vlm_sample_interval_ms = 7;
    float confidence_threshold = 8;
}

message TriggerCondition {
    TriggerType type = 1;
    string parameter = 2;
    uint32 threshold = 3;
}

enum TriggerType {
    TRIGGER_PERIODIC = 0;
    TRIGGER_ON_MOTION = 1;
    TRIGGER_ON_NEW_TRACK = 2;
    TRIGGER_ON_CLASS = 3;
    TRIGGER_ON_COUNT = 4;
    TRIGGER_ON_QUERY = 5;
}
```

### Code: Characterization Prompt

```rust
// ai-server/src/orchestrator/characterization.rs

const CHARACTERIZATION_PROMPT: &str = r#"
Analyze this video sample and provide a structured JSON assessment:

{
  "scene_type": "indoor" | "outdoor" | "mixed",
  "environment": "office" | "warehouse" | "retail" | "street" | "parking" | "home" | "industrial" | "transit" | "other",
  "lighting": "daylight" | "night" | "artificial" | "mixed" | "low_light",
  "motion_level": "static" | "slow" | "moderate" | "fast" | "chaotic",
  "camera_type": "fixed" | "ptz" | "handheld" | "drone" | "vehicle",
  "content_hints": [
    {"category": "string", "confidence": 0.0-1.0, "detail": "string"}
  ],
  "people_likelihood": 0.0-1.0,
  "vehicle_likelihood": 0.0-1.0,
  "text_likelihood": 0.0-1.0,
  "quality": {
    "noise_level": 0.0-1.0,
    "blur_level": 0.0-1.0,
    "suitable_for_detection": true | false,
    "suitable_for_recognition": true | false
  }
}

Based on this assessment, I will automatically determine:
- Primary detection model
- Specialized detectors needed
- Appropriate VLM for queries
- Trigger conditions for analysis
- Optimal sampling intervals
"#;

pub struct Characterizer {
    vlm: Arc<dyn VlmModel>,
    cache: RwLock<HashMap<String, VideoCharacteristics>>,
}

impl Characterizer {
    pub async fn characterize(
        &self,
        source_id: &str,
        frames: &[Frame],
    ) -> Result<VideoCharacteristics> {
        // Check cache
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(source_id) {
                return Ok(cached.clone());
            }
        }

        // Run characterization VLM
        let response = self.vlm
            .analyze_frames(frames, CHARACTERIZATION_PROMPT)
            .await?;

        // Parse JSON response
        let characteristics = self.parse_response(&response)?;

        // Determine strategy
        let strategy = self.determine_strategy(&characteristics);
        let mut characteristics = characteristics;
        characteristics.recommended_strategy = Some(strategy);

        // Cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(source_id.to_string(), characteristics.clone());
        }

        Ok(characteristics)
    }

    fn determine_strategy(&self, chars: &VideoCharacteristics) -> AnalysisStrategy {
        let mut strategy = AnalysisStrategy::default();

        // Select primary detector
        strategy.primary_detector = "yolov8n".to_string();

        // Add specialized detectors based on content hints
        if chars.vehicle_likelihood > 0.7
            && matches!(chars.environment, Environment::Parking | Environment::Street)
        {
            strategy.specialized_detectors.push("license_plate".to_string());
        }

        if chars.people_likelihood > 0.7
            && matches!(chars.environment, Environment::Industrial)
        {
            strategy.specialized_detectors.push("ppe_detector".to_string());
        }

        if chars.text_likelihood > 0.5 {
            strategy.optional_models.push("ocr".to_string());
        }

        // Select VLM based on motion and detail needs
        strategy.primary_vlm = match chars.motion_level {
            MotionLevel::Fast | MotionLevel::Chaotic => "vila_2b".to_string(),  // Fast
            _ => "qwen2_vl_7b".to_string(),  // Detailed
        };

        // Set triggers
        strategy.vlm_triggers.push(TriggerCondition {
            r#type: TriggerType::OnNewTrack as i32,
            parameter: String::new(),
            threshold: 0,
        });

        strategy.vlm_triggers.push(TriggerCondition {
            r#type: TriggerType::Periodic as i32,
            parameter: String::new(),
            threshold: 30,  // 30 seconds
        });

        // Set intervals based on motion
        strategy.detection_interval_frames = match chars.motion_level {
            MotionLevel::Static => 5,
            MotionLevel::Slow => 2,
            _ => 1,
        };

        strategy.vlm_sample_interval_ms = match chars.motion_level {
            MotionLevel::Fast | MotionLevel::Chaotic => 2000,
            MotionLevel::Moderate => 5000,
            _ => 10000,
        };

        strategy
    }
}
```

### Verification

```bash
cargo run --example characterize -- --video test.mp4
# Output: VideoCharacteristics { scene: OUTDOOR, environment: PARKING, ... }
```

### Deliverable

`ai-server/src/orchestrator/characterization.rs`

---

## Milestone 5.2: Model Router

**Duration:** 5 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 5.2.1 | Define model registry | List available models with metadata | Registry populated |
| 5.2.2 | Implement strategy selection | Characteristics to strategy mapping | Correct mapping |
| 5.2.3 | GPU memory management | Check memory before loading | Memory checked |
| 5.2.4 | Dynamic model loading | Load via Triton API | Models loaded |
| 5.2.5 | Model unloading for memory | Evict LRU when needed | Memory freed |
| 5.2.6 | Pipeline configuration builder | Generate PipelineConfig | Config generated |

### Code: Model Router

```rust
// ai-server/src/orchestrator/model_router.rs

pub struct ModelRouter {
    characterization_vlm: Arc<dyn VlmModel>,
    detectors: HashMap<String, Arc<dyn DetectionModel>>,
    vlms: HashMap<String, Arc<dyn VlmModel>>,
    specialized: HashMap<String, Arc<dyn SpecializedModel>>,
    triton: TritonClient,
    loaded_models: RwLock<HashSet<String>>,
    model_specs: HashMap<String, ModelSpec>,
}

pub struct ModelSpec {
    pub name: String,
    pub model_type: ModelType,
    pub memory_required: u64,
    pub priority: u32,  // Higher = more important to keep loaded
}

pub enum ModelType {
    Detector,
    Vlm,
    Specialized,
}

impl ModelRouter {
    /// Characterize video and configure analysis pipeline.
    pub async fn configure_pipeline(
        &self,
        source_id: &str,
        sample_frames: &[Frame],
    ) -> Result<PipelineConfig> {
        // Stage 1: Characterize
        let characteristics = self.characterize_video(sample_frames).await?;

        let strategy = characteristics.recommended_strategy.as_ref()
            .ok_or_else(|| anyhow!("No strategy determined"))?;

        // Stage 2: Load models
        self.ensure_model_loaded(&strategy.primary_detector).await?;

        for detector in &strategy.specialized_detectors {
            self.ensure_model_loaded(detector).await?;
        }

        self.ensure_model_loaded(&strategy.primary_vlm).await?;

        // Build config
        Ok(PipelineConfig {
            source_id: source_id.to_string(),
            characteristics,
            active_detectors: self.get_active_detectors(strategy),
            active_vlm: strategy.primary_vlm.clone(),
            triggers: strategy.vlm_triggers.clone(),
            detection_interval: strategy.detection_interval_frames,
            vlm_interval_ms: strategy.vlm_sample_interval_ms,
        })
    }

    async fn ensure_model_loaded(&self, model_id: &str) -> Result<()> {
        {
            let loaded = self.loaded_models.read().await;
            if loaded.contains(model_id) {
                return Ok(());
            }
        }

        // Check GPU memory
        let memory = self.triton.get_gpu_memory().await?;
        let spec = self.model_specs.get(model_id)
            .ok_or_else(|| anyhow!("Unknown model: {}", model_id))?;

        if memory.available < spec.memory_required {
            self.free_memory_for(spec.memory_required).await?;
        }

        // Load model
        tracing::info!(model = %model_id, "Loading model");
        self.triton.load_model(model_id).await?;

        let mut loaded = self.loaded_models.write().await;
        loaded.insert(model_id.to_string());

        Ok(())
    }

    async fn free_memory_for(&self, required: u64) -> Result<()> {
        let loaded = self.loaded_models.read().await;
        let mut candidates: Vec<_> = loaded.iter()
            .filter_map(|name| {
                self.model_specs.get(name).map(|spec| (name, spec))
            })
            .filter(|(_, spec)| spec.model_type == ModelType::Vlm)  // Prefer unloading VLMs
            .collect();

        // Sort by priority (lowest first)
        candidates.sort_by_key(|(_, spec)| spec.priority);
        drop(loaded);

        let mut freed = 0u64;
        for (name, spec) in candidates {
            if freed >= required {
                break;
            }

            tracing::info!(model = %name, "Unloading model to free memory");
            self.triton.unload_model(name).await?;

            let mut loaded = self.loaded_models.write().await;
            loaded.remove(name);
            freed += spec.memory_required;
        }

        if freed < required {
            return Err(anyhow!("Could not free enough memory"));
        }

        Ok(())
    }
}
```

### Verification

```bash
cargo run --example route_models -- --video parking.mp4
# Output: Loading yolov8n, license_plate_detector, qwen2-vl-7b
#         Pipeline configured: detection_interval=2, vlm_interval=5000ms
```

### Deliverable

`ai-server/src/orchestrator/model_router.rs`

---

## Milestone 5.3: Multi-Model Execution Engine

**Duration:** 4 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 5.3.1 | Parallel model execution | Run multiple detectors concurrently | Concurrent execution |
| 5.3.2 | Result merging | Combine detections from multiple models | Correct merge |
| 5.3.3 | Trigger evaluation | Check trigger conditions | Triggers fire correctly |
| 5.3.4 | VLM scheduling | Queue VLM requests with priority | Priority ordering |
| 5.3.5 | Specialized model invocation | On-demand OCR/pose/action | On-demand works |

### Code: Execution Engine

```rust
// ai-server/src/orchestrator/execution_engine.rs

pub struct ExecutionEngine {
    config: PipelineConfig,
    triton: TritonClient,
    context_store: Arc<ContextStore>,
    vlm_queue: Arc<VlmQueue>,
}

impl ExecutionEngine {
    /// Process a frame through the pipeline.
    pub async fn process_frame(&self, frame: &Frame) -> Result<FrameResult> {
        let start = Instant::now();

        // Run detectors in parallel
        let detection_futures: Vec<_> = self.config.active_detectors.iter()
            .map(|detector| {
                let triton = self.triton.clone();
                let frame = frame.clone();
                async move {
                    triton.infer_detection(&detector, &frame).await
                }
            })
            .collect();

        let detection_results = futures::future::join_all(detection_futures).await;

        // Merge detections
        let merged_detections = self.merge_detections(detection_results)?;

        // Evaluate triggers
        for trigger in &self.config.triggers {
            if self.should_trigger(trigger, &merged_detections) {
                self.schedule_vlm_analysis(frame, &merged_detections).await?;
                break;
            }
        }

        // Store in context
        self.context_store.store_detections(
            &frame.source_id,
            frame.frame_number,
            &merged_detections,
        ).await?;

        Ok(FrameResult {
            frame_number: frame.frame_number,
            detections: merged_detections,
            processing_time: start.elapsed(),
        })
    }

    fn merge_detections(
        &self,
        results: Vec<Result<Vec<Detection>>>,
    ) -> Result<Vec<Detection>> {
        let mut all_detections = Vec::new();

        for result in results {
            if let Ok(detections) = result {
                all_detections.extend(detections);
            }
        }

        // NMS across all detections
        let merged = non_max_suppression(&all_detections, 0.5);

        Ok(merged)
    }

    fn should_trigger(
        &self,
        trigger: &TriggerCondition,
        detections: &[Detection],
    ) -> bool {
        match TriggerType::from_i32(trigger.r#type) {
            Some(TriggerType::OnNewTrack) => {
                detections.iter().any(|d| d.is_new_track)
            }
            Some(TriggerType::OnClass) => {
                detections.iter().any(|d| d.class_name == trigger.parameter)
            }
            Some(TriggerType::OnCount) => {
                detections.len() as u32 > trigger.threshold
            }
            _ => false,
        }
    }

    async fn schedule_vlm_analysis(
        &self,
        frame: &Frame,
        detections: &[Detection],
    ) -> Result<()> {
        let request = VlmRequest {
            frame: frame.clone(),
            context_detections: detections.to_vec(),
            priority: VlmPriority::Normal,
        };

        self.vlm_queue.enqueue(request).await
    }
}

pub struct VlmQueue {
    queue: RwLock<BinaryHeap<VlmRequest>>,
    triton: TritonClient,
    context_store: Arc<ContextStore>,
    rate_limiter: RateLimiter,
}

impl VlmQueue {
    pub async fn process_loop(self: Arc<Self>) {
        loop {
            // Rate limit: max 1 request per 5 seconds per source
            if let Some(request) = self.dequeue().await {
                if !self.rate_limiter.check(&request.frame.source_id) {
                    continue;
                }

                let response = self.triton
                    .infer_vlm(&request.frame, &self.build_prompt(&request))
                    .await;

                if let Ok(response) = response {
                    self.context_store.store_vlm_response(
                        &request.frame.source_id,
                        request.frame.frame_number,
                        &response,
                    ).await.ok();
                }
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}
```

### Verification

```bash
cargo run --example multi_model -- --video industrial.mp4
# Output: Running yolov8n + ppe_detector + pose_estimator
#         VLM triggered by: person without helmet
```

### Deliverable

`ai-server/src/orchestrator/execution_engine.rs`

---

## Milestone 5.4: Detection Router & Aggregation

**Duration:** 3 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 5.4.1 | Subscribe to DeepStream detections | Receives all events | Events received |
| 5.4.2 | Implement filtering logic | Filter by class, confidence | Filtering works |
| 5.4.3 | Publish filtered detections | Only relevant forwarded | Correct filtering |
| 5.4.4 | Add detection aggregation | Combine across streams | Aggregation works |

### Code: Detection Router

```rust
// ai-server/src/orchestrator/detection_router.rs

pub struct DetectionRouter {
    event_bus: EventBusClient,
    filters: HashMap<String, DetectionFilter>,
    output_tx: broadcast::Sender<FilteredDetection>,
}

pub struct DetectionFilter {
    pub classes: Option<HashSet<String>>,
    pub min_confidence: f32,
    pub min_track_age: u32,
}

impl DetectionRouter {
    pub async fn run(&self) -> Result<()> {
        // Subscribe to all detection topics
        self.event_bus.subscribe(&["detection.*"]).await?;

        while let Some(envelope) = self.event_bus.recv().await {
            if let Ok(detections) = decode::<StreamDetections>(&envelope.payload) {
                let filtered = self.filter(&detections);
                if !filtered.is_empty() {
                    let _ = self.output_tx.send(FilteredDetection {
                        source_id: detections.source_id,
                        frame_number: detections.frame_number,
                        detections: filtered,
                    });
                }
            }
        }

        Ok(())
    }

    fn filter(&self, detections: &StreamDetections) -> Vec<Detection> {
        let filter = self.filters.get(&detections.source_id)
            .unwrap_or(&DetectionFilter::default());

        detections.detections.iter()
            .filter(|d| d.confidence >= filter.min_confidence)
            .filter(|d| {
                filter.classes.as_ref()
                    .map(|c| c.contains(&d.class_name))
                    .unwrap_or(true)
            })
            .cloned()
            .collect()
    }
}
```

### Verification

```bash
cargo run --example filtered_detections -- --class person
# Only person detections published
```

### Deliverable

`ai-server/src/orchestrator/detection_router.rs`

---

## Milestone 5.5: VLM Trigger System

**Duration:** 4 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 5.5.1 | Event-based triggers | Trigger on new track | Fires on new track |
| 5.5.2 | Periodic triggers | Every N seconds per source | Fires periodically |
| 5.5.3 | On-demand triggers | From client query | Fires on query |
| 5.5.4 | Rate limiting | Max 1 request/5s per source | Rate limited |
| 5.5.5 | Priority queue | Urgent requests first | Priority ordering |

### Code: VLM Trigger

```rust
// ai-server/src/orchestrator/vlm_trigger.rs

pub struct VlmTriggerSystem {
    triggers: Vec<TriggerCondition>,
    last_trigger: RwLock<HashMap<String, Instant>>,
    min_interval: Duration,
    queue: Arc<VlmQueue>,
}

impl VlmTriggerSystem {
    pub async fn evaluate(
        &self,
        source_id: &str,
        frame: &Frame,
        detections: &[Detection],
    ) -> Result<bool> {
        // Check rate limit
        {
            let last = self.last_trigger.read().await;
            if let Some(last_time) = last.get(source_id) {
                if last_time.elapsed() < self.min_interval {
                    return Ok(false);
                }
            }
        }

        // Check each trigger
        for trigger in &self.triggers {
            let fired = match TriggerType::from_i32(trigger.r#type) {
                Some(TriggerType::OnNewTrack) => {
                    detections.iter().any(|d| {
                        d.track_state == Some(TrackState::New as i32)
                    })
                }
                Some(TriggerType::OnClass) => {
                    detections.iter().any(|d| d.class_name == trigger.parameter)
                }
                Some(TriggerType::OnCount) => {
                    detections.len() as u32 > trigger.threshold
                }
                Some(TriggerType::OnMotion) => {
                    self.detect_significant_motion(detections)
                }
                _ => false,
            };

            if fired {
                // Update last trigger time
                {
                    let mut last = self.last_trigger.write().await;
                    last.insert(source_id.to_string(), Instant::now());
                }

                // Queue VLM request
                self.queue.enqueue(VlmRequest {
                    frame: frame.clone(),
                    context_detections: detections.to_vec(),
                    priority: VlmPriority::Normal,
                    trigger_reason: format!("{:?}", trigger.r#type),
                }).await?;

                tracing::info!(
                    source = %source_id,
                    trigger = ?trigger.r#type,
                    "VLM triggered"
                );

                return Ok(true);
            }
        }

        Ok(false)
    }

    fn detect_significant_motion(&self, detections: &[Detection]) -> bool {
        // Check if any tracked object has significant velocity
        detections.iter().any(|d| {
            let velocity = (d.velocity_x.powi(2) + d.velocity_y.powi(2)).sqrt();
            velocity > 50.0  // pixels per frame
        })
    }
}
```

### Deliverable

`ai-server/src/orchestrator/vlm_trigger.rs`

---

## Milestone 5.6: Context Store

**Duration:** 3 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 5.6.1 | Ring buffer storage | Fixed-size, FIFO | Correct eviction |
| 5.6.2 | Store detection history | Query by time range | Queries work |
| 5.6.3 | Store VLM descriptions | Query by source/time | Queries work |
| 5.6.4 | Semantic search (optional) | Embed for similarity | Similarity works |

### Code: Context Store

```rust
// ai-server/src/orchestrator/context_store.rs

pub struct ContextStore {
    detections: RwLock<HashMap<String, VecDeque<DetectionRecord>>>,
    vlm_responses: RwLock<HashMap<String, VecDeque<VlmRecord>>>,
    max_records_per_source: usize,
    retention: Duration,
}

pub struct DetectionRecord {
    pub frame_number: u64,
    pub timestamp: Instant,
    pub detections: Vec<Detection>,
}

pub struct VlmRecord {
    pub frame_number: u64,
    pub timestamp: Instant,
    pub prompt: String,
    pub response: String,
    pub trigger_reason: String,
}

impl ContextStore {
    pub async fn store_detections(
        &self,
        source_id: &str,
        frame_number: u64,
        detections: &[Detection],
    ) -> Result<()> {
        let mut store = self.detections.write().await;
        let queue = store.entry(source_id.to_string()).or_default();

        queue.push_back(DetectionRecord {
            frame_number,
            timestamp: Instant::now(),
            detections: detections.to_vec(),
        });

        // Evict old records
        while queue.len() > self.max_records_per_source {
            queue.pop_front();
        }

        Ok(())
    }

    pub async fn query_detections(
        &self,
        source_id: &str,
        time_range: Duration,
        class_filter: Option<&[String]>,
    ) -> Result<Vec<Detection>> {
        let store = self.detections.read().await;
        let queue = store.get(source_id).ok_or(anyhow!("Unknown source"))?;

        let cutoff = Instant::now() - time_range;

        let detections: Vec<_> = queue.iter()
            .filter(|r| r.timestamp > cutoff)
            .flat_map(|r| &r.detections)
            .filter(|d| {
                class_filter
                    .map(|classes| classes.contains(&d.class_name))
                    .unwrap_or(true)
            })
            .cloned()
            .collect();

        Ok(detections)
    }

    pub async fn query_vlm_history(
        &self,
        source_id: &str,
        time_range: Duration,
    ) -> Result<Vec<VlmRecord>> {
        let store = self.vlm_responses.read().await;
        let queue = store.get(source_id).ok_or(anyhow!("Unknown source"))?;

        let cutoff = Instant::now() - time_range;

        let records: Vec<_> = queue.iter()
            .filter(|r| r.timestamp > cutoff)
            .cloned()
            .collect();

        Ok(records)
    }

    /// Build context for a VLM query.
    pub async fn build_context(
        &self,
        source_id: &str,
        context_window: Duration,
    ) -> Result<QueryContext> {
        let recent_detections = self.query_detections(
            source_id,
            context_window,
            None,
        ).await?;

        let recent_descriptions = self.query_vlm_history(
            source_id,
            context_window,
        ).await?;

        Ok(QueryContext {
            recent_detections,
            recent_descriptions,
        })
    }
}
```

### Verification

```bash
cargo test -p yama-ai-server -- context_store
```

### Deliverable

`ai-server/src/orchestrator/context_store.rs`

---

## Milestone 5.7: Query Handler

**Duration:** 3 days

### Tasks

| ID | Task | Description | Test Criteria |
|----|------|-------------|---------------|
| 5.7.1 | Parse `VideoQueryRequest` | Extract intent | Intent extracted |
| 5.7.2 | Gather context | Recent detections/descriptions | Context gathered |
| 5.7.3 | Build VLM prompt | Include context | Prompt built |
| 5.7.4 | Execute VLM inference | Get response | Response received |
| 5.7.5 | Format response | Include frame references | Response formatted |

### Code: Query Handler

```rust
// ai-server/src/orchestrator/query_handler.rs

pub struct QueryHandler {
    triton: TritonClient,
    context_store: Arc<ContextStore>,
    video_provider: Arc<dyn VideoProvider>,
}

impl QueryHandler {
    pub async fn handle_query(
        &self,
        request: VideoQueryRequest,
    ) -> Result<VideoQueryResponse> {
        // Get current frame
        let frame = self.video_provider
            .get_frame(&request.source_ids[0])
            .await?;

        // Gather context
        let context = if request.context.include_recent_detections
            || request.context.include_recent_descriptions
        {
            Some(self.context_store.build_context(
                &request.source_ids[0],
                Duration::from_secs(request.context.context_window_seconds as u64),
            ).await?)
        } else {
            None
        };

        // Build prompt with context
        let prompt = self.build_prompt(&request.query, context.as_ref());

        // Run VLM
        let vlm_response = self.triton.infer_vlm(
            &frame.to_image()?,
            &prompt,
            Some(512),
            Some(0.7),
        ).await?;

        Ok(VideoQueryResponse {
            request_id: request.request_id,
            analysis: vlm_response.text,
            frame_reference: Some(FrameReference {
                source_id: frame.source_id,
                frame_number: frame.frame_number,
                timestamp: frame.timestamp,
            }),
            context_used: context.map(|c| ContextSummary {
                detection_count: c.recent_detections.len() as u32,
                description_count: c.recent_descriptions.len() as u32,
            }),
        })
    }

    fn build_prompt(&self, query: &str, context: Option<&QueryContext>) -> String {
        let mut prompt = String::new();

        if let Some(ctx) = context {
            if !ctx.recent_detections.is_empty() {
                prompt.push_str("Recent detections in this scene:\n");
                for det in &ctx.recent_detections {
                    prompt.push_str(&format!(
                        "- {} (confidence: {:.2})\n",
                        det.class_name, det.confidence
                    ));
                }
                prompt.push('\n');
            }

            if !ctx.recent_descriptions.is_empty() {
                prompt.push_str("Recent scene descriptions:\n");
                for desc in ctx.recent_descriptions.iter().rev().take(3) {
                    prompt.push_str(&format!("- {}\n", desc.response));
                }
                prompt.push('\n');
            }
        }

        prompt.push_str("User query: ");
        prompt.push_str(query);

        prompt
    }
}
```

### Verification

```bash
cargo run --example query_client -- "What vehicles?"
# Response includes analysis + context
```

### Deliverable

`ai-server/src/orchestrator/query_handler.rs`

---

## Dependencies

- Phase 1 (Abstraction Layer) - provider traits
- Phase 3 (DeepStream) - detection stream
- Phase 4 (Triton) - model serving

## Blocks

- Phase 7 (Tool System) - needs orchestration
- Phase 8 (E2E Integration) - core component

---

## Use Case Examples

**Security Camera in Parking Lot**
```
Characterization: outdoor, parking, daylight, slow motion, vehicles 0.9
Strategy: YOLOv8n + license_plate, Qwen2-VL-7B, trigger on new track
Result: "Person in blue jacket approaching silver Toyota Camry, plate ABC-1234"
```

**Industrial Safety Monitoring**
```
Characterization: indoor, industrial, artificial, moderate motion, people 0.8
Strategy: YOLOv8n + PPE + pose, Qwen2-VL-3B, trigger on person detection
Result: "Worker without hard hat in zone A, unsafe posture detected"
```

---

## Checklist

- [ ] 5.1.1 Define `VideoCharacteristics` proto
- [ ] 5.1.2 Create characterization prompt
- [ ] 5.1.3 Implement sample frame extraction
- [ ] 5.1.4 Parse VLM response to proto
- [ ] 5.1.5 Cache characteristics per source
- [ ] 5.2.1 Define model registry
- [ ] 5.2.2 Implement strategy selection
- [ ] 5.2.3 GPU memory management
- [ ] 5.2.4 Dynamic model loading
- [ ] 5.2.5 Model unloading for memory
- [ ] 5.2.6 Pipeline configuration builder
- [ ] 5.3.1 Parallel model execution
- [ ] 5.3.2 Result merging
- [ ] 5.3.3 Trigger evaluation
- [ ] 5.3.4 VLM scheduling
- [ ] 5.3.5 Specialized model invocation
- [ ] 5.4.1 Subscribe to DeepStream detections
- [ ] 5.4.2 Implement filtering logic
- [ ] 5.4.3 Publish filtered detections
- [ ] 5.4.4 Add detection aggregation
- [ ] 5.5.1 Event-based triggers
- [ ] 5.5.2 Periodic triggers
- [ ] 5.5.3 On-demand triggers
- [ ] 5.5.4 Rate limiting
- [ ] 5.5.5 Priority queue
- [ ] 5.6.1 Ring buffer storage
- [ ] 5.6.2 Store detection history
- [ ] 5.6.3 Store VLM descriptions
- [ ] 5.6.4 Semantic search (optional)
- [ ] 5.7.1 Parse `VideoQueryRequest`
- [ ] 5.7.2 Gather context
- [ ] 5.7.3 Build VLM prompt
- [ ] 5.7.4 Execute VLM inference
- [ ] 5.7.5 Format response
