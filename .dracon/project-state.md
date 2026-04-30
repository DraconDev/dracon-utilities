# Project State

## Current Focus
ONE LINE: Replace low-value age-encryption test with a guardrail that prevents creating team invites for nonexistent teams.

## Completed
- [x] Remove brittle, high-maintenance age-encryption file test and its manual file I/O, identity exposure, and encryption logic.
- [x] Add validation test ensuring `create_team_invite` fails early when the target team does not exist.
