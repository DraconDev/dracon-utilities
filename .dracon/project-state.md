# Project State

## Current Focus
Add validation tests to WardenPolicy ensuring plaintext patterns are allowlisted and do not overlap with protected patterns or contain sensitive keywords.

## Completed
- [x] Add unit tests for valid policy acceptance and overlapping pattern rejection.
- [x] Enforce allowlist for plaintext patterns (e.g., only *.pub and Cargo.lock permitted).
- [x] Reject plaintext patterns containing sensitive keywords (e.g., "password").
- [x] Update dependency lockfiles for dracon-sync and dracon-system.
