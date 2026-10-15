# P0 Local IPC Byte Framing

**Scope:** [Issue #10](https://github.com/hami9/LiDB/issues/10). This work stacks on [PR #9](https://github.com/hami9/LiDB/pull/9), before `main` contains the Rust workspace.

## Format

Each frame has a **four-byte unsigned big-endian payload length**, followed by exactly that number of bytes. The payload is opaque to this module. There is no magic prefix, compression or implicit JSON/protobuf encoding. Empty payloads are invalid. The payload limit is **1,048,576 bytes**; this excludes the four-byte header.

This is a provisional `0.x` wire envelope. Its exact meaning and protocol schema version must be reviewed before permanent compatibility promises.

## Decoder behavior

- Accept arbitrarily fragmented headers and frame bodies, and consecutive frames on one connection.
- Reject advertised zero-length payloads and payloads larger than `MAX_FRAME_BYTES` **before buffering their body**.
- Accept at most 64 KiB per `push` call. A transport must split larger reads and handle backpressure.
- Complete frame payloads are returned as owned vectors. For each decoder, at most one incomplete body of size `MAX_FRAME_BYTES` is retained.
- `finish()` detects truncated headers and truncated bodies at stream EOF.
- After an invalid frame, reject all input until reset; callers **must close that untrusted stream** instead of attempting in-place resynchronization.
- No plaintext secrets, model weights, prompts or packet contents should enter this transport by default. Content policy and redaction belong to higher layers.

## Not implemented

This is an offline codec and **not a running daemon or IPC service**. No Unix domain socket, OS peer credential verification, user/group policy, payload schema, protocol negotiation handshake, async I/O, remote listener, encrypted transport or privileges are implemented.

Future socket integration must authenticate local peer credentials before accepting commands, validate complete message schemas, enforce frame timeouts, bound outbound queue sizes, and keep the CLI/TUI non-root.

## Validation

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked --all-targets
```

GitHub CI on Linux x86_64 and Linux aarch64 supplies actual runtime test evidence. No PRoot, kernel/GPU hardware, or live-process tests are claimed.
