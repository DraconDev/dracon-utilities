# Dracon-AI Blueprint

## Issues Fixed

### 1. JSON extraction could grab wrong content
- **Problem:** `rfind('}')` finds the last `}` which If AI response contains multiple JSON objects or `}` in text after JSON, this extracts incorrect content.
- **Fix:** Now uses bracket counting to find matching `}`
- **Priority:** High
- **Status:** [x]

### 2. Config parsing silently ignores errors
- **Problem:** Invalid TOML silently returns defaults
- **Fix:** Added warning on parse failure
- **Priority:** Medium
- **Status:** [x]

### 3. Unused variable `pinned_model`
- **Problem:** `pinned_model: Option<String> = None;` - dead code
- **Priority:** Low
- **Status:** [x] (documented as unused)

### 4. Missing RoutingTask::Writing variant in match
- **Problem:** Match statement at line 512 didn't cover `RoutingTask::Writing`, causing compile error
- **Fix:** Added `RoutingTask::Writing => "writing"` case
- **Priority:** High
- **Status:** [x]

### 5. intent_to_lane missing "writing" mapping
- **Problem:** `intent_to_lane()` didn't map writing-related intents to `RoutingTask::Writing`
- **Fix:** Added mapping for "writing", "write", "docs", "documentation"
- **Priority:** Medium
- **Status:** [x]

---

## Code Quality Notes

### Agent Loop Protection
- `repeat_guard` BTreeMap tracks command signatures to detect stuck agent loops
- After 3 identical plan repeats, agent stops with warning

### Dangerous Command Detection
- `is_dangerous_shell()` provides basic heuristics for sudo, rm, mv, --force, -rf, mkfs, dd
- Requires `--dangerous` flag or `DRACON_AI_DANGEROUS=1` to execute

### JSON Repair
- `strip_trailing_commas()` handles JSON5-style trailing commas from AI output
- Combined with bracket-counting JSON extraction for robust parsing

---

## Remaining Low Priority

- Dangerous command detection incomplete (consider more patterns detection)
- Shell command injection risk (AI executes arbitrary commands)
- No logging used despite dependency
- No graceful shutdown handling for streaming (Ctrl-C only stops stream)
