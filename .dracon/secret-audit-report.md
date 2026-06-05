# Secret Audit Report — dracon-* Repos

**Date:** 2026-06-05
**Scope:** All dracon-* repos under ~/Dev
**Method:** git log -p with pattern matching for secrets in code files

---

## Critical Findings

### 1. dracon-demons — Paddle API Keys

| Item | Value |
|------|-------|
| File | `dracon-billing-daemon/credentials.env` |
| Commit | `d804aefd` |
| Severity | CRITICAL |

**Secrets found:**
- `[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBYRThpSWEyUnhmdFVxb1E2YmhGTEtOU25kc1JWekRJckhhTkpaVFZnM1RVClcyb2NmQmdpVlBWbHlqRHhIenJzL0hmUHNlYVBkaUhHNVRTU2VoSG13ZTgKLT4gWDI1NTE5IGxJMnloeG1sTUpYZzNDNnhtSmh1V3NkZWNJK3p5ZUNscXgwNWlaL1oyencKbGpvMzV3YmtLQ0xjMkxTRGpub09QVU5OUEdSL21neHN5Q0lMU2xHak1xRQotPiBvZH4tZ3JlYXNlIChQdnt2IGpaIEQKeWdodHVmeWZBcGZjZEpLbmRrQ051ZzRPVm5Ma0xoVnZVMjhXZTgvVAotLS0gRFRYdzFTUGhyNjBxcDZxNU43RGNTaWJIc245c1k4WGo3K0VaWkNYZEhPSQpewcSjP6GeMKwkYUxWeSXlwpCVAzOjeMiIylcgJ3YeskQjXZjzkSh9h2mSdNepQx+9o9lgrhdr4pnLvSG2xY4kCjPlAzYcPvCaeYaEmoox2D0Y99/PG1GaVY7qdr5P7lsyhki8IaPpW6Fg84HxpWeWmrUtTmt5JWZbseNx]` (sandbox)
- `[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBtcDd1U2dyMEJYVHRGejd5Y1QzbkI5REVLU1ZrK1FrSHJZRDdDZ0hXQWlZClBiUU1nV2R4aGN4bjQvSjlFenQxbkFCSzFKa291c0lnb0h1ODlvdUxKT0kKLT4gWDI1NTE5IFAwd3JQZ2VnT0g3ZDZMcHg5b04vdXpxdEppWC93cXFqSHpkWDg1N1ovUWcKUlNBM1J3anQrdG1VZDZDd2U4OHg4UTRuZXc5a0ljOUtvR2ZHQ052UWxMUQotPiBFcDwtZ3JlYXNlIHRcayB8ZyArZ1VEaG8KMEU4SldDUHN5WjBiRkpkeFZFNFY4c1kKLS0tIEtQZmlTSkxOb3h4dXM0aEwwM3dhUmUxRE50WTBiVWN2YzR1aDZ4QUlobFUK7WfNDL1DrPf7YzD2rzg3kpABfE6lbhi9P2OZLr5eLQpaGHqK9R1GsCpfHuQXMFGMiipY4hRj7OnU/EUuLDEDrfCoO+Fn6pgsSIK/EEbFKZlD3VR8u+8Dek+s2V+PxzyClmsxZO5mn/4eE5lILnwaPzng3QEnLE90kiOXCkQKdfFfL4fDhw==]` (sandbox)
- Commented-out production key also visible: `pdl_live_apikey_01kkrvn8gv3j9vcegxdzmm66jp_smR8M3ewpA6E31CctB2rF3_A57`

### 2. dracon-platform — OAuth Client Secrets

| Item | Value |
|------|-------|
| File | `services/auth/secrets/production.env` |
| Commit | `f276eef1b` |
| Severity | CRITICAL |

