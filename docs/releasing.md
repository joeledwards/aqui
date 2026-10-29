# 8.0.0 publication handoff

Version 8.0.0 is prepared locally and has not been published by the agent. It
continues the Node version lineage, with the package version also supplying the
CLI version and User-Agent. This release includes the HTTP-method core, bounded
request-payload previews, offline status-code lookup and HTTPS IP/geolocation.

## Latest validation (2026-10-29)

- 35 isolated unit tests passed on Rust 1.87, including all pretty IP formats in
  auto/always/never color modes with terminal and piped destinations.
- Stable Clippy with warnings denied, formatting and diff whitespace passed.
- Rust 1.87 binary build and documentation generation with warnings denied passed.
- The previously authorized loopback suite passed all 80 HTTP/TLS/process cases
  again against the rebuilt binary, without external requests.
- Offline locked source-package build/verification and archive review passed.
  Source, license, documentation and the opt-in transport tool are included;
  developer captures, credentials and target/build artifacts are excluded.

The pretty JSON regression test failed before its fix. The loopback suite covers
all seven methods, HEAD, literal and 64 KiB binary stdin bodies, response status
and exit policy, redirects/replay/auth stripping, credential redaction, duplicate
headers, timeouts, truncation, body limits, broken pipes, CLI help/version, color
controls and self-signed TLS verification/opt-out. It starts ephemeral loopback
servers and cleans up temporary OpenSSL fixtures.

## Deliberate scope and remaining validation

- Status filters use literal case-insensitive text rather than Node regular
  expressions. The catalog is a bundled historical snapshot with source attribution.
- Pretty IP modes follow destination-aware `--color auto` by default. Forced
  color is explicit; compact IP JSON and plain CLI output always remain plain.
  Environment/theme customization remains future work.
- IP lookup uses HTTPS ipwho.is, a five-second default timeout, a 64 KiB response
  limit and schema validation. No live provider requests were made by the agent.
- Trusted public TLS, proxies, real terminal appearance, blocked/failed stdin,
  early replies, large-stream RSS and other process output failures remain manual
  validation. Prepared headers are initial metadata, not a wire capture; stdin
  byte counts measure source reads, not confirmed server delivery.

Optional developer checks after building (these contact the provider):

```sh
aqui ip -c
aqui ip -gJ
aqui ip -gJ | cat                  # auto mode: no generated ANSI
aqui ip -R --color never
```

For repeatable local verification in this workspace, use the compiler/environment
prefix in README (Rust 1.87 and `CRATE_CC_NO_DEFAULTS=1`) with:

```sh
cargo test --offline --locked --lib --bins
cargo build --offline --locked --bin aqui
cargo doc --offline --locked --no-deps
cargo package --offline --locked
python3 tools/check_transport.py --binary target/debug/aqui
```

The transport suite has separate prior authorization; these commands do not
extend that authorization to external endpoints or broader test campaigns.

## Developer publication

Registry ownership and version availability have not been queried. No push, tag,
registry dry run or publication was performed. Release-binary distribution and
cross-platform CI remain outside this source release.

After the local release commit, publish from `repos/aqui` using the developer's
private credential context, not the workspace-local tool Cargo home:

```sh
cargo publish --dry-run --locked
cargo publish --locked
```

If version 8.0.0 already exists in the registry, choose a new version before
publishing; local package verification does not establish registry availability.
