# ProjectState

## Current Focus
Expose public keys for team key handling by implementing conversion to age Recipient## Completed
- [x] Added `TeamKey::to_public` method that converts the internal byte slice to `age::x25519::Recipient` by parsing the identity as UTF-8
- [x] Integrated this method into the `TeamKey` struct, enabling downstream code to obtain age‑compatible public recipients directly
- [x] Preserved zero‑knowledge semantics while providing a clean, idiomatic conversion path ready for cryptographic usage
