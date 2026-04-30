# Project State

## Current Focus
Harden secret serialization and simplify team key decryption by replacing zeroize skip with serde skip and streamlining x25519 identity parsing with proper UTF-8 handling.

## Completed
- [x] RegistryCredential password excluded from serialization via serde(skip) to avoid accidental persistence of secrets.
- [x] Decryption path now decodes team identity bytes as UTF-8 before x25519 parsing, improving portability and error clarity while adding Cursor wrapping for age decryptor compatibility.
