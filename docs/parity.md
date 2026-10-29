# Node aquí feature intent and Rust coverage

The target is a useful, accurate HTTP CLI with the same broad intent. Node's
implementation details and bugs are not compatibility requirements.

| Capability | Rust state | Remaining work |
| --- | --- | --- |
| GET, HEAD, OPTIONS, DELETE, PATCH, POST, PUT | Implemented, construction/CLI/output unit tests | All methods/HEAD validated on loopback; broader endpoints remain optional |
| Optional positional write payload | Implemented, exact UTF-8 bytes including empty input | Live upload/redirect checks |
| Stream payload from stdin (`-I`) | Implemented without buffering or sniffing | Live streaming/early-response/error/cancellation checks |
| Custom headers, timeout, redirect limit | Implemented | Live timeout/redirect/proxy verification |
| Basic authentication (`-u`, `-p`) | Implemented with conflict/credential validation | Loopback credential/redirect checks passed |
| Invalid-certificate opt-in (`-C`) | Implemented per client, verified by default | Self-signed TLS rejection/opt-out checks passed |
| Body, response headers, request headers, URL, status, duration | Implemented | Request headers are prepared initial metadata, not a final wire trace |
| Request payload presentation | Missing | Bounded display, JSON/hex rendering, streamed upload byte count |
| Raw response streaming (`-D`) | Implemented, same completion/`--fail` exit policy as normal | Process/HTTP-status matrix passed |
| JSON/color presentation | Implemented | Theme/environment customization, response charset and binary display policy |
| Public IP/geo lookup | Missing | HTTPS provider choice, schema validation, clear output modes/timeouts |
| Status-code lookup/filter | Missing | Curated official/unofficial dataset with provenance and filter contract |

## Accuracy choices already made

- Send positional payload bytes as supplied: no JSON reserialization, loss of
  duplicate keys, rounding, or falsy-value omission. Explicit empty input is a
  body; absence remains distinct.
- Infer application/json for positional JSON (including scalar values); otherwise
  application/octet-stream. An explicit Content-Type wins. Streams default to
  application/octet-stream; use a header for a different type. JSON detection is
  bounded by the supplied argument and serde_json's nesting limit.
- Only DELETE/PATCH/POST/PUT accept a body. GET/HEAD/OPTIONS reject both a
  positional body and stdin mode. Payload plus stdin is an argument error.
- Let the client own framing: explicit Content-Length and Transfer-Encoding are
  rejected. Positional bodies have exact byte Content-Length; streams are
  unknown-length and HTTP framing is transport-dependent. The initial header
  snapshot does not claim to show transport-generated stream framing.
- Preserve duplicate header values, URL escapes/query order, and the actual
  initial method in display. Label a different final URL separately.
- Time execution through full response-body receipt, excluding client/request
  construction and output formatting. This is elapsed request time, not isolated
  server processing or full process runtime. Streaming uploads include source
  wait time; a blocking stdin read is not guaranteed to be interruptible by the
  network timeout.

## Recommended next steps

1. Local `tools/check_transport.py` passed 80 developer-authorized HTTP/TLS/process
   cases. Keep it available for regression checks.
2. Complete developer-run HTTP/HTTPS validation covering every method, bodies,
   HEAD, redirects, timeouts, duplicates, and read/write failures. Unit success
   does not establish transport correctness. Reqwest's redirect behavior applies:
   methods/bodies can be rewritten for 301/302/303, while streaming bodies cannot
   be replayed for redirects requiring the original body. Do not buffer stdin or
   resend a modifying request silently to achieve legacy parity.
3. Improve diagnostics: bounded request-body display, uploaded byte counts,
   initial versus final request metadata, and optionally separate header/body
   timing. Do not claim header snapshots are a wire capture.
4. The revised exit contract is process-tested: a completed response
   succeeds in either mode; `--fail` fails HTTP 4xx/5xx after output. In particular,
   201 and 204 succeed in raw mode. Transport/I/O failures always fail.
5. Add status-code lookup, then IP/geo lookup. Decide provider and output schemas
   before implementation, and avoid carrying over Node's plain-HTTP geo endpoint.

Developer captures `g.c` and `g2.c` are local inputs, not test fixtures; they
remain untouched. The authorized loopback suite ran successfully; no external
endpoint calls were made by the agent.
