# Content Analyst Persona

> **MVP Target Persona** - The primary user Yama is designed for in its initial release.

## Profile

**Name:** Content Analyst

**Role:** Researcher, journalist, or analyst who works with video archives to extract insights, find patterns, and produce deliverables.

**Environment:**
- Works on a Mac (primary) or Linux workstation
- Has local storage with video collections (100GB - 10TB)
- Comfortable with file management, not necessarily technical
- May work solo or share findings with a team

**Mindset:**
- "I have too much footage and not enough time"
- "I know what I'm looking for, but finding it is the bottleneck"
- "I need to prove my findings with evidence (clips, timestamps, data)"

---

## Sub-Types

| Variant | Context | Example Task |
|---------|---------|--------------|
| **Journalist** | News investigation | Find all mentions of "layoffs" across 200 hours of earnings calls |
| **Academic** | Behavioral research | Count instances of specific gestures in interview footage |
| **Archivist** | Cataloging | Generate metadata and searchable index for video library |
| **Content Creator** | Repurposing | Find the funniest moments across 50 podcast episodes |
| **Legal/Compliance** | Discovery | Locate all footage containing a specific individual |

---

## Goals

1. **Find** - Locate specific moments without watching everything
2. **Extract** - Pull clips, frames, or segments that match criteria
3. **Count** - Quantify occurrences across a corpus
4. **Summarize** - Generate written overviews of video content
5. **Catalog** - Create searchable metadata for archives

---

## Pain Points (Current State)

| Pain Point | Impact |
|------------|--------|
| Manual scrubbing through hours of footage | Days of wasted time |
| No way to search video visually | Miss non-verbal cues, objects, scenes |
| Transcript search misses context | "He said 'fine'" - angry or agreeable? |
| Manual clip extraction is tedious | Premiere/ffmpeg for each segment |
| No structured output | Copy-paste timestamps into spreadsheets |
| Can't batch process | One video at a time |

---

## User Stories

### Epic 1: Query-Based Discovery

**US-1.1: Natural Language Search**
> As a Content Analyst, I want to search my videos using natural language so that I can find moments without knowing exact keywords.

*Acceptance Criteria:*
- [ ] Can query "find arguments" or "people laughing" (not just transcript words)
- [ ] Results show thumbnail + timestamp + confidence score
- [ ] Can click result to preview that moment in context
- [ ] Works across multiple videos in a folder

**US-1.2: Multi-Modal Search**
> As a Content Analyst, I want to combine audio, visual, and semantic criteria so that I can find nuanced moments.

*Examples:*
- "Someone saying 'no' while shaking their head"
- "Crowded scenes with applause"
- "Empty room with door opening"

---

### Epic 2: Clip Extraction

**US-2.1: Extract Matching Segments**
> As a Content Analyst, I want to automatically extract clips matching my query so that I don't have to manually cut each one.

*Acceptance Criteria:*
- [ ] Specify query + padding (e.g., 5 seconds before/after)
- [ ] Exports to folder with sensible naming (`source_HH-MM-SS.mp4`)
- [ ] Batch across entire corpus
- [ ] Progress indicator with cancel option

**US-2.2: Merge Adjacent Clips**
> As a Content Analyst, I want adjacent matches merged into single clips so that I don't get 50 2-second clips.

*Acceptance Criteria:*
- [ ] Configurable merge threshold (e.g., merge if gap < 10s)
- [ ] Option to keep separate or merge

---

### Epic 3: Counting & Tracking

**US-3.1: Object Counting**
> As a Content Analyst, I want to count how many times X appears in each video so that I can quantify my findings.

*Acceptance Criteria:*
- [ ] Specify object class or description ("red car", "person with hat")
- [ ] Per-video counts + totals + averages
- [ ] Export as CSV
- [ ] Optional: timestamps of each occurrence

**US-3.2: Presence Duration**
> As a Content Analyst, I want to know how long X is visible so that I can measure screen time or exposure.

*Acceptance Criteria:*
- [ ] Total duration per video
- [ ] Percentage of video
- [ ] Timeline visualization (optional)

---

### Epic 4: Summarization

**US-4.1: Per-Video Summary**
> As a Content Analyst, I want a written summary of each video so that I can quickly understand content without watching.

*Acceptance Criteria:*
- [ ] Key topics, people visible, notable moments
- [ ] Configurable length (brief / detailed)
- [ ] Timestamps for notable moments
- [ ] Export as markdown or plain text

**US-4.2: Corpus-Level Insights**
> As a Content Analyst, I want a summary across all videos so that I can identify themes and patterns.

*Acceptance Criteria:*
- [ ] Common themes, recurring elements
- [ ] Outliers or unique moments
- [ ] Links to specific videos/timestamps as evidence

---

### Epic 5: Cataloging & Indexing

**US-5.1: Batch Metadata Generation**
> As a Content Analyst, I want to generate metadata for my entire video library so that I can search it later.

*Acceptance Criteria:*
- [ ] Extracts: duration, resolution, key topics, detected objects, transcript snippets
- [ ] Outputs JSON or CSV
- [ ] Incremental (skip already-processed)
- [ ] Stores embeddings for semantic search

**US-5.2: Persistent Index**
> As a Content Analyst, I want my analysis persisted so that I don't reprocess every time.

*Acceptance Criteria:*
- [ ] Index stored locally (SQLite or similar)
- [ ] Fast subsequent queries against index
- [ ] Can rebuild/update index when videos change

---

## Workflow: End-to-End Example

```
1. IMPORT
   └─ Drag folder of 47 meeting recordings into Yama

2. INDEX (background, ~30 min for 47 videos)
   ├─ Transcription (Whisper)
   ├─ Scene detection
   ├─ Object detection (key frames)
   └─ Embedding generation

3. QUERY
   └─ "Find all mentions of 'Q4 projections' with visible charts"

4. REVIEW
   ├─ 23 results across 12 videos
   ├─ Click through thumbnails to preview
   └─ Star/flag relevant ones

5. EXTRACT
   └─ Export starred results as clips (5s padding)

6. REPORT
   ├─ Generate summary: "Q4 was discussed in 12/47 meetings..."
   └─ Export as markdown with embedded timestamps
```

---

## Key UX Principles

| Principle | Implementation |
|-----------|----------------|
| **Batch-first** | Every action works on folders, not just single files |
| **Progress transparency** | Always show "12/47 videos, ~15 min remaining" |
| **Non-destructive** | Never modify source videos, only create outputs |
| **Resumable** | Interruptions don't lose progress |
| **Offline-capable** | Works without internet after initial model download |

---

## Success Metrics

| Metric | Target |
|--------|--------|
| Time to first result (47 videos) | < 5 minutes (initial index) |
| Query response time (indexed) | < 3 seconds |
| Clip extraction throughput | 10 clips/minute |
| False positive rate | < 20% for natural language queries |

---

## Technical Implications

| Requirement | Implication |
|-------------|-------------|
| Batch processing | Job queue, background workers |
| Local-first | SQLite index, local model inference |
| Large corpus | Efficient seeking, key-frame extraction (not every frame) |
| Multi-modal | Whisper (audio) + CLIP/VLM (visual) + embeddings |
| Clip export | ffmpeg integration, fast seeking |

---

## Non-Goals (MVP)

These are explicitly out of scope for the Content Analyst persona:

- Real-time monitoring / live streams
- Multi-user collaboration
- Cloud storage integration
- Mobile access
- Alert systems

---

## Related Documents

- [Model Strategy](../model-strategy.md) - VLM and embedding model selection
- [Phase 5: VLM Orchestration](../phase-5-vlm-orchestration.md) - Inference architecture
