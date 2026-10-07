# Push failure classification: a local hook refusal is not a transport failure

**Date**: 2026-10-07
**Status**: Code + tests landed in `dracon-sync` — **NOT yet built or deployed**
**Scope**: `dracon-sync/src/git/push.rs`

---

## TL;DR

`classify_push_failure()` had no arm for a **local** pre-push hook refusal, so
every repo-local policy rejection fell through to the final `else` and was
reported as

> transport/auth failure (network, timeout, or credentials)

On 2026-10-07 four repos were push-stuck on exactly this misdiagnosis: the
real causes were a bucket asset-retirement guard, a bucket size budget, and a
warden secret scan. The daemon sent the operator to the network and
credentials. Retryable-looking transport failures also burn the full 5-try
stuck budget on rejections that retrying can never fix.

> **Deployment note**: this change is in the working tree only. The running
> `dracon-sync.service` still carries the old classifier until the binary is
> rebuilt and reinstalled. Nothing here was deployed, and the service was not
> stopped (per the quiesce policy in `AGENTS.md`).

## §1 — Why this misdiagnosis kept happening

`classify_push_failure` was introduced in v0.113.50 precisely to stop the
daemon "misdirecting the operator to network/credentials when the true cause
was a history fork". The same defect then recurred for server-side policy
(v0.113.83 added the DNS arm) and now for local hooks — the **third** instance
of the same class.

The predicates are all *forge-side* shaped:

| predicate | matches |
|---|---|
| `is_transient_network_outage` | DNS / name resolution |
| `is_transient_forge_outage` | Gitaly / 5xx |
| `is_pack_too_large` | `GH001`, `pack exceeds` |
| `is_permanent_push_rejection` | `pre-receive hook declined`, `protected branch`, `Permission denied (publickey)` |
| `is_push_rejected` | `rejected`, `non-fast-forward`, `fetch first` |

A local hook prints its **own** stderr, e.g.

```
pre-push: bucket high-water guard blocked this push
error: failed to push some refs to 'github.com:DraconDev/dracon-platform.git'
```

which matches none of them. Note `is_permanent_push_rejection` looks for
`hook declined` / `pre-receive hook declined` — server-side wording. `pre-push`
is a different string and does not contain it.

## §2 — The fix

A new predicate plus one arm, placed after the transient arms (so genuine
transient handling is untouched) and **before** `is_permanent_push_rejection`:

```rust
pub(crate) fn is_local_hook_rejection(err_msg: &str) -> bool {
    err_msg.contains("pre-push")
        || err_msg.contains("pre-commit")
        || err_msg.contains("dracon-warden:")
        || err_msg.contains("dracon-warden hook")
        || err_msg.contains("Possible plaintext secrets")
}
```

returning

> local pre-push hook refused the push (repo policy: asset-guard / size budget / secret scan / history guard — fix the policy, not the network)

### `Possible plaintext secrets` is not redundant

The warden secret scan is the one local refusal that names **no** hook label —
it prints only its own warning. An earlier draft of this predicate listed only
the label markers and the new test failed on exactly that message. It is the
`pi-goal-list-loop-audit` case, and it is why the marker is there.

### No collision with server-side arms

`pre-receive hook declined` does not contain `pre-push`, so the two sets are
disjoint. Pinned by `test_local_hook_rejection_does_not_swallow_server_side_policy`,
which also asserts the GitHub `GH006` protected-branch message still classifies
as `server-side policy`.

## §3 — Tests

`cargo test --locked --bins` — **1259 passed, 0 failed, 3 ignored**.

Three new cases, each using verbatim messages from the incident:

- `test_local_hook_rejection_is_not_transport_failure` — the bucket high-water
  guard, the bucket size guard, the warden secret scan and the warden history
  guard all classify as a local hook, and **none** may report `transport/auth`.
- `test_local_hook_rejection_does_not_swallow_server_side_policy` — the
  server-side arms still win.
- `test_local_hook_rejection_leaves_real_transport_alone` — timeout, DNS and
  `Permission denied (publickey)` are untouched.

## §4 — Follow-up not done here

`is_local_hook_rejection` is currently only consulted by
`classify_push_failure`, which picks the human-readable cause string. The
stuck-budget accounting is a separate decision and was not changed: a local
hook refusal still counts against the stuck budget, which is correct — it is
not transient, and the operator must act. What changed is only *what the
operator is told to go and look at*.

Note also that the guard still retries a hook-blocked push five times before
handing over. That is deliberate and left alone, but a follow-up could skip
retries for `is_local_hook_rejection` the way DNS/forge-outage already do,
since a local policy verdict is deterministic.