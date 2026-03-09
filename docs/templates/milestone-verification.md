# Milestone Verification Template

## Purpose

Every milestone must include a verification section that enables agents to confirm completion without human intervention. This template defines the standard structure.

---

## Template

```markdown
## Verification

### Automated Tests
- [ ] `cargo test -p {package} {test_pattern}` passes
- [ ] `cargo clippy -p {package}` shows no new warnings

### Demo Script
- [ ] `scripts/demos/milestone-{phase}.{milestone}.sh` runs without errors
- [ ] Output matches expected in `expected/milestone-{phase}.{milestone}.json`

### Acceptance Criteria
- [ ] AC-1: {specific testable criterion}
- [ ] AC-2: {specific testable criterion}
- [ ] AC-3: {specific testable criterion}
```

---

## Components

### 1. Automated Tests

All new code MUST have corresponding tests:

```markdown
### Automated Tests
- [ ] `cargo test -p yama-host-apple test_index_provider` passes
- [ ] `cargo test -p yama-host-apple test_batch_processor` passes
- [ ] `cargo clippy -p yama-host-apple` shows no new warnings
- [ ] `cargo fmt -p yama-host-apple -- --check` passes
```

**Rules:**
- Tests must be deterministic (no flaky tests)
- Tests must complete in <60 seconds (individually)
- Tests must not require external resources (network, specific files)

### 2. Demo Script

For milestones with observable behavior, provide a demo script:

```bash
#!/bin/bash
# scripts/demos/milestone-2.1.sh
# Demo: Key Frame Extraction

set -e  # Exit on error

VIDEO="${1:-~/Videos/sample.mp4}"

echo "Extracting key frames from: $VIDEO"
cargo run --example extract_keyframes -- "$VIDEO"

echo "Verifying output..."
# Count extracted frames
FRAME_COUNT=$(ls -1 /tmp/yama-frames/*.jpg 2>/dev/null | wc -l)
echo "Extracted $FRAME_COUNT frames"

if [ "$FRAME_COUNT" -lt 5 ]; then
    echo "ERROR: Expected at least 5 frames"
    exit 1
fi

echo "Demo completed successfully"
```

**Rules:**
- Scripts must be executable (`chmod +x`)
- Scripts must exit with code 0 on success
- Scripts must print clear error messages on failure
- Scripts should accept parameters for flexibility

### 3. Expected Output

For complex verification, provide expected output files:

```json
// expected/milestone-2.1.json
{
  "test": "keyframe_extraction",
  "video": "sample.mp4",
  "duration_ms": 120000,
  "frames_extracted": 24,
  "scene_changes_detected": 8,
  "average_thumbnail_size_kb": 42.5,
  "peak_memory_mb": 1850
}
```

### 4. Acceptance Criteria

Specific, testable criteria (see `acceptance-criteria.md`):

```markdown
### Acceptance Criteria
- [ ] AC-1: `extract_keyframes` produces 24 frames from 120s video
- [ ] AC-2: Thumbnails average <50KB
- [ ] AC-3: Memory stays <2GB during extraction
- [ ] AC-4: Scene detection reduces frame count by 30%
```

---

## Example: Complete Milestone Verification

```markdown
## Milestone 2.1: Key Frame Extraction

**Duration:** 2 days

### Tasks
| ID | Task | Description |
|----|------|-------------|
| 2.1.1 | Scene change detection | Histogram-based |
| 2.1.2 | Uniform sampling fallback | Every N seconds |
| 2.1.3 | Thumbnail generation | 320px JPEG |
| 2.1.4 | Batch extraction | Multiple videos |

### Implementation Files
- `platform/apple/host/src/indexer/keyframe.rs`
- `platform/apple/host/src/indexer/histogram.rs`

### Verification

#### Automated Tests
- [ ] `cargo test -p yama-host-apple test_keyframe_extraction` passes
- [ ] `cargo test -p yama-host-apple test_scene_detection` passes
- [ ] `cargo test -p yama-host-apple test_thumbnail_generation` passes
- [ ] `cargo clippy -p yama-host-apple` shows no new warnings

#### Demo Script
- [ ] `scripts/demos/milestone-2.1.sh ~/Videos/sample.mp4` runs without errors
- [ ] Output matches expected in `expected/milestone-2.1.json`

#### Acceptance Criteria
- [ ] AC-1: Extracts frames at 5-second intervals from test video
- [ ] AC-2: Scene change detection identifies transitions correctly
- [ ] AC-3: Thumbnails average <50KB file size
- [ ] AC-4: Memory stays <2GB for 10 concurrent videos
- [ ] AC-5: Progress callback invoked for each frame
```

---

## Verification Workflow

When completing a milestone:

1. **Run automated tests**
   ```bash
   cargo test -p {package} {test_pattern}
   ```

2. **Check lint**
   ```bash
   cargo clippy -p {package}
   ```

3. **Run demo script** (if applicable)
   ```bash
   scripts/demos/milestone-{id}.sh
   ```

4. **Compare output** (if applicable)
   ```bash
   diff expected/milestone-{id}.json actual/milestone-{id}.json
   ```

5. **Check all acceptance criteria**
   - Verify each AC is satisfied
   - Document any deviations

6. **Report results**
   ```markdown
   ## Milestone 2.1 Verification Report

   **Status:** Complete ✓

   - [x] All automated tests pass
   - [x] Clippy clean
   - [x] Demo script runs successfully
   - [x] All acceptance criteria met

   **Notes:**
   - Scene detection threshold tuned to 0.35 (from 0.30)
   ```

---

## Directory Structure

```
yama/
├── scripts/
│   └── demos/
│       ├── milestone-1.1.sh
│       ├── milestone-1.2.sh
│       ├── milestone-2.1.sh
│       └── ...
├── expected/
│   ├── milestone-1.1.json
│   ├── milestone-2.1.json
│   └── ...
└── docs/
    └── templates/
        ├── acceptance-criteria.md
        └── milestone-verification.md
```
