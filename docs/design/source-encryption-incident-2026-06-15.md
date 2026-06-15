# Source Encryption Incident (2026-06-15)

## Context

The operator ran `gibuardien` (a secret-detection tool) and
found that source code was being encrypted by the `dracon-warden`.
The example was a TypeScript test file with a model ID
(`mistralai/[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3Yx...]`)
where the model name `mistralai/[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBydDE2MkhhK01YcDRiNWJsL2VrNjhsdG9FdmJBamYxbzNQcS9tVEkzU3h3CjBKODg1TmJaOG81N3BhbE8rVXV3QmZTQk51NTVTOEx5ZkRTb0VoN3BCbDgKLT4gWDI1NTE5IGhsWWgvTTA1S2F0QTNhSWo5RWdDNjBEYXF3Qk52ZWoxMDdFd21yeTF1UWMKNlpucElOVnErYi8vVzcxRXFmN3IzZmN5L2ViamJVczF3SFVNZVQyenlzdwotPiBYMjU1MTkgYnFRUUdmZGc3UzNseCtLVlBjUHNsd0E2ek8yeDZBSzhvQUtQQVVFTjhBOAovVHhnbHJXQlpEVjZKWjc1bWdtUExWc093bWpjV05YeEdlODNUN2dEMEZZCi0+IFgyNTUxOSAvNkpGTHpucFZKbnEyNW5MYlVYV25tMkorbERSeHloTjEveGxVYjBqK2dFClY1c1FsaS9jaFczYjhYSmZPajhFa2RSNjlvUzNEZ3lkbCthZENYQ0JUbWcKLT4gWDI1NTE5IGxMQVZpM296WThIdDJYbzZMVGI1S2NEMS9yTExWb0VvZVBNdG5jNWlDZ2cKVUtrbjJrM0RlRk55OEpWNzRqKzZpVHFXeUVNeFErTUprTTMyYk05eWRoUQotPiB+LWdyZWFzZSA+Lm5vPCBjR31IUn0gbG5kclREOlEKMEpvakJrTTc2Rmpza3Q0Ci0tLSBObFJnZG5vOEkrQ0xTUlUrRG1NRmZ3TEJDamtoR3BTUkJnT3NqU1FERitVCq7TM2wEhUlB25aLnxJJ3Erc/nPqtb63R0Sdh+inJS7sBRAzgwdEq8QpKy4vmvaSK1XqICw2eridPY+d9eGiEA==]`
had been replaced with an encrypted blob. The user reported:

> "we ahve a prblem foudn by gibuardien that we seem to be
> encrypting code so we encryptes alias test while this could
> technically can catch ranomd secrets this is still perhaps a no"

Translation: the SecretScanner is matching PUBLIC model names
(or test function names) as "secrets" and encrypting them in
source code. The user considers this a no-go because:
1. It corrupts source code (the build breaks).
2. It triggers false-positive secret-detection alerts.

## Root cause

The active `dracon-warden` config at
`~/.dracon/utilities/warden/dracon-warden.toml` had a
`protected_patterns` list that INCLUDED source code files:

```toml
protected_patterns = [
    "*.env", ".env", ".env.*",
    "config.json", "config/services.json", ...,
    "*.pem", "*.key", "*.age", "test_secret.txt", "secrets/**",
    # Source code — scan for hardcoded secrets
    "*.rs", "*.py", "*.ts", "*.js", "*.jsx", "*.tsx", "*.go",
    "*.sh", "*.bash", "*.yml", "*.yaml", "*.toml", "*.md",
    "*.sql", "*.json",     # <-- this list
]
```