**Secrets found:**
- `[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBlMytUaWhkSFlzMnIydU1XVHVob1JrL285bGVycmRyNUs3ZitiTno4bld3CllOWUtCMTJwdkwvOGkwQmtmMW5KcVJQckZSRUNvVmZ2bkVXQWovSVdReWcKLT4gWDI1NTE5IDlsZDZ1Qm93UG9Zcm1UYlIwcHA0aE90ZUJqeXlCbXZxSVVLRlp0OElvd2MKbmJaWlgwclpMRGErWnhxcDBTOWkrLzg1ZDNYWHZjRmtaWVpqWHE2Zkg5UQotPiAtSUgtZ3JlYXNlIDFvCmZZcWdrYUp6V2JQWjNRd2UvZVhUVHhnVHY2Q0ZuWnIyVjBnZVZJTFBRV09jZTZyVTVWbmlGWm9tSFZtazNHQ2EKdm9YekVRVU1iVjBndlUwN0REd25hbGRhbmM2SmFWVU5yaW8KLS0tIEkyV2FYKzdDc0lodjFNa2xnNUFUdVpvdGRRQ2M3c3ZUZHZtZ0prZG9oN00KR3LqX4jnwReUbZlQO4FhHtOB180vlXiOF3XPbO2uz9vI7YpKdDN6Ngs2IyCEXGGpn+xeSaRSnBU3j3a2PrIahUnulPsPfOZO8tyYeGN0pXXnMxORqfgR8g==]`
- `[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBla3lPMERGMms2Q2t5djhLcjZnZVRPeEQzNEVLdzhiRU9ROE5qcSt1Q0djCkZNY3ZVREpOVWV0V2pzYk9DSlF4alB2V3ZjWTEwZjhpMlJMajk1UmJRbFkKLT4gWDI1NTE5IEs0QUxmQ3NROTFEa3Q0bWNKNDltWlBia05NQ1diVEY4SVIxaGJjbVh1d2MKVmQxSmxZMk85ejhpUExFVmJud09OQkxqSnhGdlVjVlNWMDR6aVF3SkRPSQotPiA9aC4tZ3JlYXNlIEN5PXcgY3kqIGN6X1kgL0lxNEguCjBYWGp1OE9IRHRsZFViVGZVekx0eWkwaFhVR2xGVWJYYTZMemNiUkNVZmV4SVVpN2w5Sy8yVndhUnRSd2IrcGoKUmc4dmVRCi0tLSA1MUNSTXJhbVlOdlJwTE1lcmlsTmE5d0VSdEY5YUtDbFRVbWZTcnRxTkFjChwYzDSiJItZhTykj+cy+Te0Qnf0xkZFOIHqqsIKTBk6b/Yq+aZ9u1fVAyoxGHwG445zJQiY3/+Mob8di/f9x/fzpnoCVcnYTx4rtwy5tP9nOBL1J2Z4Hyd4hWVF0w==]`
- `[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBaSnQ1cHdPZUVNYzdsWjA2NllvMnZvL2l1aHFMNmx4WVNyN3YyYUNsbGdBCmhGdGl1TzZ4c3ZSak0ydlNIOStsem40NFhBNWY5QjZPV2NSd29JbmJ6QlUKLT4gWDI1NTE5IDhyVnRINkVlWUZ6RUxodzYwWWNzOUNVTkFRQThvdlo0TnBjc1R2azBYZ2sKUnNBNDZqNDZVdkxxUW52RFFWUWNGdlNqRFZIUlh5Uk81bytGWGxKRm85bwotPiB9QXQnNl1ANy1ncmVhc2UgNX0KVUh2VlcxK3UrZk1FV0tTMkowNmt5MGFDbloyQWxQQnZIUGM2ZTV4NmNzZjNXYTF5VUhsd2xBWDlJYkNOVFEKLS0tIHFBSGozeC9kbDdxV3NTZXVBTUVGbVhxNnpiNTA2VTAwQnY0TDg3QWdVS0EKkSzodTlTzxBDFgoy19M4Ny66gHKUWqgQHlOWk4SzxXYfUCafCL1sxUVcc0G84ofq8HGTnbekdpcl24sizvSljvAMf9OhtELGZ454XhFI0J1f8HnlCZQ=]`
- `[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSB6Y2tKNkZKQXF2SXFlQzcyU2VIZGZNdU5sSGRVTlpjR3ZnTkMxcHl6MEFZCm9lTmx6dmN2RVhHQUxId2h5TVVVVXdpN25na09zekNqN3VuU1MzME9qbUkKLT4gWDI1NTE5IFRpc3VHekd6c2hVYU8vZ3lXaUJwK0NpN1VzMlZsTDV6akxldzYxQXlyWDAKaGZBMFphSnRYL3A1Sm1zMmFuQWV0dnJUOVZCZGhXRWFjSnZlVkRBMlo3QQotPiBiU1RHcWwqLWdyZWFzZSBHWzkKUDIrVkFuRGU1WFUKLS0tIHhIbUNLUllsdGovMDlYNE1BWTdFL0hRby9kZWhJeWtoZkN0Tk0vQ3hmeEkKV8IsbHuBcFB3Wj2ioa5oSu9GtkXJwYmK/UhgeXXBM8Gei8G5w657BYjgtcp6MLo++BjZ6r8hE27Znav8zn44QXTpEZWHMlUKZcDI6ewn/JbmtCS7Mr4xjIHP5WP08/vM]`

