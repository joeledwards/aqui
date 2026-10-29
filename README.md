# aquí

Version **8.0.0** is prepared for release under the [MIT license](LICENSE).
The major version advances the User-Agent from Node's `aqui/7.x` to `aqui/8.0.0`,
avoiding a misleading version regression to `aqui/0.1.0`. This Rust release provides
the HTTP-method core, payload previews, status lookup and public IP/geolocation
lookup. It does not claim complete Node feature parity.
See the [release checklist](docs/releasing.md) and [changelog](CHANGELOG.md).

A human-friendly HTTP CLI, ported from [Node aquí](https://github.com/joeledwards/node-aqui).
Supports GET, HEAD, OPTIONS, DELETE, PATCH, POST, and PUT, including positional
and streamed stdin payloads for write methods, Basic authentication, and a
per-client TLS verification opt-out, offline status-code lookup, and HTTPS
public IP/geolocation lookup.
See [feature parity and next steps](docs/parity.md).

Requires Rust 1.87 or newer. Build with `cargo build --locked`.
Install from a local checkout with `cargo install --path . --locked`.

```sh
aqui get https://example.com
aqui get localhost:8080/data -h 'Accept: application/json' -t 10000
aqui get https://example.com/file -D > file
aqui get https://example.com --max-redirects 0
aqui --color always get https://example.com
aqui get https://example.com --color never
aqui head https://example.com
aqui options https://example.com
aqui post https://example.com/records '{"name":"test"}'
aqui patch https://example.com/records/1 '{"active":true}'
aqui put https://example.com/records/1 -I -h 'Content-Type: application/json' < record.json
aqui delete https://example.com/records/1
aqui get https://example.com/private -u user -p password --fail
aqui get https://localhost:8443 -C
```

Use `aqui get --help` for all options. `-h` supplies a header, following Node aquí;
help for each method command is available through `--help`.

## Status-code lookup

```sh
aqui status-codes
aqui status-codes '42'
aqui status-codes 'not found'
aqui status-codes -u
aqui status-codes -U
```

This command runs offline. A case-insensitive literal text filter matches the
code or description and implies inclusion of unofficial entries. Regex metacharacters are literal; no matches prints the heading/source and
succeeds.
`-u/--include-unofficial` includes unofficial entries; `-U/--only-unofficial`
restricts results even when combined with a filter or `-u`. Duplicate codes are
retained for distinct unofficial usages, marked `[unofficial]` in plain and colored
output. Codes are numerically ordered. Filters currently use literal text rather
than Node's regular-expression syntax.

The bundled historical catalog comes from node-http's Wikipedia-based list;
413/422 use current registered names, and 306/418 are labeled unused (418 retains
its historical teapot label for search). It is not a live or exhaustive registry.
Source attribution prints with every listing. See `src/status.rs` for the snapshot.

## Public IP and geolocation

```sh
aqui ip
aqui ip -c
aqui ip -gJ
aqui ip -gr
```

IP lookup sends one HTTPS GET to `https://ipwho.is/`, including for IP-only output.
The provider sees the caller's public IP (or configured proxy's IP) and may apply
availability, rate, or usage restrictions. There is no fallback to plain HTTP.
TLS verification stays enabled, redirects are not followed, and `-t/--timeout`
sets a positive total timeout in milliseconds (default 5000). Responses are limited
to 64 KiB. Non-success HTTP status, invalid JSON, provider failure, missing/invalid
IP, and malformed supplied fields fail with exit 1 before output.

Default output labels the IP; `-g/--geo` adds geolocation. `-c/--cli` prints the IP
on one line, optionally followed by `CC,REGION,ZIP LAT,LON`; `-C/--cli-pretty`
is the human-colored variant. `-j/--json` and `-J/--json-pretty` produce compact
and indented normalized JSON. Without `-g`, normalized JSON contains only `ip`.
With `-g`, fields match Node's names: `ip`, `as`, `isp`, `org`, `countryCode`,
`countryName`, `regionCode`, `regionName`, `city`, `zipCode`, `metroCode`,
`timeZone`, `latitude`, `longitude`. ASN is formatted as `AS<number>`; unavailable
fields are JSON null (human output uses `unknown`). Metro code is unavailable.
`-r/--raw` and `-R/--raw-pretty` serialize the provider's complete JSON object,
including geolocation, compact or indented; they do not preserve response whitespace.
Raw provider schema can change independently of normalized output.

