# Dracon Utilities TODO Sprint

## Completed
- [x] **Review corrections** — 0 production unwraps, 39 sync.rs tests, 0 own-code clippy warnings
- [x] **CI/CD pipeline** — `.github/workflows/ci.yml`
- [x] **Lint gates** — `#![warn(missing_docs)]` on all 4 crate roots
- [x] **cargo-outdated** — v0.19.0 installed, deps checked
- [x] **Clone audit** — 51 clones all legitimate
- [x] **dracon-libs docs** — Fixed all 95 missing-doc warnings in dracon-git
- [x] **Incident ledger** — Already implemented
- [x] **Time-window dedup** — Not needed

### Module Extraction Progress (system/main.rs)
| Module | Lines | Status |
|--------|-------|--------|
| events.rs | 260 | ✅ Extracted |
| links.rs | 233 | ✅ Extracted |
| zram.rs | 119 | ✅ Extracted |
| doctor.rs | 89 | ✅ Extracted |
| safety.rs | 144 | ✅ Extracted |
| guard | ~15 fns | ⏸️ Heavily coupled — deferred |
| storage | ~12 fns | ⏸️ Heavily coupled — deferred |

**main.rs**: 3,926 → 3,158 lines (20% reduction)

### Module Extraction Progress (dracon-sync/git/)
| Module | Lines | Status |
|--------|-------|--------|
| multi_remote.rs | 469 | ✅ Extracted (iteration 6) |
| discovery.rs | 155 | ✅ Extracted (iteration 6) |
| status.rs | 96 | ✅ Extracted (iteration 6) |
| config.rs | 37 | ✅ Extracted (iteration 6) |
| branch.rs | 238 | ✅ Extracted (iteration 6) |
| urls.rs | 70 | ✅ Extracted (iteration 6) |
| ops.rs | 164 | ✅ Extracted (iteration 6) |
| push.rs | 153 | ✅ Extracted (iteration 7) |
| diff.rs | 132 | ✅ Extracted (iteration 7) |
| staging.rs | 273 | ✅ Extracted (iteration 7) |
| misc.rs | 55 | ✅ Extracted (iteration 7) |
| mod.rs | 2,600 | Remaining (tests + core sync logic) |

**git/mod.rs**: 4,772 → 2,600 lines (45% reduction, 11 modules extracted, 1,849 lines extracted)

## Reflection

### What we accomplished
- 16 modules extracted across two crates (5 system + 11 git)
- **system/main.rs**: 3,926 → 3,158 lines (20% reduction)
- **git/mod.rs**: 4,772 → 2,600 lines (45% reduction)
- All 694 tests passing (0 failures, 6 ignored)
- All unused import warnings cleaned
- Every extracted module matches original function signatures exactly
- Backward compatibility maintained via `pub(crate) use submodule::*;` re-exports

### Lessons learned (iteration 7)
- Batch extraction by copying function bodies into new files works, but **signatures must match the original exactly** — even small changes (HashMap vs Vec, usize vs Vec<String>, added arguments) break callers
- The `&String` vs `&str` type mismatch in env tuples requires `.as_str()` or `.to_str().unwrap_or()` calls
- Test-only imports (`PathBuf`, `FileStatus`, `AuthType`) need `#[cfg(test)]` gating to avoid unused-import warnings
- PATH_LOCK static re-export across modules doesn't work reliably — use `OnceLock` in the consuming module instead

## Deferred (future work)
- **system/main.rs guard + storage**: Deeply coupled through `GuardRuntimeState`, needs careful incremental approach
- **Remaining git/mod.rs**: Tests and core sync logic (daemon calls, safety guards, cycle orchestration) — tightly coupled, diminishing returns
