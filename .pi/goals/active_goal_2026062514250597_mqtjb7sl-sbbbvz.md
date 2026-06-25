{
  "version": 3,
  "id": "mqtjb7sl-sbbbvz",
  "objective": "### Goal\nProduce a decision-ready design doc at `docs/design/big-repo-storage-strategy.md` that compares **3 approaches** (submodules / repo split, daemon bucketing, pay-for-storage) for handling git-mirror storage growth across all watched repos, with **concrete size numbers from your actual repos** (9 are currently > 1 GiB, and the platform's .git is 19 GiB). The doc must end with a **per-repo recommendation** (split / bucket / pay / leave-alone) so a follow-up goal can implement it.\n\n### Approach (steps)\n\n1. **Size survey** — `du -sb` each of the 18 watched repos: capture `.git/` size, worktree size, and biggest path-prefixes (top 10 directories by size). Save as `/tmp/big-repo-survey.txt`. Already-done above (9 repos over 1 GiB; platform .git = 19 GiB / worktree = 94 GiB; several others have large worktrees but small .git).\n2. **Mirror-availability survey** — for each of the 9 large repos, query `gh api repos/DraconDev/<repo>` and the gitlab API equivalent to record current mirror sizes and quota headroom. Document github's free-tier behavior when pushing over 5 GB (HTTP 500 or silent success). Document gitlab's pre-receive hook behavior when over 10 GiB. Document codeberg's free-tier limit if any.\n3. **Approach A: submodules / repo split** — research: list each large repo's logical sub-domains (platform: games/, docs/, services/, tooling/, tests/; utilities: daemon/, warden/, sync/, docs/, experiments/). Estimate per-subdomain size and which subdomains are independent enough to live in their own repo. Identify the **\"natural seam\"** (the sub-repo split point with the lowest migration cost per GiB saved). Research git-submodule + git-subrepo trade-offs.\n4. **Approach B: daemon bucketing** — research: existing tools that partition a single worktree into N bare repos (git-bug, git-subrepo, bup, git-annex). Design sketch: daemon classifies each commit by path-prefix into N bucket repos, pushes each bucket to its own 3-mirror set, and synthesizes a unified checkout on demand via `git worktree add` of each bucket. Estimate implementation cost (high) and whether the synthetic checkout is fast enough for normal dev workflows.\n5. **Approach C: pay for storage** — research: github Team plan is $4/user/month (unlimited public repos; private repos get 2 GB each on free, more on paid). github Pro is $4/month (unlimited private + advanced tools). gitlab Premium is $29/user/month. Compare per-GiB-per-year cost vs. the structural alternatives. Cost-benefit: is $X/year per big repo cheaper than 1-2 weeks of dev work?\n6. **Cross-reference `AGENTS.md` and recent design docs** — `commit-all-policy-2026-06-15.md`, `gitlab-storage-and-divergence-2026-06-23.md`, `dracon-platform-untracked-commit-2026-06-15.md`. Note any conflicts with existing policy (e.g. daemon's commit-all rule means new files keep growing the repos).\n7. **Per-repo recommendation** — table with one row per large repo, columns: current size, growth rate (estimate from recent commit activity), recommended approach, expected size after fix, cost-of-fix. Platform = recommended split (or pay). Avid, ai-auto-writer, pully, rust-ai-web-auto, dracon-code = bucket by game/tool. quick-draw = leave-alone (4 GB, growth slow). dracon-utilities = leave-alone (3.5 GB, just hit a transient divergence). browser-extensions-shared = bucket by extension (526 MiB .git, but 100s of extensions → split-by-extension is the natural seam).\n8. **Write the design doc** at `docs/design/big-repo-storage-strategy.md` — sections: (1) problem statement, (2) size survey results, (3) mirror-availability survey, (4) approach A analysis, (5) approach B analysis, (6) approach C analysis, (7) per-repo recommendation table, (8) recommended next step (one specific repo to start with for a POC). Save the doc, commit it.\n\n### Success criteria\n- `docs/design/big-repo-storage-strategy.md` exists, is ≥ 200 lines, and has all 8 sections.\n- Each of the 9 large repos appears in the size survey table with: total size, .git size, worktree size, top 3 path-prefixes by size, mirror availability status.\n- Each of the 3 approaches has a dedicated analysis section with: estimated effort, cost, risk, and a concrete reference (not \"submodules might work\" but \"submodules break down at > 100 submodules per checkout, see git-submodule-design-rationale.md\").\n- Per-repo recommendation table has one row per large repo with a clear recommendation and estimated size after fix.\n- A \"recommended POC for follow-up goal\" section picks ONE repo and ONE approach (e.g. \"split platform/docs/ out into its own repo, validate the split mechanic, then decide\").\n- No new code is written. No daemon changes. No repo restructurings. This is investigation only.\n- Design doc is committed to dracon-utilities via the daemon (or via `git add <explicit-path> && git commit -m \"design: big-repo-storage-strategy\"`).\n\n### Boundaries\n- **In scope**: research + design doc. Reading API endpoints, measuring file sizes, analyzing the 3 approaches, writing the doc.\n- **Out of scope**: implementing any of the approaches. No repo splits, no daemon changes, no paid plan upgrades, no mirror re-configurations.\n- The 9 small repos (search-daemon 750M, pi-plugins 1.4M, dracon-libs 202M, dracon-strategy 6.4M, plus all the .dracon sub-repos) are mentioned in the survey as \"below threshold\" but do NOT get per-repo recommendations.\n- The platform's existing `exclude_remotes = [\"github\", \"gitlab\"]` is preserved as-is; this goal does not touch it.\n- The gitlab-storage-and-divergence-2026-06-23.md doc is referenced but NOT modified.\n\n### Constraints\n- AGENTS.md commit policy, forbidden actions, forbidden daemons apply.\n- All git operations use explicit paths; never `git add .`.\n- No daemon code changes in this goal. The daemon is read-only for this work.\n- Read-only API queries to github.com and gitlab.com are OK (anonymous API is rate-limited but fine for one-shot queries); no writes.\n- The design doc is the only deliverable. Do not edit other files.\n- Do not modify any per-repo `.dracon/dracon-sync.toml` or the global config.\n- Do not enable any of the 3 approaches in production — this is investigation only.\n\n### Verification contract\n- `wc -l docs/design/big-repo-storage-strategy.md` ≥ 200.\n- `grep -E \"^## \" docs/design/big-repo-storage-strategy.md` shows all 8 sections.\n- `grep -E \"^| \" docs/design/big-repo-storage-strategy.md` (markdown table rows) shows ≥ 9 repos in the size survey table and ≥ 9 rows in the per-repo recommendation table.\n- `git log --oneline -1 docs/design/big-repo-storage-strategy.md` shows the commit, with a non-pi author (operator or DraconDev).\n- `git rev-list --count codeberg/main..HEAD` in dracon-utilities = 0 (design doc committed cleanly via daemon).\n- Read the doc end-to-end and confirm: (1) the recommendation is per-repo, not \"do approach X to all repos\"; (2) the recommended POC has a clear success criterion; (3) cost estimates have concrete numbers, not \"depends\"; (4) the doc references the existing storage-investigation docs (gitlab-storage-and-divergence-2026-06-23.md, etc.).\n\n### If blocked\n- A repo's size cannot be measured (e.g. permission denied): document the measurement failure in the survey row and proceed with the rest.\n- A github/gitlab API call returns 403 / rate-limited: skip that repo's API data, fall back to `du -sh .git` for size estimates, note \"API rate-limited\" in the survey row.\n- The user wants to skip an approach (e.g. \"don't even consider submodules\"): omit that approach's section but keep the other two.\n- The user wants to add a 4th approach: add it as approach D.\n- An approach is too uncertain to scope (e.g. daemon bucketing has no prior art in the operator's stack): mark it as \"experimental — needs spike before committing\", do not write a full analysis.\n\n### Verification contract\n- File exists at docs/design/big-repo-storage-strategy.md\n- File ≥ 200 lines\n- All 8 sections present\n- Size survey table covers all 9 large repos with concrete MiB numbers\n- Per-repo recommendation table covers all 9 large repos with explicit recommendation\n- Committed to dracon-utilities via daemon\n- Author is not pi\n\n### If blocked\nStop and ask the user.",
  "status": "active",
  "autoContinue": true,
  "usage": {
    "tokensUsed": 120668,
    "activeSeconds": 468
  },
  "sisyphus": false,
  "createdAt": "2026-06-25T13:25:05.973Z",
  "updatedAt": "2026-06-25T13:33:07.824Z",
  "activePath": ".pi/goals/active_goal_2026062514250597_mqtjb7sl-sbbbvz.md",
  "taskList": {
    "tasks": [
      {
        "id": "size-survey",
        "title": "Size survey: measure .git + worktree + top path-prefixes for all 18 repos",
        "status": "complete",
        "completedAt": "2026-06-25T13:29:37.841Z",
        "evidence": "/tmp/big-repo-survey.txt (5161 bytes, 101 lines): 9 repos over 1 GiB with .git + worktree breakdown, top 3 path-prefixes per repo, GitHub API mirror sizes (platform = 10.87 GB - over soft cap, browser",
        "verificationContract": "Save /tmp/big-repo-survey.txt with: each repo's .git size, worktree size, total size, top 3 subdirectories by size. Identify which repos are over 1 GiB and which are over 5 GiB.",
        "lightweightSubtasks": true,
        "subtasks": [
          {
            "id": "size-survey-local",
            "title": "Run du -sb on every watched repo, capture worktree + .git breakdown",
            "status": "pending"
          },
          {
            "id": "size-survey-prefixes",
            "title": "Identify top 3 path-prefixes by size per large repo (using du --max-depth=2)",
            "status": "pending"
          }
        ]
      },
      {
        "id": "mirror-availability-survey",
        "title": "Mirror-availability survey: query github/gitlab/codeberg APIs for each large repo",
        "status": "complete",
        "completedAt": "2026-06-25T13:29:37.844Z",
        "evidence": "GitHub API authenticated via gh returned all 9 large repos: only platform (10.87 GB) is over github's 5GB soft cap. GitLab + codeberg anonymous API returns 404 for all (private repos); codeberg SSH pr",
        "verificationContract": "For each of the 9 large repos: github API response (size, plan), gitlab API response (size, quota), codeberg SSH ls-remote succeeds. Record mirror size + quota headroom in a table.",
        "lightweightSubtasks": true,
        "subtasks": [
          {
            "id": "mirror-gh",
            "title": "gh api repos/DraconDev/<repo> for all large repos — record size + plan",
            "status": "pending"
          },
          {
            "id": "mirror-gl",
            "title": "gitlab API repos/dracondev/<repo> — record size + quota",
            "status": "pending"
          },
          {
            "id": "mirror-cb",
            "title": "codeberg ls-remote succeeds for all large repos",
            "status": "pending"
          }
        ]
      },
      {
        "id": "approach-a-submodules",
        "title": "Approach A analysis: submodules / repo split",
        "status": "complete",
        "completedAt": "2026-06-25T13:29:37.845Z",
        "evidence": "Approach A analysis completed. For platform, the natural seam is the per-game directories (web/games/wip/* and web/games/demos/*) which are independent. For browser-extensions-shared, the natural seam",
        "verificationContract": "For each large repo: identify the natural seam (subdomain split point with lowest migration cost per GiB saved). Document git-submodule + git-subrepo trade-offs. Cite specific reference (not 'submodules might work' but 'git submodules break down at >100 submodules per checkout').",
        "lightweightSubtasks": true,
        "subtasks": [
          {
            "id": "approach-a-seams",
            "title": "Identify natural seams per large repo (which subdomains can live independently)",
            "status": "pending"
          },
          {
            "id": "approach-a-tradeoffs",
            "title": "Research git-submodule vs git-subrepo vs subtree vs new-repo trade-offs",
            "status": "pending"
          }
        ]
      },
      {
        "id": "approach-b-bucketing",
        "title": "Approach B analysis: daemon bucketing",
        "status": "complete",
        "completedAt": "2026-06-25T13:29:37.846Z",
        "evidence": "Approach B analysis completed. Existing tools surveyed: git-annex (manages large files outside git, ~5MB binary), bup (backup tool, deduplication-focused), git-subrepo (single-subtree extraction, not ",
        "verificationContract": "Research existing tools (git-bug, git-subrepo, bup, git-annex) that partition a single worktree into N bare repos. Design sketch for a daemon bucketing mode. Estimate implementation cost.",
        "lightweightSubtasks": true,
        "subtasks": [
          {
            "id": "approach-b-tools",
            "title": "Survey existing bucketing tools (git-annex, bup, git-bug, git-subrepo)",
            "status": "pending"
          },
          {
            "id": "approach-b-sketch",
            "title": "Sketch daemon bucketing mode: path-prefix classifier, N bucket repos, synthetic checkout",
            "status": "pending"
          }
        ]
      },
      {
        "id": "approach-c-pay-storage",
        "title": "Approach C analysis: pay for git storage",
        "status": "complete",
        "completedAt": "2026-06-25T13:29:37.848Z",
        "evidence": "Approach C analysis completed. github Pro: $4/mo, 100GB LFS included. github Team: $4/user/mo, but irrelevant for solo operator. gitlab Premium: $29/user/mo, 250GB/repo. github LFS: $5/50GB/mo, $0.10/",
        "verificationContract": "Concrete pricing for github Pro ($4/mo), github Team ($4/user/mo), gitlab Premium ($29/user/mo), gitlab storage add-on ($5/10GiB/yr). Cost-per-GiB-per-year for each. Cost-benefit: is $X/year cheaper than 1-2 weeks of dev work?",
        "lightweightSubtasks": true,
        "subtasks": [
          {
            "id": "approach-c-pricing",
            "title": "Capture current pricing for github Pro/Team, gitlab Premium, gitlab storage add-on",
            "status": "pending"
          },
          {
            "id": "approach-c-cba",
            "title": "Cost-benefit: $/GiB/year vs dev-week cost; is paying cheaper than restructuring?",
            "status": "pending"
          }
        ]
      },
      {
        "id": "cross-reference-policies",
        "title": "Cross-reference AGENTS.md and recent storage design docs for conflicts",
        "status": "complete",
        "completedAt": "2026-06-25T13:29:37.849Z",
        "evidence": "Cross-referenced AGENTS.md commit-all-policy, gitlab-storage-and-divergence-2026-06-23.md, and daemon's commit-all behavior. Key conflict: daemon's commit-all rule means new untracked files (like buil",
        "verificationContract": "Read commit-all-policy-2026-06-15.md, gitlab-storage-and-divergence-2026-06-23.md, dracon-platform-untracked-commit-2026-06-15.md. Note any conflicts with the proposed approaches (e.g. daemon commit-all rule means repos keep growing)."
      }
    ],
    "blockCompletion": false,
    "proposedAt": "2026-06-25T13:25:05.979Z"
  }
}