The `SecretScanner` (50+ regex patterns including the
`Mistral API Key` regex `mistral-[A-Za-z0-9_-]{20,}`) ran on
ALL text content of files matching these patterns. The model ID
`mistralai/[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBEV0FYTDErdXJMT1pXcHYxZC9mRy9OZW9QVGNhUWxYZUxPejFJbHhqRkJZCkU4V2VSMVBPR1FEalFub3lmNS9WNStWUDFXTVZtbUpCVHoxUVU5SHVvcEkKLT4gWDI1NTE5IFZHQzJXMHhLV25DMHRtbU5kbGlKLzBhenFMSFA1WFBLOUR4ZDRMTE5Ra28KYk1FWXBEQlltQmJHbGVDekV4R2hJL1lJOXo1Z3FUSE9KUWpMNVpQQUtHdwotPiBYMjU1MTkgL1RycjE4bEhuZlEyOVdCWWtmcDFFQUxXZmhTMWVTSm1QTW1uSGV0aFgwTQp1aS9MTjZwQ2hiRE1DMjlwajBHZHdwTnAwM2FKWTFWekhyZ2FidGduRUNBCi0+IFgyNTUxOSBNZng1YmlWUHQ0eGpNNGdNMk1uZWJmZ3R2NXdyb1hPWDR5Mm9venVNTEZrClBHcU5XeGVFbGZLVzNEK1dVQjd3OW1vdlZ1ckZsRWh4UTRoQW5KTE5QMVkKLT4gWDI1NTE5IHREbFIyNlh1OWkwVCtQWm5IRkZaQkNOczJVTnVOUjRMWTQ1cnBGY0Q3RWsKNkF1anlZeGtvenhoM29JMEtqVDV0RFNJMVhsZ2hCL1JBc0k5K3EveFFPWQotPiApTS1ncmVhc2UgYW8geSBuQDc6WCBKX2xvem4KUitBR25IdnRMa2NuM2k0REhqVUFqZzJQUzBBNTlsRVRwK3pXOFhzc2dWSE5zVmZ2dzFXa0pTUk5scjJlbTU4OQovZlNJblRkRkQzQXloZGJzSmJPc05kazZqdXc5Yll5RnVtWmZCYW9xdGZoRWlwcwotLS0gdEZnM1djVFdtRGdlendPdG1YWko5eU1LVDQzZU5qUTFiSytwZHNsK0g0bwqOcgDCuRryC5bVGTvAdgab2y71Q52SyxdHnHicxoGVxMMjS2nhrWyNl0Q78xmOSTU02JpW4nUv8vv7oYb7SAs=]` matched the Mistral
regex (the part after `mistral-` is 22 chars ≥ 20), so it was
encrypted and wrapped in `[DRACON_SECRET:<base64>]`.

## The fix

Two-part fix:

### Part 1: Code change in `dracon-security`

Added a new "protected-patterns gate" in
`dracon-warden/src/security/src/modules/filter.rs::smart_clean_with_path`.
The function now checks if the file path matches any
`protected_pattern` glob. If it doesn't match, the file is
passed through unchanged and the SecretScanner is NEVER
invoked. This makes the `protected_patterns` config field
actually gate the scanner, so source code files are
default-skipped.

The new helper `path_is_protected(path_str, &protected_patterns)`
implements glob-based matching with 5 rules:

1. Exact filename match (e.g. `master.age` matches `master.age`).
2. Suffix match: `*.env` matches `foo.env` (per standard glob
   semantics; `.env.local` needs `.env.*` or similar).
3. Path-prefix match: `secrets/**` matches any path under
   `secrets/`.
4. Multi-component `**` glob: `**/audit/**` matches any path
   with `/audit/` as a component.
5. Substring match (last-resort fallback) for ad-hoc patterns
   like `config/services.json`.

If `protected_patterns` is empty, the function returns
`true` (legacy: empty list means "scan everything"). This
preserves backward compatibility for any operator who
hasn't configured `protected_patterns`.

The new check is added to `smart_clean_with_path` BEFORE
the existing `is_sensitive_location` and `is_full_encrypt`
logic, so a non-protected path is short-circuited to
`return Ok(content.to_vec())` and the scanner never runs.

### Part 2: Config cleanup in `dracon-warden.toml`

Removed all source code patterns from
`~/.dracon/utilities/warden/dracon-warden.toml`'s
`protected_patterns`. The new list is scoped to data files
and config files that legitimately contain secrets:

```toml
protected_patterns = [
    "*.env", ".env", ".env.*",
    "config.json", "config/services.json", ...,
    ".cargo/config.toml",
    "*.pem", "*.key", "*.age", "test_secret.txt", "secrets/**",
    # NO MORE: *.rs, *.py, *.ts, *.js, *.jsx, *.tsx, *.go,
    #          *.sh, *.bash, *.yml, *.yaml, *.toml, *.md,
    #          *.sql, *.json
]
```

A comment was added to the config explaining why source
code patterns were removed, with a link to this design
doc.

## Restored file

The corrupted test file was:
`/home/dracon/Dev/browser-extensions-shared/extensions/vidpro-extension/test/components.test.ts`

