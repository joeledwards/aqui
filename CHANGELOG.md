# Changelog

## 8.0.0 — prepared, unreleased

Rust CLI implementation continuing the Node aquí version lineage. Version 8
keeps the User-Agent moving forward from Node's `aqui/7.x`; `aqui/0.1.0` would
misleadingly identify the new implementation as an older client. This is a
major implementation transition, not a claim of complete Node feature parity.

- Support GET, HEAD, OPTIONS, DELETE, PATCH, POST, and PUT with timeout,
  redirect limits, repeatable headers and verified HTTPS via rustls.
- Send positional payloads unchanged, including empty strings and JSON scalars;
  stream stdin bodies without buffering. Infer default media types without
  rewriting JSON; manage content framing in the HTTP client.
- Show JSON bodies, duplicate response headers, prepared initial request headers,
  method, URL, status and elapsed request time through body receipt.
- Add Node-inspired truecolor styling with `--color auto|always|never`, default
  auto, preserving URL escapes/query order and plain path separators.
- Stream exact raw response bytes with `-D`; bound normal-mode bodies to 8 MiB
  by default with `--max-body-bytes` override.
- Declare Rust 1.87 minimum, MIT license, explicit source package contents, and
  isolated behavior tests. Refresh inherited serde/JSON lockfile versions.
- Add Basic-auth options with conflict/credential validation, per-client `-C`
  certificate verification opt-out, and redacted outbound credential metadata.
- Use the same HTTP exit policy for normal/raw output: completed responses succeed
  by default; `--fail` fails HTTP 4xx/5xx after output. Successful 201/204 no longer
  fail just because raw mode is enabled.
- Prepare a bounded, opt-in Python loopback HTTP/TLS/process validation suite.

### Scope and differences

- Status filtering currently uses literal text instead of Node regular expressions.
- IP lookup uses HTTPS ipwho.is, and all pretty modes follow `--color`.
- Only write methods accept a payload; payload and stdin mode conflict. Custom
  Content-Length and Transfer-Encoding are rejected in favor of managed framing.
- Default completion status and opt-in `--fail` are independent of output mode.
  Transport/I/O failures always fail in either mode.
- Prepared headers are not a redirect/proxy wire trace. Streamed bodies cannot
  be replayed when redirects require resending them; blocking stdin reads may
  outlive the network timeout. No automatic content decompression is enabled.
- JSON display uses serde_json's nesting limit and last-value semantics for
  duplicate keys. Uploaded literal bytes are preserved independently of display.

See docs/parity.md and docs/releasing.md for remaining validation and release gates.

### Node parity additions

- Offline `status-codes [filter]` with case-insensitive text, unofficial filters,
  duplicate usages, source attribution and destination-aware color.
- Bounded positional payload JSON/hex previews and stdin source-read counts;
  raw output stays undecorated.
- HTTPS `ip`/geolocation lookup via ipwho.is, normalized/provider JSON and CLI
  modes, bounded responses and validation; compact machine formats remain plain.