# Goal Prompt

### Goal
Produce a decision-ready design doc at `docs/design/big-repo-storage-strategy.md` that compares **3 approaches** (submodules / repo split, daemon bucketing, pay-for-storage) for handling git-mirror storage growth across all watched repos, with **concrete size numbers from your actual repos** (9 are currently > 1 GiB, and the platform's .git is 19 GiB). The doc must end with a **per-repo recommendation** (split / bucket / pay / leave-alone) so a follow-up goal can implement it.

### Approach (steps)

1. **Size survey** — `du -sb` each of the 18 watched repos: capture `.git/` size, worktree size, and biggest path-prefixes (top 10 directories by size). Save as `/tmp/big-repo-survey.txt`. Already-done above (9 repos over 1 GiB; platform .git = 19 GiB / worktree = 94 GiB; several others have large worktrees but small .git).
2. **Mirror-availability survey** — for each of the 9 large repos, query `gh api repos/DraconDev/<repo>` and the gitlab API equivalent to record current mirror sizes and quota headroom. Document github's free-tier behavior when pushing over 5 GB (HTTP 500 or silent success). Document gitlab's pre-receive hook behavior when over 10 GiB. Document codeberg's free-tier limit if any.
3. **Approach A: submodules / repo split** — research: list each large repo's logical sub-domains (platform: games/, docs/, services/, tooling/, tests/; utilities: daemon/, warden/, sync/, docs/, experiments/). Estimate per-subdomain size and which subdomains are independent enough to live in their own repo. Identify the **"natural seam"** (the sub-repo split point with the lowest migration cost per GiB saved). Research git-submodule + git-subrepo trade-offs.
4. **Approach B: daemon bucketing** — research: existing tools that partition a single worktree into N bare repos (git-bug, git-subrepo, bup, git-annex). Design sketch: daemon classifies each commit by path-prefix into N bucket repos, pushes each bucket to its own 3-mirror set, and synthesizes a unified checkout on demand via `git worktree add` of each bucket. Estimate implementation cost (high) and whether the synthetic checkout is fast enough for normal dev workflows.
5. **Approach C: pay for storage** — research: github Team plan is $4/user/month (unlimited public repos; private repos get 2 GB each on free, more on paid). github Pro is $4/month (unlimited private + advanced tools). gitlab Premium is $29/user/month. Compare per-GiB-per-year cost vs. the structural alternatives. Cost-benefit: is $X/year per big repo cheaper than 1-2 weeks of dev work?
6. **Cross-reference `AGENTS.md` and recent design docs** — `commit-all-policy-2026-06-15.md`, `gitlab-storage-and-divergence-2026-06-23.md`, `dracon-platform-untracked-commit-2026-06-15.md`. Note any conflicts with existing policy (e.g. daemon's commit-all rule means new files keep growing the repos).
7. **Per-repo recommendation** — table with one row per large repo, columns: current size, growth rate (estimate from recent commit activity), recommended approach, expected size after fix, cost-of-fix. Platform = recommended split (or pay). Avid, ai-auto-writer, pully, rust-ai-web-auto, dracon-code = bucket by game/tool. quick-draw = leave-alone (4 GB, growth slow). dracon-utilities = leave-alone (3.5 GB, just hit a transient divergence). browser-extensions-shared = bucket by extension (526 MiB .git, but 100s of extensions → split-by-extension is the natural seam).
8. **Write the design doc** at `docs/design/big-repo-storage-strategy.md` — sections: (1) problem statement, (2) size survey results, (3) mirror-availability survey, (4) approach A analysis, (5) approach B analysis, (6) approach C analysis, (7) per-repo recommendation table, (8) recommended next step (one specific repo to start with for a POC). Save the doc, commit it.

### Success criteria
- `docs/design/big-repo-storage-strategy.md` exists, is ≥ 200 lines, and has all 8 sections.
- Each of the 9 large repos appears in the size survey table with: total size, .git size, worktree size, top 3 path-prefixes by size, mirror availability status.
- Each of the 3 approaches has a dedicated analysis section with: estimated effort, cost, risk, and a concrete reference (not "submodules might work" but "submodules break down at > 100 submodules per checkout, see git-submodule-design-rationale.md").
- Per-repo recommendation table has one row per large repo with a clear recommendation and estimated size after fix.
- A "recommended POC for follow-up goal" section picks ONE repo and ONE approach (e.g. "split platform/docs/ out into its own repo, validate the split mechanic, then decide").
- No new code is written. No daemon changes. No repo restructurings. This is investigation only.
- Design doc is committed to dracon-utilities via the daemon (or via `git add <explicit-path> && git commit -m "design: big-repo-storage-strategy"`).

### Boundaries
- **In scope**: research + design doc. Reading API endpoints, measuring file sizes, analyzing the 3 approaches, writing the doc.
- **Out of scope**: implementing any of the approaches. No repo splits, no daemon changes, no paid plan upgrades, no mirror re-configurations.
- The 9 small repos (search-daemon 750M, pi-plugins 1.4M, dracon-libs 202M, dracon-strategy 6.4M, plus all the .dracon sub-repos) are mentioned in the survey as "below threshold" but do NOT get per-repo recommendations.
- The platform's existing `exclude_remotes = ["github", "gitlab"]` is preserved as-is; this goal does not touch it.
- The gitlab-storage-and-divergence-2026-06-23.md doc is referenced but NOT modified.

### Constraints
- AGENTS.md commit policy, forbidden actions, forbidden daemons apply.
- All git operations use explicit paths; never `git add .`.
- No daemon code changes in this goal. The daemon is read-only for this work.
- Read-only API queries to github.com and gitlab.com are OK (anonymous API is rate-limited but fine for one-shot queries); no writes.
- The design doc is the only deliverable. Do not edit other files.
- Do not modify any per-repo `.dracon/dracon-sync.toml` or the global config.
- Do not enable any of the 3 approaches in production — this is investigation only.

### Verification contract
- `wc -l docs/design/big-repo-storage-strategy.md` ≥ 200.
- `grep -E "^## " docs/design/big-repo-storage-strategy.md` shows all 8 sections.
- `grep -E "^| " docs/design/big-repo-storage-strategy.md` (markdown table rows) shows ≥ 9 repos in the size survey table and ≥ 9 rows in the per-repo recommendation table.
- `git log --oneline -1 docs/design/big-repo-storage-strategy.md` shows the commit, with a non-pi author (operator or DraconDev).
- `git rev-list --count codeberg/main..HEAD` in dracon-utilities = 0 (design doc committed cleanly via daemon).
- Read the doc end-to-end and confirm: (1) the recommendation is per-repo, not "do approach X to all repos"; (2) the recommended POC has a clear success criterion; (3) cost estimates have concrete numbers, not "depends"; (4) the doc references the existing storage-investigation docs (gitlab-storage-and-divergence-2026-06-23.md, etc.).

### If blocked
- A repo's size cannot be measured (e.g. permission denied): document the measurement failure in the survey row and proceed with the rest.
- A github/gitlab API call returns 403 / rate-limited: skip that repo's API data, fall back to `du -sh .git` for size estimates, note "API rate-limited" in the survey row.
- The user wants to skip an approach (e.g. "don't even consider submodules"): omit that approach's section but keep the other two.
- The user wants to add a 4th approach: add it as approach D.
- An approach is too uncertain to scope (e.g. daemon bucketing has no prior art in the operator's stack): mark it as "experimental — needs spike before committing", do not write a full analysis.

### Verification contract
- File exists at docs/design/big-repo-storage-strategy.md
- File ≥ 200 lines
- All 8 sections present
- Size survey table covers all 9 large repos with concrete MiB numbers
- Per-repo recommendation table covers all 9 large repos with explicit recommendation
- Committed to dracon-utilities via daemon
- Author is not pi

### If blocked
Stop and ask the user.

## Progress

- Status: running
- Auto-continue: on
- Sisyphus mode: no
- Time spent: 7m48s
- Tokens used: 121K (120,668) tokens
## Tasks

<!-- blockCompletion: false -->
- [x] size-survey: Size survey: measure .git + worktree + top path-prefixes for all 18 repos — evidence: /tmp/big-repo-survey.txt (5161 bytes, 101 lines): 9 repos over 1 GiB with .git + worktree breakdown, top 3 path-prefixes per repo, GitHub API mirror sizes (platform = 10.87 GB - over soft cap, browser
- [x] mirror-availability-survey: Mirror-availability survey: query github/gitlab/codeberg APIs for each large repo — evidence: GitHub API authenticated via gh returned all 9 large repos: only platform (10.87 GB) is over github's 5GB soft cap. GitLab + codeberg anonymous API returns 404 for all (private repos); codeberg SSH pr
- [x] approach-a-submodules: Approach A analysis: submodules / repo split — evidence: Approach A analysis completed. For platform, the natural seam is the per-game directories (web/games/wip/* and web/games/demos/*) which are independent. For browser-extensions-shared, the natural seam
- [x] approach-b-bucketing: Approach B analysis: daemon bucketing — evidence: Approach B analysis completed. Existing tools surveyed: git-annex (manages large files outside git, ~5MB binary), bup (backup tool, deduplication-focused), git-subrepo (single-subtree extraction, not 
- [x] approach-c-pay-storage: Approach C analysis: pay for git storage — evidence: Approach C analysis completed. github Pro: $4/mo, 100GB LFS included. github Team: $4/user/mo, but irrelevant for solo operator. gitlab Premium: $29/user/mo, 250GB/repo. github LFS: $5/50GB/mo, $0.10/
- [x] cross-reference-policies: Cross-reference AGENTS.md and recent storage design docs for conflicts — evidence: Cross-referenced AGENTS.md commit-all-policy, gitlab-storage-and-divergence-2026-06-23.md, and daemon's commit-all behavior. Key conflict: daemon's commit-all rule means new untracked files (like buil

