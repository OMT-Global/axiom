# Stage1 stdlib status

This page maps the current stage1 stdlib to the open phase-c roadmap issues.
It is intentionally a status contract, not a promise that every named issue is
complete.

## Landed bootstrap floor

| Issue | Current stage1 support | Still out of scope |
| --- | --- | --- |
| #232 generic collections | `std/collections.ax` has generic borrowed-slice helpers, and `std/string_builder.ax` now provides an owned string accumulator. | Growable `Vec<T>`, maps, sets, traits, and mutable-borrow-backed collection mutation. |
| #237 structured JSON | `std/json.ax` supports scalar parse/stringify and manual `field_*` / `object*` builders for deterministic JSON object encoding. | Derived struct encode/decode, streaming parse, JSON Schema export, and macros. |
| #238 regex | `std/regex.ax` supports `is_match`, `find`, and `replace_all` over a deterministic NFA-state engine with anchors, `.`, `?`, `*`, `+`, escaped literals, and character classes/ranges. | Captures, alternation/grouping, Unicode character properties, and precompiled regex values. |
| #239 structured logging | `std/log.ax` supports deterministic JSON-line event formatting, levels, key-value attributes, and ambient stderr emission. | Host log sinks, replay buffers, filtering, and runtime logger configuration. |

## Explicitly open

| Issue | Current state | Reason it remains open |
| --- | --- | --- |
| #233 fs write-side | Only `std/fs.ax read_file` is supported, behind the existing read capability. | Write APIs need a separate capability and path policy. |
| #234 net sockets | Only DNS resolution and HTTP client GET exist. | Raw sockets need host:port capability policy and async integration. |
| #236 crypto | Only `std/crypto_hash.ax sha256` exists. | HMAC, AEAD, Ed25519, RNG, and constant-time helpers need real audited implementations. |
| #240 richer testing | `axiomc test` discovers `*_test.ax`, golden stdout, and assertion helpers. | Table tests, property tests, snapshot helpers, and benchmark framework integration need a separate test-harness design. |
| #97 HTTP server | `std/http.ax get` is client-only. | Server lifecycle, routing, response APIs, capability policy, and concurrent handling remain AG4.3 work. |

## Verification handles

- `stage1/examples/stdlib_string_builder`
- `stage1/examples/stdlib_json`
- `stage1/examples/stdlib_regex`
- `stage1/examples/stdlib_log`
- `cargo test --manifest-path stage1/Cargo.toml -p axiomc`
- `make stage1-smoke`