Choose one output flag; conflicting formats are argument errors. Compact JSON
(`-j`, `-r`) and plain CLI (`-c`) never contain generated ANSI, even with
`--color always`. All pretty modes (`-C`, `-J`, `-R`) and default human output
follow `--color`: auto colors terminals, always forces color, and never disables
it. Pretty JSON uses the same token palette as HTTP response JSON. Every mode
ends with a newline.

## Current contract

- Missing URL schemes default to `http://`. Only HTTP and HTTPS are supported.
  URL credentials and whitespace are rejected; fragments are removed.
- TLS certificates are verified using rustls and bundled public roots. Explicit
  `-C` / `--allow-invalid-certs` disables verification for this request's client
  only, including its redirects. It does not change environment/global TLS state.
  Reqwest's default system
  proxy/environment behavior applies; custom roots and proxy options are pending.
- A total request timeout defaults to 5,000 milliseconds; zero is invalid.
  Up to five redirects are followed. A zero redirect budget returns the original
  response; exhausting a positive budget fails. Redirects display a separate final URL.
- Headers split at the first colon, trim surrounding spaces/tabs, and allow
  empty values. Duplicate values are preserved. Invalid headers fail before
  sending. `User-Agent: aqui/<package version>` is supplied unless overridden.
- `-u` / `--username` and optional `-p` / `--password` send preemptive HTTP Basic
  authentication. Password defaults to empty; a password flag requires a username.
  Empty usernames are allowed; colons in usernames and control characters in
  credentials are rejected. Password colons and UTF-8 credentials are allowed.
  Basic-auth flags conflict with any explicit Authorization header. Reqwest's
  cross-origin redirect credential-stripping rules apply.
- DELETE/PATCH/POST/PUT accept an optional positional payload or
  `-I` / `--payload-from-stdin`; combining them is an argument error.
  GET/HEAD/OPTIONS accept neither. Literal payloads are sent as exact UTF-8 bytes,
  including an explicitly empty argument. JSON literals default to
  application/json; other literals and stdin default to application/octet-stream.
  An explicit Content-Type overrides detection. Stdin streams without buffering
  or JSON detection.
- Normal mode displays positional request payloads after the method/URL and before
  outbound headers: byte count plus pretty JSON for complete valid JSON, otherwise
  hex with offsets and printable ASCII. At most 4096 source bytes are shown;
  larger bodies show a hex prefix and explicit truncation notice, without changing
  sent bytes. Empty literals display a zero-byte payload; absent bodies do not.
  Stdin stays unbuffered and displays only bytes read from the source, explicitly
  not confirmed server delivery. Payloads are displayed verbatim without secret
  redaction. Raw response mode bypasses all upload diagnostics.
- Content-Length and Transfer-Encoding are managed by the HTTP client; supplying
  either as a custom header is rejected. Literal bodies have exact byte lengths;
  streamed framing depends on HTTP version and is not included in the prepared
  header snapshot. Redirect method rewriting/replay follows reqwest; streamed
  bodies cannot be replayed when a redirect requires resending the body.
  A blocking stdin read is not guaranteed to be interruptible by the timeout.
- Normal mode buffers at most 8 MiB of response body by default. Override with
  `--max-body-bytes N`. An oversized or failed read produces an error before any
  response output. JSON formatting may allocate additional memory; the byte
  limit is not a total heap limit.
