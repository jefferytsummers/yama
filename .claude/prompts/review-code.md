# Code Review Checklist

Use this prompt when reviewing code changes for quality, correctness, and adherence to project conventions.

---

## Review Scope

**Files to review:**
- {FILE_PATH_1}
- {FILE_PATH_2}

**Context:** {BRIEF_DESCRIPTION_OF_CHANGES}

---

## Checklist

### 1. Correctness

- [ ] Logic errors checked
- [ ] Edge cases handled (empty inputs, zero values, bounds)
- [ ] Error propagation correct (? operator, no unwrap in library code)
- [ ] Resource cleanup (file handles, connections, GPU memory)
- [ ] Thread safety verified for concurrent code

### 2. Project Conventions

- [ ] Uses `anyhow::Result` for application errors
- [ ] Uses `thiserror` for library errors
- [ ] Uses `tracing` for logging (not `println!` or `log`)
- [ ] Async traits use `#[async_trait]`
- [ ] No hardcoded paths or values (use config)
- [ ] Follows Obsidian Lens design tokens for UI code

### 3. Performance

- [ ] No unnecessary allocations in hot paths
- [ ] Appropriate use of references vs cloning
- [ ] Batch operations where applicable
- [ ] No blocking I/O in async code

### 4. Security

- [ ] No SQL injection risks (use parameterized queries)
- [ ] Input validation on external data
- [ ] No hardcoded secrets or credentials
- [ ] Path traversal protection for file operations

### 5. Testing

- [ ] New code has corresponding tests
- [ ] Edge cases tested
- [ ] Error paths tested
- [ ] Tests are deterministic (no flaky tests)

### 6. Documentation

- [ ] Public APIs have doc comments
- [ ] Complex logic has inline comments
- [ ] Module-level documentation where needed

---

## Output Format

### Critical Issues

Issues that must be fixed before merging:

```
file:line - Issue description
  - Impact: {severity}
  - Fix: {suggested fix}
```

### Major Issues

Issues that should be addressed:

```
file:line - Issue description
  - Impact: {severity}
  - Fix: {suggested fix}
```

### Minor Issues

Non-blocking suggestions:

```
file:line - Issue description
  - Suggestion: {improvement}
```

### Positive Notes

Good patterns worth highlighting:

```
file:line - What was done well
```

---

## Example Review

### Critical Issues

```
platform/apple/host/src/indexer/clip.rs:142 - SQL injection vulnerability
  - Impact: Security - user input directly interpolated into query
  - Fix: Use parameterized query with params![] macro

platform/apple/host/src/tools/executor.rs:89 - Potential deadlock
  - Impact: Correctness - holding read lock while acquiring write lock
  - Fix: Release read lock before acquiring write lock
```

### Major Issues

```
platform/apple/host/src/indexer/batch.rs:67 - Missing error handling
  - Impact: Reliability - unwrap() will panic on invalid video
  - Fix: Use ? operator and propagate error
```

### Minor Issues

```
platform/apple/host/src/model_manager.rs:23 - Could use tracing instead of println
  - Suggestion: Replace println! with tracing::info!
```

### Positive Notes

```
platform/apple/host/src/indexer/keyframe.rs:45 - Good use of histogram caching
  - Efficient scene detection without redundant computation
```

---

## Quick Reference

### Common Patterns

**Error handling:**
```rust
// Good
let result = operation().context("Failed to do X")?;

// Bad
let result = operation().unwrap();
```

**Logging:**
```rust
// Good
tracing::info!(video_path = %path.display(), "Indexing video");

// Bad
println!("Indexing video: {}", path.display());
```

**SQL:**
```rust
// Good
conn.execute("SELECT * FROM videos WHERE id = ?1", params![id])?;

// Bad
conn.execute(&format!("SELECT * FROM videos WHERE id = {}", id), [])?;
```

### Severity Levels

| Level | Definition | Action |
|-------|------------|--------|
| Critical | Security vulnerability, data loss, crash | Block merge |
| Major | Bug, performance issue, convention violation | Fix before merge |
| Minor | Style, optimization opportunity | Fix later |
| Suggestion | Enhancement, alternative approach | Consider |