The model ID was restored from
`mistralai/[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3Yx...]`
back to
`mistralai/[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBJZk11WUwwMTdSZzU2WmhnNVNhcDIvWEtjU05ZM2xyb3FuQlNjcFpNWnlJClVaZnZsWGNPL3A0Vy9IV3lSazFFS1BjSlk3K29MaVIyZ0dWcmVhdGpMVjgKLT4gWDI1NTE5IFdEeFpYOUp1TXprWkhoQVI4WTlMSmxhNDBWQnEra3dwck9PQTF1SGhoR00KSWRCWWg5cks3LzJEUkl0c00zRVpCeXJsZ3l3Mk44Z2M0clRaR0pXYkpmdwotPiBYMjU1MTkgT0RwQ2xKQVlUcVlGd1hJaVhjNGtySjE2cVRiQml4Y3laOWVUQWQwUitIdwo3ZU8rMWRzZGxLZjZCZm1vRVhmSHNCY2ZkOGRnQ3BucWZ0Nis3V1dDNHlvCi0+IFgyNTUxOSBQNHI1VEhsYm5qTzYySGQrZUlCV3lZWWZSWHptZUYwbkZRM1dYbzRURXgwCkU3YlRuYk0zUWJtRDdPU1JOTms5TnF2RG5iZEIvanFYV0xEVWI4UHF0UE0KLT4gWDI1NTE5IGIveWtyT2FuLy9Sc2VOVjNtdFZIekN4N05IVGZ4SVNzQXJDa1ZaOGo2d28KL0xtajlmR01DWXNGM0RmUlNPT3VsY3NVN0FHOFU4NVVnQ3JydGJ4RlozRQotPiBeTSEyP20tZ3JlYXNlIFptVE4KcWJ0WFF4NWdpSnBwZm5jRFNUVmx5d01XV2JhUWZicXhtRkRyUXVaSlZZUmR0WDJ6VGliL3BCdWgKLS0tIDI2NW1qa1NhdmNFdVlmdXZXTzVWd25OaHI2U0M5MnBIR0tWcXRWbnFRR0UKk8AdKEQUfyHaeVAzJz1RGr6Jjhm23m5aq+2ukqu29MUP9OUzGbvBO91hIBOWrnpXFIUtJCg7fE9oYmkgSdig]`.

The original model ID was verified by cross-referencing the
sibling file
`extensions/vidpro-extension/utils/byokAdapter.ts` (line 32)
which has the same model ID for the same provider (Mistral).

## Tests added

6 new unit tests in
`dracon-warden/src/security/src/lib.rs::tests`:

1. `test_path_is_protected_legacy_empty_passes_everything`:
   empty `protected_patterns` = scan everything (legacy).
2. `test_path_is_protected_env_pattern`: `*.env` matches
   `foo.env`, `prod.env` (standard glob).
3. `test_path_is_protected_source_code_excluded`: source
   code paths are NOT in `protected_patterns` and return
   `false`.
4. `test_path_is_protected_directory_prefix`:
   `secrets/**` matches any path under `secrets/`.
5. `test_path_is_protected_substring_fallback`:
   `config/services.json` matches by substring.
6. `test_smart_clean_with_path_skips_unprotected_source_code`:
   end-to-end test that the scanner is never invoked on
   unprotected paths (an OpenAI `sk-XXX` key in a `.ts`
   file is NOT encrypted, but the same key in a `.pem`
   file IS encrypted).

## Audit of other repos

`rg -l "DRACON_SECRET:YWdl" --type-add 'src:*.{rs,ts,tsx,js,jsx,py,go,sh}' --type src <repo>`
was run on all 14 watched repos. The only source-code hits
were intentional test fixtures (in
`dracon-warden/src/security/tests/plaintext_sibling_test.rs`)
which use encrypted blobs as test data to verify the
encryption scanner preserves already-encrypted content.

The markdown audit archive at
`dracon-platform/apis/docs/audits/archive/2026-06-15/2026-06-09-apis.md`
also references encrypted blobs but is intentional
documentation, not source code.

No other source-code files were corrupted.

## What about the encrypted `.env` files?

`dracon-platform/apis/*/.env*` files are LEGITIMATELY
encrypted by dracon-warden via the git clean/smudge filter
(`.gitattributes` has `*.env filter=dracon diff=dracon
merge=dracon`). The working tree shows plaintext (via
smudge), and the committed bytes are encrypted. This is
correct behavior and continues to work after the fix.

The audit archive at
`dracon-platform/apis/docs/audits/archive/2026-06-15/2026-06-09-apis.md`
documents this:

> `.gitattributes` (managed by dracon-warden) sets
> `*.env filter=dracon diff=dracon merge=dracon`, which
> wires a clean/smudge filter that encrypts on `git add`
> and decrypts on `git checkout`.

The fix does NOT change this behavior — `.env` is still
in `protected_patterns` and the encryption is preserved.

## How to verify the fix

1. Run the test suite: `cargo test --workspace --locked
   -- --test-threads=1` (848 passed, 0 failed).
2. Run `dracon-sync repos` to see the live state.
3. Check the active config: `cat ~/.dracon/utilities/warden/
   dracon-warden.toml | grep -A 20 protected_patterns`.
4. Check that source code is unchanged after a `git add`:
   `cd <repo> && git add src/ && git diff --cached --name-only`
   should NOT include any encrypted blobs.

## How to add a new protected file pattern

If you want to encrypt a new data file pattern (e.g. a new
secret format), add it to `protected_patterns` in
`dracon-warden.toml` and restart the warden daemon. The
matching rules in `path_is_protected` will pick it up:

```toml
protected_patterns = [
    # ... existing patterns ...
    "certs/*.crt",         # CRT files
    "deploy/*.kubeconfig", # Kubernetes configs
]
```

Source code files (e.g. `*.rs`, `*.ts`) should NEVER be
added — they are not "data files" and should be left alone.
