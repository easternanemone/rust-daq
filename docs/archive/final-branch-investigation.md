# Final Branch Investigation Results

**Date**: 2025-10-24  
**Method**: Parallel investigation (1 Gemini success, 4 manual completions)  
**Branches**: 5 remaining after Phase 2 cleanup  
**Result**: 0 cherry-picks, 2 concepts to extract, 5 branches to delete

---

## Summary Table

| Branch | Status | Reason | Concept Value | Action |
|--------|--------|--------|---------------|--------|
| bd-40-pixelbuffer-enum | Obsolete | Architecture changed | ✅ High (4× memory savings) | Extract → Issue → Delete |
| daq-33-trigger-test-coverage | Obsolete | Tests comprehensive | ❌ None | Delete |
| feature/log-consolidation | Has implementation | Single commit feature | ⚠️ Medium (UX improvement) | Extract → Issue → Delete |
| fix/bd-18-pvcam-plots | Obsolete | Already works | ❌ None | Delete |
| feature/remote-api | Obsolete | Architecture incompatible | ✅ High (design reference) | Document → Delete |

---

## Investigation Details

### 1. bd-40-pixelbuffer-enum ✅ EXTRACT CONCEPT

**Gemini Analysis**: 4× memory reduction for camera data  
**Current**: Vec<f64> (8 bytes/pixel) in ImageData  
**Proposed**: PixelBuffer enum (U8/U16/F64)  
**Savings**: 25MB per 2048×2048 frame, 250MB/s at 10Hz

**Recommendation**: Create beads issue, delete branch

---

### 2. daq-33-trigger-test-coverage ❌ DELETE

**Finding**: src/data/trigger.rs already has comprehensive tests  
**Coverage**: All modes (Edge/Level/Window), holdoff, pre/post samples, edge cases  
**Value**: None - tests already excellent

**Recommendation**: Delete immediately

---

### 3. feature/log-consolidation ⚠️ EXTRACT CONCEPT

**Finding**: Branch has 1 commit (f7ec65c) implementing consolidation  
**Feature**: Group duplicate errors with occurrence count  
**Current main**: No consolidation - just filtering  
**Value**: UX improvement for repetitive errors

**Recommendation**: Review implementation, create issue, delete branch

---

### 4. fix/bd-18-pvcam-plots ❌ DELETE

**Finding**: ImageTab in mod.rs renders images correctly  
**Implementation**: Full support for Measurement::Image  
**Integration**: PVCAM → ImageTab rendering works

**Recommendation**: Delete immediately

---

### 5. feature/remote-api 📦 DOCUMENT DESIGN

**Finding**: Comprehensive REST + WebSocket API (24 files, 779 lines)  
**Issue**: Uses Arc<Mutex<DaqAppInner>> (incompatible with actor model)  
**Value**: Design pattern useful for future implementation  
**Components**: Axum, auth, OpenAPI, client examples

**Recommendation**: Extract design doc, create issue, delete branch

---

## Beads Issues to Create

### Issue 1: PixelBuffer Memory Optimization
```
Title: Implement PixelBuffer enum for 4× memory reduction
Priority: 1 (High)
Type: feature
Branch: bd-40 (design reference, deleted)
```

### Issue 2: Log Error Consolidation
```
Title: Implement error consolidation in Event Log UI
Priority: 2 (Medium)  
Type: feature
Branch: log-consolidation (implementation reference, deleted)
```

### Issue 3: Remote API Design
```
Title: Design remote API for actor model architecture
Priority: 2 (Medium)
Type: feature
Branch: remote-api (design reference, deleted)
```

---

## Final Branch Deletion

```bash
# Delete all 5 remaining branches
git push origin --delete \
  bd-40-pixelbuffer-enum \
  daq-33-trigger-test-coverage \
  feature/log-consolidation \
  fix/bd-18-pvcam-plots \
  feature/remote-api
```

**Result**: 47 → 0 remote branches (100% cleanup)

---

## Metrics

**Total Investigation Time**: 4.5 hours (3 phases)  
**Branches Eliminated**: 47  
**Concepts Preserved**: 3 (PixelBuffer, Log consolidation, Remote API)  
**Value**: Clean repository + actionable design documents
