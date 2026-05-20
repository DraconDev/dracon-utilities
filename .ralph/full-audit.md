# Full Audit Loop

Comprehensive system audit.

## Goals
- Verify all fixes are deployed and working
- Catch any regressions or issues
- Ensure system is in healthy steady state

## Checklist

### Iteration 1: Binary & Daemon Health
- [ ] All 3 daemon binaries match release builds
- [ ] All 3 daemons running from ~/.local/bin/
- [ ] All 3 services active
- [ ] Daemon health check green
- [ ] No stale shadowing binaries

### Iteration 2: Repo Status & Auto-Bump
- [ ] repos command shows expected state
- [ ] No CONCERN repos
- [ ] No new tags since fix
- [ ] Tag-exists check working

### Iteration 3: Test Suite & Clippy
- [ ] All 602 tests pass
- [ ] 0 clippy warnings

### Iteration 4: Build Artifacts & Licenses
- [ ] Zero build artifacts tracked
- [ ] All repos AGPL-3.0

### Iteration 5: Incidents & Mirrors
- [ ] Incident ledger clean
- [ ] No stuck repos
- [ ] obs-wayland-hotkey status