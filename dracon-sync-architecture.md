# Ratio & Fact Reporting — AI-to-AI Commit Architecture

## Core Axiom

**The commit message is not written by an AI. It's written by a dumb deterministic script.**

The Worker AI is an untrusted, chaotic coder that edits files. The Committer is a deterministic auditor that extracts raw data and stamps a routing key.

---

## The Protocol

### Phase 1: The Worker (Untrusted Mutator)

The AI Coding Agent operates in an isolated sandbox. It has **no Git knowledge**. It just:

1. **Edits files**: `auth.py`, `db.js`, `TODO.md`
2. **Mutates the ledger**: Changes `- [ ]` to `- [x]` in `todo.md`
3. **Yields control**: Stops and signals "I'm done"

The Worker never knows about commits. It never runs `git commit`. It's a chaotic variable.

### Phase 2: The Separate Committer (Deterministic Auditor)

A standalone script (Python, Rust, Bash) wakes up and performs the **Dual-Diff Audit**:

#### Step A: Parse the TODO.md Diff
```bash
git diff HEAD~1 HEAD -- TODO.md | grep -E '^\+.*\[x\]'
# Extract what the Worker newly claimed to close
# Result: ["My active task", "criteria 1", "criteria 2"]
```

#### Step B: Parse the Code Diff
```bash
git diff --stat HEAD~1 HEAD
# Result: auth.py +20/-0, db.js +15/-5
```

#### Step C: Parse Test Results
```bash
pytest --json-report | grep -o '"tests_passed": [0-9]+'
# Result: tests_passed: 42
```

#### Step D: Generate the Routing Key
The Committer doesn't guess. It **counts** and **formats**:

```text
Title: "sync: X checked, Y files"
Body: Machine-readable JSON with:
  - ledger_delta: {checked: ["task1", "task2"]}
  - code_delta: {files: ["auth.py", "db.js"]}
  - verification: {tests_passed: 42, tests_failed: 0}
```

---

## The Commit Artifact

When downstream AI queries `git show <hash>`:

```text
commit abc123...
Author: Environment-Orchestrator <bot@system>
Date:   Sat May 23 14:32:01 2026

    sync: 2 checked, 2 files

    {
      "ledger_delta": {
        "checked": [
          "My active task",
          "criteria 1"
        ]
      },
      "code_delta": {
        "files": [
          "src/auth.py",
          "tests/test_auth.py"
        ]
      },
      "verification": {
        "tests_passed": 42,
        "tests_failed": 0
      }
    }

diff --git a/TODO.md b/TODO.md
--- a/TODO.md
+++ b/TODO.md
@@ -12,7 +12,7 @@
 - [x] Done
-- [ ] My active task
+- [x] My active task
 - [x] criteria 1

diff --git a/src/auth.py b/src/auth.py
... (code changes) ...
```

---

## How Downstream AI Consumes This

### Debugging AI
```bash
git log --grep="auth.py" --pretty=format:"%H %s"
```
Output: `abc123 sync: 2 checked, 2 files`

Then queries the full commit to see:
- What tasks were checked off
- Which files were modified
- Test results

No prose to parse. No summaries to hallucinate. Just structured data.

### Project Manager AI
```bash
git log --grep="ledger_delta"
```
Finds all commits where the Worker reported its ledger state.

### Janitor AI (Ghost Code Finder)
```bash
git log --grep="sync: 0 checked" --pretty=format:"%H %s"
```
Finds commits where the Worker modified files but checked off zero TODOs.

Then reads the JSON body:
```bash
git show abc123 HEAD --format=json | jq .code_delta.files
```

### Revert AI (Suspicious Batch Detector)
```bash
git log --grep="sync: 5 checked, 2 files" --pretty=format:"%H %s"
```
Finds suspicious commits (5 checked, 2 files = batched work needs review).

---

## Edge Cases Handled

### 1. The Lazy Batch
Worker checks 5 TODOs but only edits 2 files.

```
Title: sync: 5 checked, 2 files
Body: ledger_delta.checked = ["a", "b", "c", "d", "e"]
       code_delta.files = ["auth.py"]
```

Downstream AI sees `5 checked` vs `2 files`. **Flag as suspicious.**

### 2. The Ghost Fix
Worker fixes `payments.js` but never opens `TODO.md`.

