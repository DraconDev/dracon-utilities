# Warden global-hook ownership (2026-08-15)

`dracon-warden setup-hooks --global` owns the three files in
`~/.config/git/hooks/`: `pre-commit`, `pre-push`, and `pre-rebase`.
Ownership of those filenames does not mean silently destroying an existing
hook.

The installer now:

1. stages all three executable hook files in same-directory temporary files;
2. detects same-named files that are not Warden hooks;
3. moves those foreign hooks to a unique `<hook>.dracon-foreign` sibling;
4. installs the Warden wrappers and chains the preserved hooks; and
5. rolls back the set if a preserve or replacement step fails.

The pre-push wrapper buffers Git's ref stream so both hooks receive identical
stdin. Repository-local foreign hooks continue to be chained by the Warden
wrappers as before. Existing `.bak` files are not guessed or adopted because
their ownership is ambiguous; operators can inspect them and explicitly
rename them if they are intended hooks.

The generic Nix checker does not recognize the conventional
`homeManagerModules` output. `scripts/check-flake.sh` treats that exact
warning, and only that warning, as expected.
