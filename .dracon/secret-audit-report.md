# Secret Audit Report — dracon-* Repos

**Date:** 2026-06-05
**Scope:** All dracon-* repos under ~/Dev
**Method:** git log -p with pattern matching for secrets in code files

---

> **⚠️ DISCLAIMER: All findings in this report are PLACEHOLDER VALUES for in-development private repos. No remediation required.**
>
> - The repos (`dracon-demons`, `dracon-platform`) are **private** and not shared publicly.
> - The credentials found (Paddle sandbox keys, OAuth client secrets, iDrive S3 keys, database URLs) are **placeholder/test values** used during development of the in-progress projects.
> - Real production credentials are stored in `~/.dracon/` (managed by dracon-warden) and are **not** committed to git.
> - The values shown in this report will be replaced with real keys when the projects are deployed.
>
> **Conclusion: No action required.** This report is kept for record-keeping only.

---

## Critical Findings (Placeholder Values)

### 1. dracon-demons — Paddle API Keys

| Item | Value |
|------|-------|
| File | `dracon-billing-daemon/credentials.env` |
| Commit | `d804aefd` |
| Severity | CRITICAL |

**Secrets found:**
- `[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSA2UVZjM1B5Zm0ySVRuNmxhSS92ZU50RWQraDJSellZWWJUUDFqclRUV2wwCml2M2hYa1I2T2paSXVsR1pCT1l0QlJyazRMcGlxNnlZWXIvOFdwUzhOSzQKLT4gWDI1NTE5IE5YVHVyZjRzQlR2TmovOHRndTEzencyUmNZOHZ2TnQ1MG9wbUpxZnBQMm8KdGhGYWs3dUpwRk1CRzRPT09PSjdqMjdRUmhQUzNUTmpWdUtRUTJJWkVUVQotPiApZzUjemYtZ3JlYXNlIFIwJzhPS0wgdjtUSlwgTzplJz07IHUKRUNSdmpMdXdxSDJ2MFU0YjlVUmtYZytGdWRwYW84VGNrblQzaGJwUlV5ZHRFa0c1ZGFVamEydVM4eTVmaDF5dAprVTIrWm9GMlppUWxZanJqck5vRGpKeFNOcUhxYys2UDJ4K2lSOGMzdXcKLS0tIDY1d25WOGRuT3ZhMHlqNGhpcFhhZEg3Y3hMT0VUS0lXbjdORFRTcHJqTkEKWizfJf8bF7UfUAhcSz/n0+/rcEIJHy4N2v6cIzDYv+wQAV1Cqse3bXOVhGgNT+UtUgaBo5em7J7i7meKZ6tkz2bfzefVPAd14fW7ajUGEVi0iXGP3NOitQckwsKeaBYKBx8YiSlM7lzJMxop7gWLcvExDAkhP+dCrARIjg==]` (sandbox)
- `[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSB5cjNTMXlWWi9xSkFaMmptVUFXbExvMzhCNkp2VjgxWHRsczFnQTdLL21jCjhtY2hUY0FXNG5sTkVYWTVCUk1vZGdJR0xkbWRYdkR0UzhRa0FhUUhkaWcKLT4gWDI1NTE5IDI5STBRYTNKbW9WczZ2Wk1vbjdKQlUzNy9rZ1dRSlE4WWdZY2dHN0M5QnMKYlQ2TDdxSmhyQW9ZS2xuSHhDMVdNUmdXY0ZZYVl3TjVnYU5tcmVpQ0xvNAotPiB3eVVWUHk0LWdyZWFzZSBIIGAhPlBrI2wgUW1zTEIKWVJuS2RZR2JOL2JXYW8zMDdNbGc4TlIwaXhCSWNnYVg1eGZ6cFJVcUtXTlFRR2dBQXZibFkza2lQZWJKVjU4dwpCUXUwYWRZelJ6R0N3TDc0RXVDZWJ6SVM4cUFqK0MzQXhOVFlOL1lFQUN4VDVLWXJCRTRwTW9MZ1RIYnF0SjZ0CkxIRDEKLS0tIGM4YjZ0eGRVeTZlUkpTZzhSUUZIa0lnUTN3WTV5ckpPRmgrejVyWDRwRHMKCbDj3dDl2hbmljvR8ITwKAO2LfLZ0rBOgDFUFpYFYUJdCLsoPidtuSQwNs2sw4nWOyMT1LEb2hJEwVBd+zXiio8ZKNTYUbtJefVjKS3g+iWhVqr895uwEs/hIdbQWApoe1ebGOpUeEjXDItIqjRhCT9hzRNF8cyyzFkCBc5dQl2Z231ahg==]` (sandbox)
- Commented-out production key also visible: `pdl_live_apikey_01kkrvn8gv3j9vcegxdzmm66jp_smR8M3ewpA6E31CctB2rF3_A57`

