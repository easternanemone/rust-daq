# Jules Agent Dependency Notifications

**Jules-17 Dependency Coordinator - Automatic Notifications**

This file contains real-time notifications when Jules agent dependencies are satisfied.

**Legend**:
- ✅ Ready: All dependencies completed, task can start
- ⚠️ Blocked: Waiting on one or more dependencies
- 🔴 Critical: High-priority blocker affecting multiple tasks

---

## [2025-11-20] Initial Status Report

**9 Tasks Ready to Start Immediately**:
- bd-95pj (Jules-2): ESP300 V3 Migration ✅
- bd-l7vs (Jules-3/4): MaiTai + Newport V3 Migration ✅
- bd-e18h (Jules-5): PVCAM V3 Fix ✅
- bd-op7v (Jules-6): Standardize Measurement Enum ✅
- bd-9cz0 (Jules-7): Fix Trait Signature Mismatches ✅
- bd-rcxa (Jules-8): Arrow Batching in DataDistributor ✅
- **bd-hqy6 (Jules-10): Define ScriptEngine Trait** 🔴 CRITICAL PRIORITY
- bd-ya3l (Jules-14): Rhai/Lua Backend (ready after Jules-10)

**4 Tasks Blocked**:
- bd-vkp3 (Jules-9): HDF5 + Arrow - waiting on Jules-8 (bd-rcxa)
- bd-svlx (Jules-11): PyO3 Backend - waiting on Jules-10 (bd-hqy6) 🔴
- bd-dxqi (Jules-12): V3 API Bindings - waiting on Jules-10 + Jules-11 🔴
- bd-u7hu (Jules-13): Hot-Reload - waiting on Jules-12 🔴

**CRITICAL ACTION REQUIRED**:
Jules-10 (bd-hqy6) MUST start immediately. It blocks 4 other tasks (Jules-11, Jules-12, Jules-13, Jules-14).

---

## Usage

Run the dependency monitor to automatically detect when tasks become ready:

```bash
# One-time check
./scripts/jules_dependency_monitor.sh --once

# Continuous monitoring (every 5 minutes)
./scripts/jules_dependency_monitor.sh --daemon
```

This file will be automatically updated with notifications as dependencies are satisfied.

---
