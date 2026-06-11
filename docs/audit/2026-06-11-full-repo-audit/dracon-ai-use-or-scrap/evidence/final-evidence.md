# Final Dracon AI use-or-scrap evidence

## timestamp
2026-06-11T19:11:38+01:00

## removed directory exists?
no

## workspace members
[workspace]
members = [
    "dracon-sync",
    "dracon-system",
    "dracon-warden",
]
exclude = []
resolver = "2"

## current references to dracon-ai excluding historical audit/goal docs
./docs/public-readiness.md:72:- Former `dracon-ai/` CLI wrapper removed from this repo; validate `dracon-libs` AI runtime crates separately when touched.
./docs/public-release-branch/PUBLIC_RELEASE_PREP.md:157:Former `dracon-ai/` CLI wrapper removed from this repo; validate `dracon-libs` AI runtime crates separately when touched.
./.dracon/demon-migration-audit.md:19:| dracon-ai-lib | 0 refs | 0 refs | ✅ Clean |
./docs/public-release-plan.md:227:Former `dracon-ai/` CLI wrapper removed from this repo; validate `dracon-libs` AI runtime crates separately when touched.
./Cargo.toml:29:dracon-ai-runtime-contracts = { path = "../dracon-libs/contracts/crates/ai/dracon-ai-runtime-contracts" }
./.dracon/secret-audit-report.md:78:| dracon-ai-lib | ✅ Clean (.env encrypted with dracon-warden) |
./UTILITY_BOUNDARIES.md:63:- `dracon-ai` was removed from this repo as an orphaned CLI wrapper; AI runtime crates remain in `dracon-libs`.
./AGENTS.md:35:**Workspace policy:** the root Cargo workspace intentionally includes `dracon-sync`, `dracon-system`, and `dracon-warden` only. The former `dracon-ai/` CLI wrapper was removed from this repo; AI runtime crates live in `dracon-libs` and are validated with that sibling workspace.
./AGENTS.md:848:- `dracon-ai` standalone: removed from this repo; validate `dracon-libs` AI runtime crates separately when touched.

## fmt validation

## workspace test tail

## ai-runtime validation tail

     Running unittests src/lib.rs (target/debug/deps/dracon_ai_runtime_contracts-057eedfe71752742)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests ai_routing_runtime

running 1 test
test services/crates/ai/ai-routing-runtime/src/lib.rs - (line 14) ... ignored

test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests ai_runtime_adapters

running 1 test
test services/crates/ai/ai-runtime-adapters/src/lib.rs - (line 11) ... ignored

test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests ai_runtime_config

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests dracon_ai_contracts

running 1 test
test contracts/crates/ai/dracon-ai-contracts/src/lib.rs - (line 15) ... ignored

test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests dracon_ai_runtime_contracts

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


## recent commits
4a328c1c 1 file(s) [Cargo.toml] DELTA:+1/-3
68234aae 6 file(s) in docs [UTILITY_BOUNDARIES.md, AGENTS.md, docs/public-release-plan.md] DELTA:+12/-28
abe683f9 9 file(s) in docs,dracon-ai [docs/audit/2026-06-11-full-repo-audit/dracon-ai-use-or-scrap/evidence/pre-removal-dracon-ai-cli-snapshot.md, dracon-ai/Cargo.lock, dracon-ai/src/main.rs] DELTA:+3457/-4931 | NEW:evidence/pre-removal-dracon-ai-cli-snapshot.md DEL:dracon-ai/.gitattributes,dracon-ai/.gitignore,dracon-ai/BLUEPRINT.md,dracon-ai/Cargo.toml,dracon-ai/README.md,dracon-ai/dracon-ai.example.toml,src/main.rs
6bbde642 3 file(s) in docs [docs/audit/2026-06-11-full-repo-audit/dracon-ai-use-or-scrap/evidence/dracon-ai-source.md, docs/audit/2026-06-11-full-repo-audit/dracon-ai-use-or-scrap/evidence/consumer-reference-search.md, docs/audit/2026-06-11-full-repo-audit/dracon-ai-use-or-scrap/evidence/ai-runtime-inventory.md] DELTA:+665/-0 | NEW:evidence/ai-runtime-inventory.md,evidence/consumer-reference-search.md,evidence/dracon-ai-source.md
b10b2788 2 file(s) in docs [docs/audit/2026-06-11-full-repo-audit/dracon-ai-use-or-scrap/evidence/initial-inventory.md, docs/audit/2026-06-11-full-repo-audit/dracon-ai-use-or-scrap/evidence/dracon-ai-crate-inventory.md] DELTA:+6472/-0 | NEW:evidence/dracon-ai-crate-inventory.md,evidence/initial-inventory.md
799e26e6 1 file(s) in docs [docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/REPORT.md] DELTA:+96/-0 | NEW:branch-reconciliation/REPORT.md
916364d7 3 file(s) in docs [docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/final-inventory.json, docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/dracon-libs-post-state.md, docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/final-inventory.stderr] DELTA:+395/-0 | NEW:evidence/dracon-libs-post-state.md,evidence/final-inventory.json,evidence/final-inventory.stderr
109e110f 1 file(s) in docs [docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/approval-log.md] DELTA:+57/-0 | NEW:evidence/one-mil-girls-post-state.md,evidence/post-inventory.json,evidence/post-merge-state.md
