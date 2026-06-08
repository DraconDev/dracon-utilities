# Draft: Multi-Remote Test Suite Plan

## Current State

### Test Infrastructure
- **Framework**: `cargo test` with `#[test]` (sync) and `#[tokio::test]` (async)
- **Dev-dependencies**: `tempfile = "3"`
- **Pattern**: Integration-style unit tests using real git commands on temp directories
- **No mocking framework** currently installed

### Existing Test Coverage (git.rs)
| Function | Tested? | Test Names |
|----------|---------|-----------|
| `load_secret` | ✅ Yes | `test_load_secret_from_env`, `test_load_secret_empty_env_var`, `test_load_secret_missing` |
| `get_remote_url` | ✅ Partial | `test_get_remote_url_nonexistent_remote` |
| `list_remotes` | ✅ Yes | `test_list_remotes_empty`, `test_list_remotes_one_remote` |
| `ensure_remote` | ✅ Yes | `test_ensure_remote_adds_new`, `test_ensu[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBhWXBvWEFOTS8vNWRUZndobWRUeHRyd2tiSWMxMlJkYUdlQ3I4SFpNQ21NCm9ZV0VMNXZTaHJhcVltVGxQMVNaWUtVeU9BMXNkajkrdnMwcW1MT09pRU0KLT4gWDI1NTE5IHI0eXFnRURKb1hXWFpXd1RBOWpkRFQwaC9lZ0tPdHk5WlFhREsvaVBsVlkKL3dCMFlRZHpIcjNoNzNNVU1OU2RUNWNJWXYvK2tqSGNPRlZGWGNVZTFNRQotPiBYMjU1MTkgOUZja2FGdGNkVWlZNjZ2czQ2UjZJL1FZdDFWVFp5QkhjbmxRTVJkYnYxZwpneXV4Q2w5Njd3WHQzODRaemdva2NvYlM2dFhrTDNGR08xU2hpTi9HdEQ0Ci0+IFgyNTUxOSBWMTdWR2Eyb0s5dzlvT0pwM0FQd3RFNFh3bDFCbEg5L0lJekltdXVOazNJCllVRzdHMXI4MUJpT3ZSMTdwVEFLMk1PRTQvN3ZvdEZMSVA1ZWJrVjFyQkEKLT4gWDI1NTE5IG12YW9DWVNUTzFFSlQwOUthNDhVU3FlVnppeDJMa0pSQXc0L05xRUxlRkkKLzJYUUFkNVRGSjliVVJyWkxKeHJRenUwVDZMYnpaU0M0MEJJZm9UVXVGOAotPiBYMjU1MTkgZHBVeTNTK3lCMjNSd3VPY2liSjNyb3g1ZkIvcStSd20yNGdhcHFING9qawpKeGZsbzVjK1pnZGxhUytTdGZoQVpWakNhT1k1VUE2Smp6MHNUMllPRUR3Ci0+IE4mNyMtZ3JlYXNlICUpSCBhNSVgfApRZXZ3eDBnL0hTVWpXb1FFZVFZM1JseXF5blBONHp3dThNR0lUMmRmTVcreDZoNlRnYXMxU3I0ZEVWRGUzbmlvCkl1Q2txMDdCd3ZKRHIrQQotLS0gUWxTU3dpK3FUNkJhNG5lZDNWQ0duNzkyeVEwV3lEZEZWL2xNaUR3L2FWMAp/hPw/FeweL6kbYyHSO4RrbClnu3iNyuqfIBESQczB5ipRtqmuJWYtAyxSvG33orWugDMwB4zZmeii]`, `test_ensure_remote_idempotent` |
| `remove_stale_remotes` | ❌ No | - |
| `resolve_push_url` | ❌ No | - |
| `configure_all_remotes` | ❌ No | - |
| `auto_create_repo` | ❌ No | - |
| `auto_create_all_remotes` | ❌ No | - |
| `create_repo_on_github` | ❌ No | - |
| `create_repo_on_gitlab` | ❌ No | - |
| `create_repo_on_codeberg` | ❌ No | - |
| `push_to_named_remote` | ❌ No | - |
| `push_to_all_remotes` | ❌ No | - |
| `push_mirror_remotes` | ❌ No | - |

### Existing Test Coverage (sync.rs)
- Many async integration tests for `sync_repo()`
- No direct tests for multi-remote push logic

## Strategy Decisions

### Testing External CLIs (gh, glab)
- **Approach**: Don't mock; test error paths and edge cases
- **Rationale**: Mocking `std::process::Command` is invasive and fragile
- **Alternative**: Test integration with real commands in CI (skipped locally if tools missing)

### Testing HTTP (Codeberg)
- **Approach**: Add `wiremock` dev-dependency, inject client
- **Rationale**: `reqwest` calls need deterministic HTTP responses
- **Refactor needed**: Make `create_repo_on_codeberg` accept `&reqwest::Client`

### Testing Git Operations
- **Approach**: Continue existing pattern - real git commands on temp repos
- **Rationale**: Proven pattern, catches real git behavior

## Test Plan Outline

### Phase 1: Easy Wins (Internal Logic)
1. `resolve_push_url` - template substitution
2. `remove_stale_remotes` - origin protection, stale cleanup
3. `configure_all_remotes` - all remotes configured, URLs resolved
4. `auto_create_all_remotes` - filtering by `auto_create`

### Phase 2: HTTP Testing (Codeberg)
5. `create_repo_on_codeberg` - success, 409 exists, 422 exists, failure
6. Add `wiremock` dev-dependency
7. Refactor to accept injectable client

### Phase 3: Git Push Testing
8. `push_to_named_remote` - SSH push, HTTPS fallback, retry
9. `push_to_all_remotes` - priority ordering, error collection
10. `push_mirror_remotes` - full integration flow

### Phase 4: External CLI Error Paths
11. `create_repo_on_github` - "already exists" error parsing
12. `create_repo_on_gitlab` - "already exists" error parsing
13. `auto_create_repo` - platform routing

### Phase 5: End-to-End Integration
14. Full multi-remote flow in `sync_repo()`
15. Daemon remote failure tracking