- Normal mode prints the body, sorted response headers (including duplicates),
  the actual method and initial request URL, sorted outbound headers, status, and elapsed
  milliseconds through body receipt. Outbound headers include explicit headers,
  default Accept and User-Agent, and a derived Host (including IPv6/port).
  Authorization, Proxy-Authorization and Cookie values are shown as `[redacted]`
  in outbound metadata, without changing sent values. Other request values and
  response bodies/headers are displayed as received.
  This is the prepared initial request, not a wire trace of proxy headers or
  subsequent redirect requests. A different final URL is shown separately.
  JSON accepted by serde_json (including its default nesting limit) is indented
  with two spaces, preserving key insertion order and
  arbitrary-precision number tokens. Duplicate JSON keys use serde_json's last
  value. Bodies that cannot be parsed as JSON are written as received.
  Response content is not escaped for terminal display; use care with untrusted
  data.
- `--color auto|always|never` controls normal response styling and defaults to
  `auto`: color on stdout terminals, plain when redirected or piped. The option
  works before or after `get`. `always` forces ANSI truecolor; `never` disables
  generated styling. Raw output (`-D`) never adds ANSI codes in any color mode.
  This option controls response output, not Clap's separate help/error styling.
  Color environment variables and palette customization are not implemented.
- The Node-inspired palette uses yellow header names and JSON keys, blue header
  values and booleans, green strings, orange numbers/method/timing, and purple
  nulls. URL scheme, host, port, path and query parts have separate colors.
  Path segments are purple with plain slash separators; ports use a plain colon
  and orange number. Status classes use blue (1xx), green (2xx), purple (3xx), red (4xx), and yellow
  (5xx/other). Styling preserves plain text and header alignment; non-JSON body
  bytes are not highlighted.
- `-D` / `--only-output-data` streams exact response body bytes, adds no newline,
  and uses bounded copy buffers. The normal-mode body limit does not apply.
  Automatic content decompression is disabled; compressed bytes remain compressed.
- Normal and raw modes use the same exit policy: a completed HTTP response
  succeeds by default, including 201/204 and HTTP error statuses. `--fail` makes
  HTTP 4xx/5xx exit 1 **after writing the response**, in either output mode.
  Unfollowed 3xx responses are not errors under `--fail`. This intentionally
  replaces Node's raw-only-200 success policy.
  Both modes fail on transport, read, write, and flush errors. Broken pipes fail
  quietly. Argument errors exit 2; other failures exit 1; success exits 0.

Remaining differences from Node aquí are documented in the parity guide.
See [test coverage](docs/test-coverage.md) for verification and remaining gaps.

## Development

```sh
cargo test --locked --lib --bins
cargo fmt --check
cargo clippy --locked --lib --bins --tests -- -D warnings
```

Unit tests build requests and use in-memory readers/writers; they do not contact
HTTP endpoints or start services. Live request/TLS/redirect checks are separate
manual validation. In this workspace use the tooling described in
`../../docs/rust-tooling.md`.

For this workspace's Zig setup, `ring` needs `CRATE_CC_NO_DEFAULTS=1` to avoid
cc-rs supplying a Rust-style target triple that Zig rejects. From the workspace
root, the verified unit-test command is:

```sh
aqui_toolchain="$PWD/.tools/rust/rustup-home/toolchains/1.87.0-x86_64-unknown-linux-gnu/bin"
scripts/with-rust env CRATE_CC_NO_DEFAULTS=1 \
  RUSTC="$aqui_toolchain/rustc" RUSTDOC="$aqui_toolchain/rustdoc" \
  "$aqui_toolchain/cargo" test --manifest-path repos/aqui/Cargo.toml \
  --offline --locked --lib --bins
```

Manual HTTP verification, **not run by the agent**: use the same wrapper/env
prefix with `cargo run --manifest-path repos/aqui/Cargo.toml --offline --locked
-- get https://example.com`, then repeat with `-D`. This contacts an external
endpoint. Local redirect, timeout, self-signed TLS, and exit-status checks passed
in the authorized loopback suite; trusted external TLS remains a separate check.

The opt-in local transport/process suite passed all 80 cases and is available in
`tools/check_transport.py`; see [release validation](docs/releasing.md). It starts
loopback HTTP/TLS servers; the developer authorized this suite on 2026-10-28.
