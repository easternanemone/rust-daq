# Next Actions - Jules Deployment

## Immediate Actions Required

### 1. Check Sessions Awaiting Feedback

Two critical foundation sessions need your attention:

**Session 1: daq-27 (Serial Communication Refactor)**
- Session ID: `60019948141310675`
- Direct URL: https://jules.google.com/session/60019948141310675
- What it does: Creates shared serial communication module
- Why it matters: Blocks daq-28 and daq-26 (P1 priority)
- CLI check: `jules remote pull --session 60019948141310675`

**Session 2: daq-28 (ESP300 Prompt Bug)**
- Session ID: `10092682552866889619`
- Direct URL: https://jules.google.com/session/10092682552866889619
- What it does: Fixes ESP300 error propagation
- Why it matters: P1 priority bug fix
- CLI check: `jules remote pull --session 10092682552866889619`

**What to do:**
1. Visit each session URL in your browser
2. Look for messages or questions from Jules
3. Approve proposed changes or answer questions
4. This should unblock those sessions

### 2. Monitor for Completions

Run the monitoring script periodically:
```bash
./scripts/monitor_jules.sh
```

This will show:
- How many slots are available
- Which sessions completed
- When you can deploy more agents

### 3. Deploy Next Wave (When Slots Open)

Once you have 5+ free slots, deploy Wave 4 (Code Quality):

```bash
# Copy-paste these commands when slots are available:

jules remote new --repo . --session "Fix all clippy warnings. Run 'cargo clippy' and systematically fix all warnings throughout the codebase. Focus on: unused imports, unnecessary clones, inefficient string operations, missing trait implementations, and any logic issues. Ensure all fixes maintain existing functionality. Priority: P2"

jules remote new --repo . --session "Add error contexts throughout codebase. Use .context() or .with_context() from anyhow for all error conversions to provide helpful context. Focus on: file operations, instrument connections, configuration loading, data processing errors. Each error should have a clear description of what operation failed. Priority: P2"

jules remote new --repo . --session "Implement Display trait for error enums. Add Display implementations for all error types in rust_daq/src/error.rs (DaqError, InstrumentError, etc.). Provide user-friendly error messages that describe what went wrong and potential fixes. Priority: P2"

jules remote new --repo . --session "Create validation module. Add rust_daq/src/validation.rs with common validation helpers for: port numbers, IP addresses, file paths, numeric ranges, configuration values. Reuse these helpers throughout the codebase to reduce duplication. Priority: P2"

jules remote new --repo . --session "Remove dead code. Clean up unused imports, unused functions, commented-out code, and unreachable code blocks throughout the codebase. Run 'cargo build' to identify dead_code warnings and systematically remove them. Priority: P3"
```

See **DEPLOYMENT_QUEUE.md** for all remaining commands.

## Progress Summary

### ✅ Deployed Successfully (15 agents)

**Wave 1: Foundation (2 agents)**
- daq-27: Serial refactor (awaiting feedback)
- daq-34: FFT architecture (in progress)

**Wave 2: Dependent (4 agents)**
- daq-28: ESP300 prompt (awaiting feedback)
- daq-26: MaiTai query (in progress)
- daq-35: FFT buffer (in progress)
- daq-31: FFT config (in progress)

**Independent Track (4 agents)**
- daq-30: MovingAverage buffer (in progress)
- daq-32: CsvWriter I/O (in progress)
- daq-33: Trigger tests (in progress)
- daq-29: Mock config (in progress)

**Wave 3: Documentation (5 agents)**
- Module docs (in progress)
- Function docs (in progress)
- ARCHITECTURE.md (in progress)
- CONTRIBUTING.md (in progress)
- README examples (in progress)

### ⏳ Ready to Deploy (15 agents)

**Wave 4: Code Quality (5 agents)** - Commands ready in DEPLOYMENT_QUEUE.md
**Wave 5: Testing (6 agents)** - Commands ready in DEPLOYMENT_QUEUE.md
**Wave 6: Infrastructure (4 agents)** - Commands ready in DEPLOYMENT_QUEUE.md

### 📋 Future Deployment (10+ agents)

**Wave 7: PR Reviews** - Deploy after PRs are submitted
- Each PR gets 2 reviewers
- Cross-review for quality assurance

## Key Files Created

1. **CURRENT_STATUS.md** - Current situation and bottleneck analysis
2. **DEPLOYMENT_QUEUE.md** - All deployment commands ready to execute
3. **JULES_STATUS.md** - Detailed status of all 15 active sessions
4. **NEXT_ACTIONS.md** - This file (what to do next)
5. **scripts/monitor_jules.sh** - Automated monitoring script

## Timeline Estimate

- **Now**: 15 active, 2 awaiting feedback
- **After feedback** (5-10 min): Those 2 sessions complete, 2 slots free
- **Next 30-60 min**: More sessions complete, deploy Wave 4
- **Next 1-2 hours**: Deploy Waves 5 and 6 as slots open
- **Tomorrow**: Review PRs and deploy review agents
- **This week**: Merge PRs following dependency chain

## Success Criteria

When all agents complete, rust-daq will have:

- ✅ 10 code quality bug fixes
- ✅ Complete documentation (module, function, architecture)
- ✅ Comprehensive error handling with contexts
- ✅ Full test coverage (integration, property, stress)
- ✅ Performance benchmarks established
- ✅ CI/CD pipeline operational
- ✅ Development tooling in place

## Questions?

- Session stuck? Check https://jules.google.com/sessions
- Need to retry? Wait for 429 rate limit to clear (~5-10 min)
- Want to cancel? Jules agents can be stopped from web interface
- Need help? Check Jules docs: https://jules.google/docs
