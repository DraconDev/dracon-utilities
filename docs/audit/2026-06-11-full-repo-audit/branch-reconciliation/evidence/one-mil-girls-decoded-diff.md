# one-mil-girls DECODED diff (smudge + worktree)

The .gitattributes has filter=dracon on *.ts/*.json/etc., so 'git diff'
shows nothing because the index holds ciphertext and the worktree holds
plaintext. To inspect real user changes, smudge the HEAD blob and diff.

## timestamp
2026-06-11T18:21:05+01:00

## git status
## main...origin/main
 M src/lib/engine/characters.test.ts
 M src/lib/engine/saveLoad.test.ts
 M src/lib/stores/saveLoad.svelte.ts
?? docs/audit/2026-06-11-cleanup/

=== src/lib/engine/characters.test.ts ===
--- /tmp/tmp.aHs3bDVEiM	2026-06-11 18:21:05.644002951 +0100
+++ src/lib/engine/characters.test.ts	2026-06-11 17:10:10.678453517 +0100
@@ -10,7 +10,7 @@
 
 const REPO_ROOT = process.cwd();
 
-// Stub $state before importing the .svelte.ts store (same shim as saveLoad.test.ts).
+// Stub $state before importing the .svelte.ts store (same stub as saveLoad.test.ts).
 function makeStateShim() {
   return new Proxy({}, {
     get(_t, prop) {

=== src/lib/engine/saveLoad.test.ts ===
--- /tmp/tmp.OQqwyBWFIL	2026-06-11 18:21:05.650003061 +0100
+++ src/lib/engine/saveLoad.test.ts	2026-06-11 17:10:07.169392355 +0100
@@ -5,7 +5,7 @@
 // rather than failing.
 //
 // Caveats:
-//   - Svelte 5 runes ($state) are not available in raw Bun. We shim $state
+//   - Svelte 5 runes ($state) are not available in raw Bun. Stub $state
 //     on globalThis as a no-op getter/setter proxy that preserves the
 //     assigned value. This lets gameState + charactersState import without
 //     exploding — we don't rely on reactivity, only on assigned values.
@@ -14,7 +14,7 @@
 import { describe, expect, test, beforeEach } from 'bun:test';
 import type { SaveData } from '$lib/engine/types.js';
 
-// ---- $state shim for Bun ----------------------------------------------------
+// ---- $state stub for Bun ----------------------------------------------------
 function makeStateShim() {
   return new Proxy({}, {
     get(_t, prop) {

=== src/lib/stores/saveLoad.svelte.ts ===
--- /tmp/tmp.IoP4Q6JoE6	2026-06-11 18:21:05.657003189 +0100
+++ src/lib/stores/saveLoad.svelte.ts	2026-06-11 17:10:03.552329312 +0100
@@ -147,7 +147,7 @@
       gameState.playtime = saveData.playtime;
       gameState.isPlaying = true;
       gameState.screen = 'vn';
-      // Restore route (backward compat: may be missing from older saves)
+      // Restore route (older saves may omit route)
       gameState.route = saveData.route ?? null;
       gameState.endingsSeen = saveData.endingsSeen ?? [];
 

