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
| Request payload presentation | Implemented | 4 KiB JSON/hex preview; stdin source-read count, not delivery confirmation |
| Raw response streaming (`-D`) | Implemented, same completion/`--fail` exit policy as normal | Process/HTTP-status matrix passed |
| JSON/color presentation | Implemented | Theme/environment customization, response charset and binary display policy |
| Public IP/geo lookup | Implemented | HTTPS ipwho.is, normalized/provider JSON and CLI modes; live provider validation remains developer-run |
| Status-code lookup/filter | Implemented | Offline historical catalog, literal and unofficial filters; current-name adjustments documented in README |

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

## Remaining work

- Status filtering currently uses case-insensitive literal text rather than Node
  regular expressions. The historical bundled catalog is not a live registry.
- Color environment/theme customization is not implemented. All pretty IP modes
  follow `--color auto|always|never`; compact IP modes remain plain.
- Live IP-provider behavior, public TLS, proxies, real terminal appearance,
  blocked/failed stdin and large-stream resource measurements remain manual
  validation. Outbound metadata describes the prepared initial request, not a
  redirect/proxy wire capture. Additional timing breakdowns are future work.

The loopback suite validates method/body/redirect/status behavior without external
requests. See releasing.md for the latest run and publication handoff. Developer
captures `g.c` and `g2.c` remain untouched.

## Lookup/payload differences from Node

- Status filters use literal case-insensitive text instead of Node regular expressions; `-U` continues to
  restrict results with a filter. Unofficial entries use the Node-style `❄️` marker except in `-U` mode,
  where markers and their legend are omitted. The source URL uses
  the shared URL component palette when color is enabled.
- IP uses HTTPS ipwho.is instead of Node's plain-HTTP ip-api.com. Output formats
  conflict explicitly rather than silently taking precedence. Missing geo fields
  are null; ASN is AS<number>. Compact JSON remains plain; all pretty modes follow `--color`.
- Literal upload previews include empty bodies and cap source bytes at 4096.
  Stdin reports bytes read, not bytes acknowledged. Raw response mode stays exact.
