# Project State

## Current Focus
ONE LINE: Extend Git repository discovery depth to 4 and refine directory exclusion logic.

## Completed
- [x] fix(git): increase recursive search depth from 2 to 4, allowing discovery of deeper nested repositories.
- [x] refactor(git): simplify exclusion checks by removing special‑case “vendor” handling, always skipping “objects” and excluded names, and postponing dot‑directory skipping until after work‑tree detection. This prevents premature exclusion of hidden directories that may contain valid repositories.
