# Durable device-pair store handoff — 2026-09-08

## Scope

Implemented the isolated `PairStore` seam requested for tablet device-pair persistence. The caller still owns the already-existing parent directory and passes the complete store path to `PairStore::open`; this change does not create or remove directories and does not modify `tablet.rs` or its HTTP tests.

## Implementation

- Added `src/pair_store.rs` and registered it as `pub(crate) mod pair_store`.
- Added `PairStore::open(path)`, `load()`, and `save(&[StoredPair])`, with the requested `StoredPair { token, exchange_csrf, revoked }` serialization contract.
- `open` retains a `.lock` file handle for the store lifetime and uses Windows `share_mode(0)` plus `create(true)/truncate(false)` so a second process cannot open the same store concurrently. Lock-open errors keep their OS error kind but expose a sanitized message.
- Persisted payloads include schema version 1 and are serialized only in memory. On Windows, `CryptProtectData` / `CryptUnprotectData` use `CRYPTPROTECT_UI_FORBIDDEN` without `CRYPTPROTECT_LOCAL_MACHINE`, binding the ciphertext to the current Windows user. DPAPI output is released with `LocalFree`.
- Saves write only ciphertext to a uniquely named same-directory temporary file, flush and sync it, then call `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH`. There is no plaintext disk fallback.
- Load rejects missing/invalid schema, duplicate or malformed secrets, oversized input, corrupt ciphertext, and invalid UTF-8/JSON with sanitized fail-closed errors. Token and CSRF values must be exactly 64 and 32 hexadecimal characters, matching the tablet generator lengths.
- Non-Windows builds return `io::ErrorKind::Unsupported` explicitly instead of persisting secrets.
- Added Windows unit tests for ciphertext secrecy/schema round-trip, exclusive locking, corrupt ciphertext, invalid hex validation, and independent-process reload under the same Windows user. The child process receives only a store path through a test-only environment variable and never prints credentials.

## API verification sources

- [Microsoft Learn: CryptProtectData](https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata) confirms same-user DPAPI behavior, the distinction from `CRYPTPROTECT_LOCAL_MACHINE`, non-interactive `CRYPTPROTECT_UI_FORBIDDEN`, and freeing `pDataOut.pbData` with `LocalFree`.
- [Microsoft Learn: CryptUnprotectData](https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptunprotectdata) confirms authenticated decryption/integrity checking and the `LocalFree` ownership requirement.
- [Microsoft Learn: MoveFileEx](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexa) confirms `MOVEFILE_REPLACE_EXISTING` replacement and `MOVEFILE_WRITE_THROUGH` persistence behavior.
- The offline `windows-sys 0.60.2` definitions were checked for the exact `CryptProtectData`, `CryptUnprotectData`, `LocalFree`, and `MoveFileExW` signatures/constants before implementation.

## Verification

```text
cargo fmt --all
cargo test --offline --locked --lib pair_store -- --nocapture
6 passed; 0 failed
cargo check --offline --locked --all-targets
passed
```

The repository-wide clippy run currently also sees an unrelated nested `if` warning in the concurrently modified `src/tablet.rs`; the isolated pair-store code itself has no new clippy warnings after the fixes above. The root agent owns that tablet integration cleanup and the final all-targets clippy/full test run.

## Remaining integration work

The root agent must finish/verify the `tablet.rs` integration: loading pairs during server startup, saving on pair creation/revocation/reset, preserving durable pairs across server restart, and fail-closed behavior for corrupt stores and save failures. The pair store has been tested independently and across a child process, but production release remains gated on those integrated HTTP tests and the supervisor’s final audit.