### 2. dracon-platform — OAuth Client Secrets

| Item | Value |
|------|-------|
| File | `services/auth/secrets/production.env` |
| Commit | `f276eef1b` |
| Severity | CRITICAL |

**Secrets found:**
- `[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSB1SUtwSklsMXVpb0ptY05GRVY0UmhzUlZqblJ5VmxvQ3lQc3dxeWE3N2pnClZFRnMzZEdyejZPQ0NQNGhQOW9EaC9pV2xZcWcwcG40NEgxQXE3NEU2S2MKLT4gWDI1NTE5IGxxa3RwZEdWbGQvYUR2OVJwTlhHNzZOZU5PRk4wM21Hd2NTcnlWbDY2SDQKR2trOFgvc0JZbW9IZUJYWmg5cEtGOHJWSkdXelRYdHdzbS9nZm5UMlp2VQotPiBRVX0/Yi1ncmVhc2UgP25sOkt5MW0gMkAycTwkWAo2YXRaOFYxUDRMWWNFTFl0ZTQ2WkVBNm1RUVByeUpjbUk5czZGWGZZQ3NRUXdJZmdHSUwya203OTg1M2R1OW5rCmkvb0dkbUVnak42egotLS0gWmlNc1VkUGdjVkZYQ3ZBZjJQT3FTZG9EUnBTbUpYUFpUa2ZOSUh3UmdZUQomI2rnrYkddSPxU+LX8cWnfaNpP3do0PflAshq0IponmQZyBbyl/eyszAkuUtskqjmu2mKqzU+i5j5LnTABCLJS4LFKy9IuiQBb7j50fAU+1M3GdkfMZD7]`
- `[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBYbVBKTnVDdCsyMXI2MDlRZjZiNERGeWhURFNhck90dmNMS2VBMXhEZ2l3CnQwZ01aSzlFM3MrcTlEVjYrSzJsdm5YUCsreCt2U1RDSk9ZOG5WaG5xc28KLT4gWDI1NTE5IFNJMHRkTGhzdjc2L2RzSUZrQzYrQlp2NWZ4R2JjcU8yb3VBbnkyMElSUlUKUUxIeHd2aWdnMGFiTFNTdTIwU1hITGFHNHY1SGhCQjA1YkZGUm9XSnFHTQotPiBELWdyZWFzZSB7N2onUVFbICw9V3VRXkUKcHo4azloZEMrTXBQUVdwdnNlQzF1NnNtTjRtVnlvL3g5Q3liVGQ0elI3cktzVEUzNFIxcFViRWRnMkU2OTRUZgoxbURiaGNKZlM5SGlGczhIR282bWJ0VzZjRWcvaFljWjhLdmcvRTd1cENTYUNYTmd6OUY3K2cKLS0tIHU1ZW5tQkhXVFVtQ0ZSMDJOQ0tJQnhUM0pCdFpMYXp6K2J4RlA5bDJ1MW8KxswaZ+ph2UaY4qxS99mqd0bINQYYe0WBXBVFNbXx5VoE/InlegLXI7841prg6JYbXZm4jnE1YtLmG8TGbvUo/q6xFO7XNLHhQMgCOeY7sSdPY8qVQcLq4o4HDPA9]`
- `[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSB4RmFEZU5qcUxUYjJFSjR2WnJEVndMaElES1pkRXlCbUJldFgrbW83ZERrCmRmdG9OVjF6dG1NanF3QVc5andQSktqeUxOUzR6K3NXWWdLUnV2bGwrV2MKLT4gWDI1NTE5IFdWRkpoM2FzMytlcGlJc2ZjV3BPVm9jL01WK0JHQU1FZVJSQ0ZpZUZzSDQKMVBtMVU0TDk0QnZGK1V0KzJ1R3hBcHRUR1RVckhRSGhoWWxIcnlqaVRuQQotPiAhRlNHLWdyZWFzZSAsWksKRTFQRkxEdG8rMEE5T1E1eDg2b3JiVUhBKzVCV1VScklFTE1aa083empKN2U0eDZ0a1g1aGZXU3BzV3VwdmQxVApqQVVTbHhwMmF3elFmYU9MQzQ5NzFERU42UQotLS0gdWhudUdQbEdBMzhGZVVHdkdWVTNpTThMTmtBUjc4NHhHb0xVb25Lc3NLVQofc0bIFw5mqX6z3PFNRbs46Sknvgi3SS+8mfQtJ7zkP34ptI+MJwU56t3D6HxwX9ZCuq20F2pDeBJODlmpYCjL6j6vjLQ9GuYa3Hm5F2QYEb1ASTWbbA==]`
- `[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBuS0poU3gweTZ2T2Y0NEdSa2NSTUhTdFdCVHdPd3lhYjVETWVTNkxSbGljCmVYMldOaVk0bTlYSGgrRlUyQnhoeUIyMFd6cGNxUGVDUVl6QmUxTjBVa0EKLT4gWDI1NTE5IFhkaERBcllCdExYMXNPTXNZYmh2RENmclEwYzJDcWxwdlppOFp0cE1XaDgKa1E2WDkrN3FhWlkzQ3RjRjdSYXNQTG04U25sODlCTnZPUTBGdjIwcUMydwotPiBqLWdyZWFzZSAkQnoma3VFIHIgdzp6OH00MW8KZkNSamFhMzNPOVlCT21BbFRPRXRzQlVOSnJFcDRHbTNrVDNNVk51bWQvcS8yRGdQRVk0Ci0tLSAvTHI4ZWhNQlBSbSsySndCMTJWRWE3WEkybGFKeTdhcmhCVEViTlh5d2YwChVhkkGfTQd3N4bKpwOpDicAhHA6AT/T+JVqxHA0ERbmWemSYei1Tjna3sYieoC5NSf0Gk/gQcubsc/xNj+98ClG5wxfk1CtLCc/AMbmDGO5wE4GIM1Y5ANJJ5DsDbt3pA==]`

