# Orchestrator Quick Reference

This is the quick reference for the main Claude session (orchestrator) when coordinating agent teams.

## Quick Commands

### Spawn Platform Agents

**Apple Agent (for Phase 1-3 Mac work):**
```
Task tool:
  subagent_type: "Explore"
  description: "Apple: [brief task]"
  prompt: |
    Read .claude/agents/apple-agent.md for your role.

    ## Task
    [Milestone or task description]

    ## Acceptance Criteria
    - AC-1: [criterion]

    ## Scope
    platform/apple/ only
```

**Jetson Agent (for Phase 4):**
```
Task tool:
  subagent_type: "Explore"
  description: "Jetson: [brief task]"
  prompt: |
    Read .claude/agents/jetson-agent.md for your role.

    ## Task
    [Milestone description]

    ## Scope
    platform/jetson/ and containers/
```

**Web Agent:**
```
Task tool:
  subagent_type: "Explore"
  description: "Web: [brief task]"
  prompt: |
    Read .claude/agents/web-agent.md for your role.

    ## Task
    [Component or feature description]

    ## Scope
    web/ only
```

### Spawn Support Agents

**Code Review:**
```
Task tool:
  subagent_type: "feature-dev:code-reviewer"
  description: "Review [component]"
  prompt: |
    Review these files:
    - [file1]
    - [file2]

    Focus: [correctness, security, conventions]
```

**Architecture Design:**
```
Task tool:
  subagent_type: "feature-dev:code-architect"
  description: "Design [feature]"
  prompt: |
    Design the architecture for [feature].

    Requirements:
    - [req1]
    - [req2]

    Output: Implementation blueprint
```

**Codebase Exploration:**
```
Task tool:
  subagent_type: "Explore"
  description: "Find [pattern]"
  prompt: |
    Search the codebase for [pattern/usage/implementation].

    Questions:
    - [question1]
    - [question2]
```

### Parallel Agents

Run multiple agents simultaneously:

```
Single message with:

Task 1:
  subagent_type: "Explore"
  run_in_background: true
  description: "Apple: IndexProvider"
  prompt: "[Apple agent task]"

Task 2:
  subagent_type: "Explore"
  run_in_background: true
  description: "Web: Project list"
  prompt: "[Web agent task]"

Then use TaskOutput to check results when needed.
```

---

## Orchestrator Responsibilities

### What YOU (Orchestrator) Handle

| Domain | Files |
|--------|-------|
| Shared traits | `shared/platform-traits/` |
| Protocol definitions | `shared/protocol/` |
| Design tokens | `shared/yama-theme/`, `docs/brand/` |
| Container SDK | `shared/container-sdk/` |
| Roadmap | `roadmap/` |
| Agent config | `.claude/` |
| Documentation | `docs/` |

### What You Delegate

| Agent | Scope |
|-------|-------|
| Apple Agent | `platform/apple/` |
| Jetson Agent | `platform/jetson/`, `containers/` |
| Web Agent | `web/` |

---

## Workflow: Implementing a Milestone

### Single-Platform Milestone

```
1. TaskUpdate: Set milestone to in_progress

2. Check scope:
   - platform/apple/ only? → Spawn Apple Agent
   - platform/jetson/ only? → Spawn Jetson Agent
   - web/ only? → Spawn Web Agent

3. Wait for agent response

4. Review output:
   - Code quality
   - Test results
   - Acceptance criteria

5. If changes needed:
   - Resume agent with feedback

6. Commit changes

7. TaskUpdate: Set milestone to completed
```

### Cross-Cutting Milestone

```
1. TaskUpdate: Set milestone to in_progress

2. Implement shared trait changes yourself:
   - Modify shared/platform-traits/
   - Update protocol if needed

3. Commit trait changes

4. Spawn platform agents in parallel:
   - Apple Agent: Implement trait for Mac
   - Jetson Agent: Implement trait for Jetson

5. Wait for both to complete

6. Review and integrate

7. Commit implementations

8. TaskUpdate: Set milestone to completed
```

---

## Integration Points

### When Traits Change

If you modify `shared/platform-traits/`:

1. Notify affected agents in their next invocation
2. Provide the new trait signature
3. List acceptance criteria for implementation

Example:
```
Task prompt:
  The VideoDecoder trait was updated. New method:

  ```rust
  async fn extract_audio_chunk(
      &self,
      path: &Path,
      start_ms: u64,
      end_ms: u64,
  ) -> Result<AudioData>;
  ```

  Implement this for GStreamer VideoToolbox.
```

### When Protocol Changes

If you modify `shared/protocol/`:

1. Regenerate Rust bindings: `cargo build -p yama-protocol`
2. Regenerate TypeScript bindings: `cd web && npm run proto:gen`
3. Notify Web Agent of new message types

---

## Handling Agent Handoffs

When an agent reports a handoff:

```markdown
## Handoff: Apple Agent → Orchestrator

**Reason:** Need to modify shared trait
**Trait:** VideoDecoder
**Change:** Add extract_audio_chunk method
```

Your response:
1. Acknowledge the handoff
2. Make the trait change
3. Commit the change
4. Resume the agent with confirmation

---

## Task Tracking

### Start of Session

```
TaskList
# See current state

TaskCreate: "1.1.1" - Define VideoDecoder trait
TaskCreate: "1.1.2" - Define IndexProvider trait
# ... create tasks from roadmap
```

### During Work

```
TaskUpdate taskId="1.1.1" status="in_progress"
# Work on task

TaskUpdate taskId="1.1.1" status="completed"
# Move to next
```

### Check Progress

```
TaskList
# Shows: pending, in_progress, completed
```

---

## Commit Protocol

Before committing agent work:

1. **Review with code-reviewer agent** (for significant changes)
2. **Run tests:** `cargo test --workspace`
3. **Check lint:** `cargo clippy --workspace`
4. **Stage specific files** (not `git add .`)
5. **Write descriptive commit message**

Commit message format:
```
type(scope): description

- Detail 1
- Detail 2

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```

Types: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`
Scopes: `apple`, `jetson`, `web`, `traits`, `protocol`, `roadmap`
