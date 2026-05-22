---
commands:
  - name: todo
    run: cat TODO.md
    timeout: 5
  - name: git-status
    run: cd /home/dracon/Dev/dracon-utilities && git status --short && git log --oneline -5
    timeout: 10
  - name: cargo-check
    run: cd /home/dracon/Dev/dracon-utilities && cargo check -p dracon-sync -p dracon-system -p dracon-warden 2>&1 | tail -5
    timeout: 120
  - name: cargo-test
    run: cd /home/dracon/Dev/dracon-utilities && DRACON_SYNC_GIT_BIN=/run/current-system/sw/bin/git cargo test -p dracon-sync -p dracon-system -p dracon-warden --test-threads=1 2>&1 | tail -15
    timeout: 300
  - name: incident-ledger
    run: cat ~/.local/state/dracon/dracon-sync-incidents.jsonl 2>/dev/null | tail -5 || echo "(no ledger)"
    timeout: 5
max_iterations: 15
items_per_iteration: 1
reflect_every: 4
inter_iteration_delay: 2
completion_promise: DONE
completion_gate: required
required_outputs:
  - TODO.md
  - OPEN_QUESTIONS.md
timeout: 600
stop_on_error: true
guardrails:
  block_commands:
    - 'git\s+push'
    - 'git\s+push\s+--force'
    - 'git\s+push\s+--force-with-lease'
    - 'rm\s+-rf\s+/'
    - 'cargo\s+publish'
    - 'gh\s+repo\s+create'
    - 'gh\s+release\s+create'
    - 'systemctl.*stop'
    - 'systemctl.*restart'
    - 'systemctl.*start'
  protected_files:
    - '.env*'
    - '*.pem'
    - '*.key'
    - '.ssh/'
    - 'secrets/'
    - 'policy:secret-bearing-paths'
---
# Dracon Utilities — Audit TODO Loop

You are an autonomous coding agent running in a loop.
Each iteration starts with a fresh context.
Your progress lives in the code and git history.

There is a `RALPH_PROGRESS.md` file in this directory. Read it at the start of each iteration to know what's been done. Write to it at the end of each iteration.

## Current state

{{ commands.todo }}

{{ commands.git-status }}

{{ commands.cargo-check }}

## The TODO list

Work through the items in **`TODO.md`** from top to bottom. Each iteration picks exactly **one** item. Do not skip items. If an item has multiple sub-tasks, do one sub-task per iteration.

### Item priority order

1. 🔴 **Settle GitHub Actions billing** — informational/reminder, check documentation
2. 🔴 **Bump `git2` in dracon-libs** — code change in sibling repo
3. 🔴 **Investigate `wal-backup` daemon loop** — diagnostic/debug task
4. 🟡 **Monitor `proc-macro-error`** — research/documentation
5. 🟡 **Add periodic incident ledger pruning** — code change
6. 🟡 **Review scribe prompt injection sanitization** — code review/improvement
7. 🟡 **Enable release profile optimizations** — config change
8. 🟡 **Test `nix_auto_update`** — testing
9. 🔵 **Update `EnvRestorer` docstring** — docs fix
10. 🔵 **Document `Restart=always` behavior** — docs improvement
11. 🔵 **Run `cargo update`** — maintenance

## Rules

- Pick exactly **one** item from the TODO list each iteration
- Items with 🔴 priority come first, then 🟡, then 🔵
- Within same priority, go top to bottom
- Do not advance to the next item until the current one is done and committed
- Run `cargo check` and relevant tests before committing
- Commit each item with a descriptive message: `fix(audit): <description>`
- Do not push — guardrails block it
- Leave `TODO.md` unchanged (it's the source of truth for what's left)
- Update `RALPH_PROGRESS.md` at the end of each iteration
- If something is blocked (e.g., needs billing), mark it with `[BLOCKED: reason]` in progress and move to the next item

## Completion

Stop with `<promise>DONE</promise>` only when:
1. All items in `TODO.md` are either done or marked `[BLOCKED: reason]`
2. `OPEN_QUESTIONS.md` exists and has no unresolved P0/P1 items
3. `cargo check` passes clean
4. No uncommitted changes in the working tree