```
Title: sync: 0 checked, 1 files
Body: ledger_delta.checked = []
       code_delta.files = ["payments.js"]
```

Downstream AI sees `0 checked` but `1 file modified`. **Flag as unanchored.**

### 3. The Context Window Save
Worker runs out of tokens, leaves tests failing.

```
Title: sync: 2 checked, 2 files
Body: verification.tests_passed = 15
       verification.tests_failed = 2
```

CI/CD doesn't auto-revert because the title says `2 checked` — the Work completed. Tests failing is just bad luck, not incomplete work.

---

## Why Deterministic Beats AI for Commits

| Aspect | AI Commit (LLM) | Deterministic Commit (Script) |
|--------|----------------|--------------------------------|
| **Intent capture** | "feat: implement JWT" | `"checked": ["My active task"]` |
| **Queryability** | Can't grep intent | `git log --grep="TODO-42"` |
| **Hallucination** | High | None |
| **Compute cost** | High (per commit) | Zero (grep/regex) |
| **Downstream AI value** | Low (must parse prose) | High (structured data) |
| **System homeostasis** | Fragile (LLM drift) | Robust (deterministic) |

**The verdict:** Deterministic commits aren't "better" — they're the **only** way to build an AI-to-AI system that doesn't hallucinate.

AI commits are fine for human-readable history in a repo that humans might occasionally browse. But for AI-to-AI, deterministic commits are superior because:
1. They're queryable with standard Git tools
2. They don't hallucinate
3. They capture the TODO text in the JSON body (not lost)
4. Downstream AI can grep for exactly what it needs

---

## Why This Architecture Works

### 1. **No LLMs at the commit boundary**
- The commit is generated by deterministic extraction, not generative AI
- AI writes code. AI updates ledger. Environment audits.

### 2. **No prose in titles**
- Titles are routing keys: `sync: X checked, Y files`
- No `feat:`, `fix:`, `chore:` — those are human artifacts
- The TODO text is in the JSON body (not repeated in the title — DRY)

### 3. **No semantic scope matching**
- The Committer doesn't verify that `auth.py` matches "Implement JWT"
- It just reports raw counts and lets downstream AI judge
- Avoids fragile keyword matching and hallucination

### 4. **The Git log is a database**
- Downstream AI queries it programmatically
- `git log --grep="TODO-42"` finds exactly what it needs
- No prose to parse, no summaries to hallucinate

### 5. **The Worker is untrusted**
- Worker never has git push access
- Worker never runs `git commit`
- Worker is a sandbox, not a developer
- The Committer is the only entity with write access to the repo

---

## The Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  AI Worker (Untrusted)                                      │
│  - Edits files in sandbox                                   │
│  - Updates TODO.md                                          │
│  - Yields control                                           │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ yields
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  dracon-sync (Dumb Auditor/Committer)                        │
│                                                              │
│  1. Parse TODO.md diff → extract claims                    │
│     git diff HEAD~1 HEAD -- TODO.md | grep '^\+.*\[x\]'  │
│                                                              │
│  2. Parse code diff → extract proofs                       │
│     git diff --stat HEAD~1 HEAD                            │
│                                                              │
│  3. Generate routing key + JSON body                        │
│                                                              │
│  4. Run git add + git commit                               │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ produces
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Git Log (Database for Downstream AI)                         │
│  - Queryable via git log --grep                             │
│  - Machine-parseable JSON bodies                             │
│  - No prose, no summaries                                   │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ queries
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Downstream AI Agents                                        │
│  - Debugging AI: traces errors to commits                    │
│  - Project Manager AI: queries ledger state                  │
│  - Janitor AI: finds ghost code                               │
│  - Revert AI: identifies suspicious commits                   │
└─────────────────────────────────────────────────────────────┘
```

---

## The Final Verdict

This architecture captures the paradigm shift required for AI-to-AI version control:

1. **The Worker is untrusted** — it's a chaotic variable that can lie
2. **The Committer is dumb but honest** — it extracts raw data, doesn't interpret
3. **The Git log is a database** — downstream AI queries it programmatically
4. **No LLMs at the boundary** — the commit is a receipt, not a summary
5. **Deterministic commits are better** — they're queryable, don't hallucinate, and capture intent in the JSON body

**No LLMs. No prose. No guessing. Just data.**