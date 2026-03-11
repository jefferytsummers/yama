# Yama UX Experiments

**Worktree:** `yama-ux`
**Branch:** `feature/ux-experiments`
**Purpose:** Explore different UX experiences for the native egui app

## Scope

This worktree focuses exclusively on **native egui UI experimentation**. Do not modify:
- Backend HTTP handlers
- Event bus implementation
- Authentication (handled in yama-auth)
- Core platform traits (handled in yama-arch)

## Focus Areas

### Priority 1: Project Configuration UX
After creating a project, users need to configure tools, workflows, and models.

**Approaches to explore:**
1. **Settings Panel** - Sidebar tab with collapsible sections
2. **Visual Builder** - Node-graph style workflow builder
3. **Inline Configuration** - Configure in project header area

### Priority 2: Unified Timeline View
Single scrollable timeline combining:
- Chat messages
- Video analysis events
- System events
- Tool executions

### Priority 3: Additional Experiments
- Command palette (`Cmd+K`)
- Floating panel layout
- Keyboard-first navigation
- Animation and transitions

## Key Files

| File | Purpose |
|------|---------|
| `platform/apple/host/src/ui/app.rs` | Main app structure |
| `platform/apple/host/src/ui/analyst/` | Analyst view components |
| `shared/yama-theme/src/` | Design system |
| `shared/yama-theme/src/animation.rs` | Animation utilities |

## New Files to Create

```
platform/apple/host/src/ui/analyst/
├── config_panel.rs     # Tools/Models/Workflows config UI
├── tool_card.rs        # Individual tool config card
└── workflow_builder.rs # Visual workflow editor

platform/apple/host/src/ui/timeline/
├── mod.rs              # Timeline view module
├── event.rs            # Event rendering
└── filters.rs          # Filter controls

shared/yama-theme/src/components/
├── toggle_switch.rs    # Toggle switch component
├── config_card.rs      # Configuration card
└── node_graph.rs       # Node graph for workflows
```

## Design Tokens

Always use tokens from `docs/brand/design-tokens.json`:
- Backgrounds: `obsidian`, `basalt`, `slate`, `graphite`, `stone`
- Accents: `amber`, `ember`, `jade`, `azure`, `violet`
- Text: `chalk`, `silver`, `ash`

## Integration with Other Worktrees

- **yama-arch** provides: PUT `/api/projects/:id/config` endpoint
- **yama-auth** provides: Authentication middleware

This worktree should assume these exist and wire up to them once available.

## Standards

- Rust 2024 edition, `clippy::pedantic`
- `tracing` for logging
- Design tokens from `docs/brand/design-tokens.json`

## Verification

Before committing:
```bash
cargo check -p yama-host-apple
cargo clippy -p yama-host-apple -- -D warnings
```

## Merge Target

When complete, merge to `dev` branch in main worktree.