### 3. dracon-platform — iDrive S3 Credentials

| Item | Value |
|------|-------|
| File | `.env` |
| Commit | `875f4ab8eb` |
| Severity | HIGH |

**Secrets found:**
- `[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSAydmJwSC9zVHRHUmVHNWNNMUhEbnVUenI5N3JZY0dJVGMxc01FZHdycUJFCkk0UFdlR0Q1b3d5bjVsY09zcGo5VElrM005ZXBQSmM2TTN1aXVqKy9CMmsKLT4gWDI1NTE5IEE3TC93ZnlpMm1CWXJsRm43S1hJTk9kOE10Ykk4bDZMT0FRdXJuYzNBRTgKZ2kxRlZjeWNwQWxZTTl1cnFlQWZZV0hPZ1FIYytkalFHcHpDbzJkZlVOVQotPiBzIi1ncmVhc2UgWSxJXwpQY3RhQlNRTDBBUnBUdTc3OENxZmlrTEJaODVRNWcKLS0tIEdueWtEMXRDaTlueDlpU0h6eFduUFd3UWNIWU9rQ3R4eHVaSmhqUmhwelkKKW3MK4mRGjtwnJO4dZc0DzJYEMKeXuYhzCbpYP3a8Vs0bwQRfo1LIyWZNV5VG4QI6NcqkmpwhCd16j13gUVUlDCP2+3V8LMmTCYZSgxfCeI=]`
- `[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBEUTZmcm91QW1hRno4bUpJd3pYUVBWekt0a0hzQ3V1Ky9NUzFQWFAxQkRjCkdrdXB3Q2JsMVl6UHllS3NwSHNLSlFVVDZEZVVjdEZ2MlZHTFZsUm5XUjgKLT4gWDI1NTE5IGtQdVpEOVV6TmVyNFQ4dk1wOHF4VGhoMEpVSWwvY29iQnNBWFFkQ3VMQjQKYS9mTnA5dEtoczczRTVCUXBIczgwS3BKYnZxZ3FOUURmdU5kL3dTUDB6TQotPiBdV0clNk1wLWdyZWFzZSBxUyBuUSA0J0I1IE87aS1CCmg3UDVla3A1YXRrWUpSaE9JaTU3SnQ0bGhYNVl3UDN0d1NheWJ2ZVVrMG5nCi0tLSBsMTk4c2xmSUc1dFRKMVpiQjV0bGVzbUJqci9ZVi9pV00vRndRVmVSL3VJCgiExuIFz7BCGSBvbGorl0qdOyWpn5gctnQPDjiFlNtj5c+mRo2kOaxbuIykIdOyM0v5fs6uUkuHYPUGDWcsjOxOf/nwI4z9emEyYPiy6coODL2zONGOn1YbLb3JzyqN5J3cX4MuGTRE]`

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

## Remediation Steps

### 1. Rotate ALL exposed credentials immediately

- **Paddle:** Regenerate sandbox API key and webhook secret via Paddle dashboard
- **Google:** Regenerate OAuth client secret via Google Cloud Console
- **GitHub:** Regenerate OAuth client secret via GitHub Developer Settings
- **Discord:** Regenerate OAuth client secret via Discord Developer Portal
- **Microsoft:** Regenerate OAuth client secret via Azure Portal
- **iDrive S3:** Regenerate access key and secret key via iDrive e2 console
- **Turso:** Regenerate connection token if it contains auth credentials

### 2. Clean git history before making repos public

```bash
# dracon-demons
cd ~/Dev/dracon-demons
bfg --delete-files credentials.env
git reflog expire --expire=now --all && git gc --prune=doNow

# dracon-platform
cd ~/Dev/dracon-platform
bfg --delete-files production.env
bfg --delete-files .env
git reflog expire --expire=now --all && git gc --prune=doNow
```

### 3. Prevent future leaks

Add `detect-secrets` or `gitleaks` pre-commit hooks to all repos:
```bash
# Install detect-secrets
pip install detect-secrets

# Initialize baseline
detect-secrets scan > .secrets.baseline

# Add to .pre-commit-config.yaml
repos:
  - repo: https://github.com/Yelp/detect-secrets
    rev: v1.4.0
    hooks:
      - id: detect-secrets
        args: ['--baseline', '.secrets.baseline']
```
