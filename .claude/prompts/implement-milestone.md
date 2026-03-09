# Implement Milestone

Use this prompt template when implementing a specific milestone from the roadmap.

---

## Context

**Phase:** {PHASE_NUMBER} - {PHASE_NAME}
**Milestone:** {MILESTONE_ID} - {MILESTONE_NAME}
**Duration:** {ESTIMATED_DAYS}

**Relevant Files:**
- {FILE_PATH_1}
- {FILE_PATH_2}

**Dependencies:**
- {DEPENDENCY_1}
- {DEPENDENCY_2}

---

## Requirements

{MILESTONE_DESCRIPTION}

### Tasks

| ID | Task | Description |
|----|------|-------------|
| {TASK_ID_1} | {TASK_NAME_1} | {TASK_DESCRIPTION_1} |
| {TASK_ID_2} | {TASK_NAME_2} | {TASK_DESCRIPTION_2} |

---

## Acceptance Criteria

- AC-1: {CRITERION_1}
- AC-2: {CRITERION_2}
- AC-3: {CRITERION_3}

---

## Verification Steps

1. **Run tests:**
   ```bash
   cargo test -p {PACKAGE} {TEST_PATTERN}
   ```

2. **Check lint:**
   ```bash
   cargo clippy -p {PACKAGE}
   ```

3. **Manual verification (if required):**
   {MANUAL_CHECK_DESCRIPTION}

4. **Confirm all acceptance criteria met**

---

## Output Requirements

When implementation is complete, provide:

1. **Files modified/created:**
   - List each file with a brief description of changes

2. **Test results:**
   ```
   test {test_name}::... ok
   test result: ok. X passed; 0 failed
   ```

3. **Acceptance criteria status:**
   - [ ] AC-1: {status}
   - [ ] AC-2: {status}
   - [ ] AC-3: {status}

4. **Deviations from plan:**
   - Any changes to the original specification
   - Rationale for changes

---

## Example Usage

### Filled Template

**Phase:** 2 - Indexing & Embeddings
**Milestone:** 2.1 - Key Frame Extraction
**Duration:** 2 days

**Relevant Files:**
- `platform/apple/host/src/indexer/keyframe.rs` (create)
- `platform/apple/host/src/indexer/histogram.rs` (create)
- `platform/apple/host/src/indexer/mod.rs` (modify)

**Dependencies:**
- VideoDecoder trait from Phase 1

### Requirements

Implement key frame extraction with scene change detection and thumbnail generation.

### Tasks

| ID | Task | Description |
|----|------|-------------|
| 2.1.1 | Scene change detection | Histogram-based visual change detection |
| 2.1.2 | Uniform sampling | Fallback extraction every N seconds |
| 2.1.3 | Thumbnail generation | 320px JPEG at quality 80 |

### Acceptance Criteria

- AC-1: `cargo test -p yama-host-apple test_keyframe_*` passes
- AC-2: Scene detection reduces frame count by >30%
- AC-3: Thumbnails average <50KB

### Verification Steps

1. Run tests:
   ```bash
   cargo test -p yama-host-apple test_keyframe_extraction
   cargo test -p yama-host-apple test_scene_detection
   ```

2. Check lint:
   ```bash
   cargo clippy -p yama-host-apple
   ```

3. Manual verification:
   Run example with sample video and inspect extracted frames

---

## Notes for Agents

- Read the relevant phase document for detailed specifications
- Check existing code patterns in similar modules
- Follow project conventions from CLAUDE.md
- Use `anyhow::Result` for application errors
- Use `tracing` for logging (not println!)
- Write tests alongside implementation
