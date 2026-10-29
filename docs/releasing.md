# 8.0.0 release checklist

Status: local source/package checks complete; publication gates remain. Not published. Version 8 continues the Node
lineage and advances its User-Agent past Node's `aqui/7.x`, avoiding a misleading
regression to `aqui/0.1.0`, while delivering the Rust HTTP-method core. Missing Node features are
listed in README, CHANGELOG and parity.md; full parity is not a release claim.

## Source/package gates

- [x] Package version 8.0.0; CLI version and default User-Agent derive from it.
- [x] MIT license selected by developer; LICENSE and Cargo metadata included.
- [x] README, changelog, parity guide, coverage matrix and package allowlist.
- [x] Source package excludes target, developer captures, credentials and Git files.
- [x] Final offline locked Rust 1.87 unit tests, formatting and stable Clippy.
- [x] Documentation generation and local package build/verification.
- [x] Review package contents; parser-level method help/version tests pass.
- [x] Process-level CLI help/version smoke checks (authorized loopback suite).

Current validation: 24 unit tests passed on Rust 1.87 after adding auth, per-client
TLS opt-out and the uniform HTTP-status policy; stable direct clippy-driver with
warnings denied and formatting passed. The status-policy regression failed with
the old policy before the fix. Rust 1.87 binary/docs build and offline locked
source package build/verification passed. Package includes the opt-in transport
suite (18 files, approximately 140 KiB unpacked / 40 KiB compressed).
Archive manifest/license/allowlist inspected. Developer subsequently authorized the
loopback suite: all 80 HTTP/TLS/process cases passed on the Rust 1.87 debug binary.
No external endpoint/registry queries, publish dry run, Git operations or publication.

## Transport/process validation

Developer explicitly authorized the local suite on 2026-10-28. All 80 cases passed
with no external requests. The first run exposed an incorrect redirect-limit
fixture that terminated successfully; it now redirects to itself. No Rust client
fix was needed. Added process help/version and piped auto-color checks; expected
peer resets in negative tests are handled by the fixture server.

- [x] All seven methods, absent/empty/JSON/text literal bodies, exact 64 KiB binary
  stdin bodies for write methods, and HEAD without body output.
- [x] 301/302/303 method rewriting, 307/308 buffered body replay, non-replayable
  stdin handling, redirect limit/zero redirects and cross-origin auth stripping.
- [x] Self-signed HTTPS rejection, explicit -C acceptance, subsequent secure-default
  rejection; timeout before headers/during body, abrupt EOF/length mismatch.
- [x] Normal/raw/default/--fail statuses 200/201/204/302/400/404/500, malformed
  arguments, body limit, duplicate response headers, redacted request credentials,
  and quiet broken output pipe.
- [x] Forced color, disabled color, auto color on piped output, unchanged raw bytes,
  CLI version, root help and POST help.
- [ ] Trusted public TLS and proxy environment behavior.
- [ ] Actual terminal auto-color/visual review.
- [ ] Blocked-stdin cancellation, live interrupted/failed stdin sources, early
  replies, larger upload/RSS/resource campaign and other output-writer failures.

Rerun the authorized suite with:

```sh
# Build first using the workspace's documented compiler/CC override if needed.
cargo build --offline --locked --bin aqui
python3 tools/check_transport.py --binary target/debug/aqui
```

It starts two ephemeral loopback HTTP servers and one TLS server, generates a
short-lived self-signed certificate via installed OpenSSL in a temporary directory,
cleans up its fixtures/services, clears proxy settings for child processes, and
makes no external requests. Each child has an eight-second deadline. Test uploads
are at most 64 KiB; the broken-pipe response is 1 MiB. The suite checks all methods,
exact literal/stdin bytes, raw/normal/default/`--fail` status combinations,
301/302/303/307/308 replay semantics, redirect budgets, cross-origin auth stripping,
redacted metadata, duplicate response headers, timeouts, truncated responses,
body limits, argument errors, quiet broken pipes, and TLS verification/opt-out.

Remaining gaps despite the passing run: trusted public TLS, proxy behavior, actual terminal auto-color, blocked stdin
cancellation, interrupted/failed stdin sources on the live upload path, RSS,
and other redirected-output failures. No memory bound is claimed from the fixture
size alone. Syntax compilation of the Python file does not start servers.

Developer manual examples, from the repository after building. These contact the
chosen endpoint; use a controlled test instance, especially for modifying methods:

```sh
./target/debug/aqui --version
./target/debug/aqui --help
./target/debug/aqui post --help
./target/debug/aqui get "$AQUI_TEST_URL/get" --color never
./target/debug/aqui head "$AQUI_TEST_URL/get"
./target/debug/aqui options "$AQUI_TEST_URL/anything"
./target/debug/aqui post "$AQUI_TEST_URL/anything" '{"n":123456789012345678901234567890}'
./target/debug/aqui put "$AQUI_TEST_URL/anything" ''
printf '\000\377' | ./target/debug/aqui patch "$AQUI_TEST_URL/anything" -I
./target/debug/aqui delete "$AQUI_TEST_URL/anything"
```

These examples do not replace assertions against returned bytes/status or cover
the complete matrix. They have not been run by the agent.

## Scope decisions before publication

- [ ] Developer accepts the documented feature subset for 8.0.0, particularly
  absent request-body display/IP/status-code commands.
- [x] Revised HTTP-status exit policy is implemented and documented: completion
  succeeds in either output mode; `--fail` fails 4xx/5xx after output. unit- and process-tested by the loopback suite.
- [ ] Registry crate-name/ownership/version availability checked in developer's
  publishing context; not established by a local package build.

## Final developer actions

Git state, credentials, commit/tag/push and publication are developer-managed.
Do not publish from the workspace-local tool Cargo home. After source review,
validation and Git work, use a private credential context:

```sh
cargo publish --dry-run --locked
cargo publish --locked
```

The dry run can access the registry and is not the agent's local offline package
verification. Release-binary distribution/cross-platform CI are not configured;
8.0.0 currently prepares a Cargo source release. No publication has occurred.
