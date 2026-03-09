# Archived: Content Analyst Traditional UX Roadmap

**Archived:** 2026-03-09
**Reason:** Replaced by agent-first vision aligning with prototype direction

## Summary

This archive contains the original Content Analyst roadmap that followed a traditional UX pattern:
- Import videos → Index → Search → Extract clips

The new roadmap adopts an **agent-first, conversational** approach where:
- Users interact through natural language with AI agent presets
- The system proactively assists rather than waiting for explicit commands
- Tools are exposed to agents for autonomous operation

## Archived Files

| File | Original Purpose |
|------|------------------|
| `phase-0-ux-discovery.md` | UX research, wireframes, component inventory |
| `phase-1-foundation.md` | Core traits, batch processing, index providers |
| `phase-2-video-indexing.md` | SQLite schema, CLIP embeddings, sqlite-vss |
| `phase-3-transcription.md` | Whisper integration, FTS5 search |
| `phase-4-inference.md` | Local GGUF/MLX models, Metal acceleration |
| `phase-5-search.md` | Semantic + full-text search, query parsing |
| `phase-6-clip-extraction.md` | FFmpeg export, batch operations |
| `phase-7-tools.md` | Content Analyst agent tools |
| `phase-8-export.md` | Markdown, CSV, JSON reports |

## What Was Preserved

The following documents from this roadmap remain active (not archived):

- `roadmap/model-strategy.md` - Technical reference for model selection
- `roadmap/personas/content-analyst.md` - User stories and workflows

## New Roadmap

See `roadmap/README.md` for the consolidated 4-phase agent-first roadmap:

1. **Phase 1: Core Infrastructure** - SQLite, batch processor, GStreamer, model manager
2. **Phase 2: Indexing & Embeddings** - CLIP, Whisper, sqlite-vss, text embeddings
3. **Phase 3: Agent System** - Presets, tool executor, chat interface, artifacts
4. **Phase 4: Jetson Thor Deployment** - DeepStream, Triton, WebRTC, edge optimization

## Historical Reference

These archived phases contain valuable technical specifications that inform the new roadmap:
- SQL schema designs → Phase 1
- CLIP/Whisper integration patterns → Phase 2
- Tool definitions and schemas → Phase 3
- Jetson deployment patterns → Phase 4 (from `archive/security-monitoring/`)
