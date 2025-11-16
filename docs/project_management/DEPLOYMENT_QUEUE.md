# Jules Deployment Queue

**Status**: Waiting for concurrent session slots to open
**Current Active**: 15 sessions
**Concurrent Limit**: ~15 sessions
**Daily Quota Remaining**: 85 sessions

## Wave 4: Code Quality (5 agents) - READY TO DEPLOY

### Agent 1: Fix Clippy Warnings
```bash
jules remote new --repo . --session "Fix all clippy warnings. Run 'cargo clippy' and systematically fix all warnings throughout the codebase. Focus on: unused imports, unnecessary clones, inefficient string operations, missing trait implementations, and any logic issues. Ensure all fixes maintain existing functionality. Priority: P2"
```

### Agent 2: Add Error Contexts
```bash
jules remote new --repo . --session "Add error contexts throughout codebase. Use .context() or .with_context() from anyhow for all error conversions to provide helpful context. Focus on: file operations, instrument connections, configuration loading, data processing errors. Each error should have a clear description of what operation failed. Priority: P2"
```

### Agent 3: Implement Display for Errors
```bash
jules remote new --repo . --session "Implement Display trait for error enums. Add Display implementations for all error types in rust_daq/src/error.rs (DaqError, InstrumentError, etc.). Provide user-friendly error messages that describe what went wrong and potential fixes. Priority: P2"
```

### Agent 4: Create Validation Module
```bash
jules remote new --repo . --session "Create validation module. Add rust_daq/src/validation.rs with common validation helpers for: port numbers, IP addresses, file paths, numeric ranges, configuration values. Reuse these helpers throughout the codebase to reduce duplication. Priority: P2"
```

### Agent 5: Remove Dead Code
```bash
jules remote new --repo . --session "Remove dead code. Clean up unused imports, unused functions, commented-out code, and unreachable code blocks throughout the codebase. Run 'cargo build' to identify dead_code warnings and systematically remove them. Priority: P3"
```

## Wave 5: Testing (6 agents) - READY TO DEPLOY

### Agent 6: Integration Tests
```bash
jules remote new --repo . --session "Add integration tests. Create tests/ directory with end-to-end integration tests for: full instrument lifecycle (connect, acquire, disconnect), data pipeline (instrument → processor → storage), session save/load functionality, configuration loading. Test realistic usage scenarios. Priority: P2"
```

### Agent 7: Property-Based Tests
```bash
jules remote new --repo . --session "Add property-based tests with proptest. Create property tests for data processors: MovingAverage (window size invariants), IIRFilter (filter stability), FFT (Parseval's theorem), Trigger (threshold detection). Focus on mathematical correctness and edge cases. Priority: P2"
```

### Agent 8: Benchmark Suite
```bash
jules remote new --repo . --session "Add Criterion benchmark suite. Create benches/ directory with performance benchmarks for: data processor throughput (samples/sec), FFT performance vs window size, storage write rates, instrument data streaming. Establish performance baselines. Priority: P2"
```

### Agent 9: Stress Tests
```bash
jules remote new --repo . --session "Add stress tests for memory and stability. Create long-running tests that: continuously stream data for 10+ minutes, monitor memory usage for leaks, test buffer overflow handling, verify async task cleanup. Use cargo-watch with memory profiling. Priority: P2"
```

### Agent 10: Verify Mock Coverage
```bash
jules remote new --repo . --session "Verify and improve mock instrument coverage. Ensure all instruments in rust_daq/src/instrument/ have complete mock mode implementations. Check: all public methods have mock behavior, mock data is realistic, error cases are testable. Add missing mock implementations. Priority: P2"
```

### Agent 11: GUI Tests
```bash
jules remote new --repo . --session "Add automated GUI tests. Use egui test utilities to test: experiment configuration panel validation, control button states and transitions, error display in UI, session save/load via GUI. Focus on user interaction flows. Priority: P3"
```

## Wave 6: Infrastructure (4 agents) - READY TO DEPLOY

### Agent 12: Config Validation
```bash
jules remote new --repo . --session "Add comprehensive config validation. In rust_daq/src/config.rs, add validation for all Settings fields: check instrument types are recognized, verify file paths exist or are writable, validate numeric ranges (sample rates, buffer sizes), provide helpful error messages with fix suggestions. Priority: P2"
```

### Agent 13: Config Migration
```bash
jules remote new --repo . --session "Add config migration system. Implement version field in config files and migration logic to handle config format changes. Support upgrading from older versions with automatic field mapping and sensible defaults. Document breaking changes. Priority: P2"
```

### Agent 14: GitHub Actions CI
```bash
jules remote new --repo . --session "Setup GitHub Actions CI pipeline. Create .github/workflows/ci.yml with jobs for: cargo test (all tests), cargo clippy (fail on warnings), cargo fmt --check, cargo audit (security), cross-platform testing (Linux, macOS, Windows). Cache dependencies for speed. Priority: P1"
```

### Agent 15: Development Scripts
```bash
jules remote new --repo . --session "Add development helper scripts. Create scripts/ directory with: dev.sh (run with hot-reload), test.sh (run all tests with coverage), lint.sh (clippy + fmt), bench.sh (run benchmarks), setup.sh (install dependencies). Make all scripts cross-platform compatible. Priority: P3"
```

## Wave 7: PR Reviews (Deploy after PRs are ready)

### Review Agent Template
Each PR will get 2 review agents:
- Primary: Focus on correctness, testing, thread safety
- Secondary: Focus on documentation, style, API design

**PR Review Command Template:**
```bash
jules remote new --repo . --session "Review PR for [PR-TITLE]. As [PRIMARY/SECONDARY] reviewer, analyze: [focus areas]. Check for: [specific concerns]. Provide constructive feedback on code quality, potential bugs, and improvement suggestions."
```

## Monitoring Commands

```bash
# Check session status
jules remote list --session | head -20

# Pull specific session results
jules remote pull --session <SESSION_ID>

# Count active sessions
jules remote list --session | grep "In Progress" | wc -l

# Check for completed sessions
jules remote list --session | grep "Completed" | head -10
```

## Deployment Strategy

1. **Check for session completions** every 10-15 minutes
2. **Deploy in batches of 3-5** as slots open
3. **Prioritize high-value tasks**: CI pipeline, testing, documentation
4. **Monitor "Awaiting Feedback"** sessions via web interface
5. **Reserve slots** for PR reviews once PRs are submitted

## Next Actions

- [ ] Check https://jules.google.com/sessions for sessions needing feedback
- [ ] Address daq-27 and daq-28 if they need user input
- [ ] Monitor for session completions
- [ ] Deploy Wave 4 (Code Quality) when 5 slots open
- [ ] Deploy Wave 5 (Testing) when 6 slots open
- [ ] Deploy Wave 6 (Infrastructure) when 4 slots open
---
*Note: "Jules" refers to an internal AI agent used for development and code management within this project.*