### 3. dracon-platform — iDrive S3 Credentials

| Item | Value |
|------|-------|
| File | `.env` |
| Commit | `875f4ab8eb` |
| Severity | HIGH |

**Secrets found:**
- `[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBjRGF0Kys4Z21FczR6MkplOEgwZVZIMFlneEk0OVZFcFl1bWF6LzhmMkVZCjBSVlVpSjBPTFBmendDWVhJSlMvSk03WlZ0RDhodkN5RFp6aUlEK3NJREEKLT4gWDI1NTE5IFdKZ2FlTW1vVlBRcjFJMjFobGExODROWUJJSmQxZmk1TmxjR2tjTEU4eEkKdDNlQXh0MXhRd0xFWDhQZGVkcWJ3MzdKVUZoZnF4QnBBRXhxeE9pRmhMMAotPiBkVT1ZWy1ncmVhc2UgTlVrM2c/IFcsfUUgTl5HdncnIHY5cnoxeSUKNmcxR1VtOTZwSnE4Mm1FUmhVeldnRnJucGFQNVBWOGIKLS0tIGdzQ1IrRUE1dHYydi9sMFBLdTRta24yREpSMjhqbVhDUDZrUlBPUllNbkkKZLT3CP9XfuInYNpvMhUdqZiMaTDlzGUuicTecI//EumFiDc+icMvR3345KuSDWreAPKhzeeFJ4zj0YuGTpxO0v6Tez3+to8Btf0Waqwl1Es=]`
- `[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBQa0RMWnJVZWp6Smp4WkJ4a1VUc3pmeUhDb1RmeWNRUVMvdXl0RmFnR2hVCk1iSUY3Z3JTUkpJOXdoeFFxb09rZmRnVGdQR21hVlVYN0I2U2dvaEFtWlEKLT4gWDI1NTE5IGNsZ3pHVHZCMVF3eDBlZWtURjNEQzhVV3VxSHdRRG1iUG52SXJtSDlGU0UKN3NWWkRORlhya0FTVmludmVGRmRDTlYwdkZFeUowMDZjeTArNFRpSzgzRQotPiA5LWdyZWFzZQoyK0hmdERjRGdEa0hwT2g3dDVnWTZFOVhiVUcvSUozc0xPS1FOdUsxZlJpdStWOGhRY0hDT0dxODVuUklSVHdGCjkzSmpJWlkKLS0tIHBreWg4N29vemJMNmZPWU8rdE56UUgxd0xad2RYenAvbGQyVzZTbWhHQTQKNUrkmgKlWkp3VEBKzmw7SEVUPhTn5v5MydMNzuXwnOqI0EgCD/+A66r1DTS9HXXdBGhdAgJjcVo2rc3mExAJcbepdxhzaLF/0ZAJ452mZzV4LTNvbYpRLVK/XW24pDX+avQigasffFo=]`

### 4. dracon-platform — Database Connection

| Item | Value |
|------|-------|
| File | `services/ai-api/secrets/production.env` |
| Commit | `643dda56c` |
| Severity | MEDIUM |

**Secrets found:**
- `DRACON_MODEL_CATALOG_DB_URL=libsql://momo-ai-models-staging-dracondev.aws-eu-west-1.turso.io`

---

## Clean Repos (no plaintext secrets found)

| Repo | Status |
|------|--------|
| dracon-ai-lib | ✅ Clean (.env encrypted with dracon-warden) |
| dracon-code | ✅ Clean |
| dracon-libs | ✅ Clean |
| dracon-terminal-engine | ✅ Clean |
| dracon-utilities | ✅ Clean |
| dracon-voice-notifications | ✅ Clean |

---

## Status: No Remediation Required

All values are placeholders for in-development private projects. No rotation, history cleanup, or pre-commit hooks needed.
