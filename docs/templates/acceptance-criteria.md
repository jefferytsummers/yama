# Acceptance Criteria Template

## Purpose

Acceptance criteria define the **testable conditions** that must be met for a task or milestone to be considered complete. Well-written criteria enable:
- Agents to verify their own work
- Clear definition of "done"
- Consistent quality across implementations

---

## Requirements

Each criterion MUST be:

| Property | Description | Example |
|----------|-------------|---------|
| **Testable** | Can be verified with a command, inspection, or measurement | `cargo test test_search` passes |
| **Specific** | No ambiguous terms like "fast", "good", "works" | Response time <3 seconds |
| **Independent** | Can be verified in isolation | Does not require manual UI interaction |
| **Measurable** | Has a clear pass/fail threshold | Memory usage <2GB |

---

## Format

Use this format for acceptance criteria:

```markdown
- AC-1: `{command or action}` {expected result}
- AC-2: {Observable behavior} {measurement threshold}
- AC-3: {Condition} when {trigger}
```

---

## Examples

### Good Criteria

```markdown
### Milestone 2.1: Key Frame Extraction

- AC-1: `cargo test -p yama-host-apple test_keyframe_extraction` passes
- AC-2: Extracted thumbnails average <50KB file size
- AC-3: Scene change detection reduces frame count by >30% vs uniform sampling
- AC-4: Memory stays below 2GB when extracting frames from 10 concurrent videos
- AC-5: Progress callback invoked for each frame extracted
```

### Bad Criteria (Avoid)

```markdown
### Milestone 2.1: Key Frame Extraction

- AC-1: Key frames are extracted correctly ❌ (not specific)
- AC-2: Extraction is fast ❌ (not measurable)
- AC-3: The system handles large videos ❌ (ambiguous)
- AC-4: User can see progress ❌ (requires manual verification)
```

---

## Criteria Categories

### Functional

Verify the feature works as specified:

```markdown
- AC: `search_videos("budget")` returns results matching "budget" in transcript
- AC: Clip extraction creates valid MP4 files that play in QuickTime
```

### Performance

Verify timing and resource constraints:

```markdown
- AC: Query response time <3 seconds for 500K indexed frames
- AC: Batch indexing processes >10 videos/minute on M1 Mac
- AC: Peak memory usage <8GB during indexing
```

### Error Handling

Verify graceful degradation:

```markdown
- AC: Missing model file returns `ModelNotFound` error with path
- AC: Corrupt video file logs warning and continues batch
- AC: Network timeout retries 3 times with exponential backoff
```

### Integration

Verify components work together:

```markdown
- AC: Indexed video appears in search results within 5 seconds
- AC: Tool executor correctly routes to analyze_frame handler
```

---

## Verification Commands

Criteria should reference specific verification methods:

### Unit Tests

```markdown
- AC: `cargo test -p {package} {test_name}` passes
```

### Integration Tests

```markdown
- AC: `cargo test --test integration_{name}` passes
```

### Manual Verification

When manual verification is unavoidable, be explicit:

```markdown
- AC: [Manual] Chat message appears incrementally in egui UI
- AC: [Manual] Video thumbnail renders without artifacts
```

### Scripts

For complex verification:

```markdown
- AC: `scripts/verify/milestone-2.1.sh` exits with code 0
```

---

## Linking to Milestones

Each milestone in the roadmap should include an Acceptance Criteria section:

```markdown
## Milestone 2.1: Key Frame Extraction

**Duration:** 2 days

### Tasks

| ID | Task | Description |
|----|------|-------------|
| 2.1.1 | Scene change detection | Histogram-based change detection |
| 2.1.2 | Uniform sampling fallback | Extract every N seconds |

### Acceptance Criteria

- AC-1: `cargo test -p yama-host-apple test_keyframe_*` passes (all tests)
- AC-2: Scene change detection reduces frames by >30% vs uniform
- AC-3: Thumbnails average <50KB
- AC-4: Memory <2GB for 10 concurrent videos
```

---

## Checklist for Writing Criteria

Before finalizing acceptance criteria, verify:

- [ ] Each criterion has a specific, measurable threshold
- [ ] Verification method is documented (test, script, manual)
- [ ] No ambiguous terms (fast, good, works, handles)
- [ ] Criteria cover both happy path and error cases
- [ ] Performance criteria include target hardware (e.g., "on M1 Mac")
