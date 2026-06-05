{
  "version": 3,
  "id": "mq127fpq-ajghlm",
  "objective": "Polish CLI output across all three dracon utilities — remove stale status fields, unify status presentation, and improve the repos table with actionable hints.",
  "status": "active",
  "autoContinue": true,
  "usage": {
    "tokensUsed": 714,
    "activeSeconds": 0
  },
  "sisyphus": false,
  "createdAt": "2026-06-05T15:08:43.214Z",
  "updatedAt": "2026-06-05T15:08:43.219Z",
  "activePath": ".pi/goals/active_goal_2026060516084321_mq127fpq-ajghlm.md",
  "taskList": {
    "tasks": [
      {
        "id": "task-1",
        "title": "Remove stale warden service row from dracon-system status",
        "status": "pending",
        "verificationContract": "dracon-system status no longer shows 'warden service' row. Builds and tests pass."
      },
      {
        "id": "task-2",
        "title": "Upgrade dracon-warden status to table format matching dracon-system style",
        "status": "pending",
        "verificationContract": "dracon-warden status outputs a comfy-table with emoji headers, same visual style as dracon-system status. Builds."
      },
      {
        "id": "task-3",
        "title": "Add hint column to repos table showing actionable info per repo",
        "status": "pending",
        "verificationContract": "repos table shows a HINT column with per-repo actionable text (e.g. 'push pending', 'no upstream', 'merge conflict'). Builds and repos command runs under 1s."
      }
    ],
    "blockCompletion": false,
    "proposedAt": "2026-06-05T15:08:43.216Z"
  }
}

# Goal Prompt

Polish CLI output across all three dracon utilities — remove stale status fields, unify status presentation, and improve the repos table with actionable hints.

## Progress

- Status: running
- Auto-continue: on
- Sisyphus mode: no
- Time spent: 0s
- Tokens used: 714 tokens
## Tasks

<!-- blockCompletion: false -->
- [ ] task-1: Remove stale warden service row from dracon-system status — contract: dracon-system status no longer shows 'warden service' row. Builds and tests pass.
- [ ] task-2: Upgrade dracon-warden status to table format matching dracon-system style — contract: dracon-warden status outputs a comfy-table with emoji headers, same visual style as dracon-system status. Builds.
- [ ] task-3: Add hint column to repos table showing actionable info per repo — contract: repos table shows a HINT column with per-repo actionable text (e.g. 'push pending', 'no upstream', 'merge conflict'). Builds and repos command runs under 1s.

