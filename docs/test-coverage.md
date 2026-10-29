# Behavior-to-test coverage

Only isolated unit tests are part of the default verification. No HTTP services
or external endpoints are contacted by these tests.

| Behavior | Tests |
| --- | --- |
| Required GET URL, unknown commands/options, finite positive timeout/body limit | `args::tests::command_contract` |
| Repeatable headers, raw option, zero redirect budget and defaults | `args::tests::command_contract` |
| Version and root/GET help, GET `-h` reserved for headers | `args::tests::help_and_version`, `command_contract` |
| Scheme-less/explicit URLs, IPv6, query/fragment preservation/removal, bad URLs and credentials | `request::tests::url_corpus` |
| Header colon/value rules, duplicates, user-agent override and injection rejection | `request::tests::headers_preserve_values_and_override_default` |
| Construct GET with headers and no body without executing it; zero timeout rejected | `request::tests::get_request_without_network` |
| Raw binary bytes, no added newline, partial writes, read/broken-pipe/flush errors | `output::tests::raw_exact_bytes_partial_writes_and_errors` |
| One-byte UTF-8/binary reads, interrupted reads, errors after partial input | `output::tests::short_interrupted_reads_and_late_eof_error` |
| Exact-fit/overflow/zero body limits, EOF failure, unrepresentable limit | `output::tests::bounded_body` |
| Exact plain output, JSON indentation/order/large integers, sorted duplicate headers/timing | `output::tests::exact_plain_response` |
| Empty, newline-terminated, invalid JSON and binary bodies; write/flush failures | `output::tests::plain_body_corpus_and_failures` |
| Uniform completion/opt-in --fail HTTP exit policy, including 201/204 | `output::tests::exit_status_contract` (failed against prior policy before fix) |
| Default auto color, terminal/nonterminal resolution, explicit always/never, global flag placement, raw-mode argument compatibility | `args::tests::color_modes_and_terminal_detection` |
| Exact JSON token palette, UTF-8/escapes/numbers, unchanged text after removing generated ANSI | `colors::tests::json_palette_and_serialized_tokens` |
| URL component palette and preservation of IPv6, repeated/escaped/empty query values | `colors::tests::urls_preserve_serialization_and_components` |
| Styled/plain response equivalence, header alignment, status classes, partial writes and write/flush failures | `output::tests::styled_response_alignment_status_and_io` |
| Outbound defaults/overrides/duplicates, derived Host with IPv6 and ports | `request::tests::outbound_defaults_custom_headers_and_authority` |
| Exact outbound block order/alignment/style, initial versus final URL, writer errors | `output::tests::outbound_headers_in_exchange` |
| All seven methods, write-only payloads, absent/empty/literal/stdin input, conflicts and method help | `args::tests::methods_and_payload_sources` |
| Method construction, exact JSON/text/binary body bytes, content type/default override, managed framing | `request::tests::methods_and_exact_payloads` |
| Streaming body construction is lazy; one-byte/interrupted input and late read errors during isolated body consumption | `request::tests::streaming_payload_is_lazy_and_propagates_read_errors` |
| Every method displayed correctly in plain and styled output | `output::tests::exchange_displays_actual_method` |
| Auth/TLS/--fail flags on all methods, secure defaults, password dependency | `args::tests::auth_tls_and_http_failure_options` |
| Basic auth exact encoding, empty/UTF-8 credentials, invalid username/controls, Authorization conflicts | `request::tests::basic_auth_encoding_and_conflicts` |
| Outbound credential redaction preserves request bytes and duplicate cookie count | `request::tests::outbound_secrets_are_redacted_without_changing_request` |
| Status lookup parsing, literal metacharacters, unofficial flags; IP formats/conflicts/timeout/help | `args::tests::lookup_commands` |
| Status catalog ordering, duplicate codes, matching, unofficial precedence, exact output, style equivalence and writer failures | `status::tests::catalog_filters_and_styling`, `exact_output_and_errors` |
| Payload absent/empty/JSON/hex, exact output, preview limit boundaries, styling and writer failure | `output::tests::payload_previews_and_failures`, `payload_placement_in_exchange` |
| Stdin source-read count, EOF and interrupted read preservation | `upload::tests::counts_actual_reads_and_preserves_errors` |
| IP normalized schema, IPv4/IPv6, missing/invalid fields, coordinates, provider failure and malformed JSON | `ip::tests::validates_and_normalizes_provider_schema`, `provider_read_errors_limits_and_status` |
| IP formats, exact CLI/JSON/human output, raw/normalized objects, plain compact machine output, colored pretty JSON regression (failed before fix), partial writes and writer/flush errors | `ip::tests::output_modes_exact_and_color`, `partial_writes_and_failures` |

| IP pretty auto color for terminal versus pipe, explicit always/never, compact output always plain and parseable | `args::tests::ip_pretty_color_destination_contract` |

## Integration evidence

Developer-authorized `tools/check_transport.py` passed all **80** bounded loopback
HTTP/TLS/process cases again on 2026-10-29 against the rebuilt Rust 1.87 binary
including the lookup/payload additions. No external requests. It covers all
methods, literal and 64 KiB binary stdin bytes, HEAD, status/exit combinations,
redirects/auth stripping/replay, self-signed TLS rejection and explicit -C
acceptance, timeouts/truncation/body limits, duplicates, credential display,
broken pipes, help/version and forced/disabled/piped-auto color. IP-provider
access is not exercised by this suite.

The initial redirect-limit fixture accidentally reached a final successful route;
corrected it to loop and reran successfully. No Rust client changes were required.
See releasing.md for exact scope and commands. This execution authorization applies
to the loopback suite, not external endpoints or broader campaigns.

## Explicit gaps

- Trusted public TLS, proxy behavior, real terminal auto-color/visual output,
  blocked stdin cancellation, early replies and live interrupted/failed stdin sources.
- Outbound metadata remains a prepared initial snapshot, not a wire trace of
  redirects/proxies or protocol-generated framing.
- A 64 KiB test upload and 1 MiB broken-pipe response do not establish RSS or
  long-stream resource bounds. Other redirected-output failures remain untested
  at process level, although isolated writer-failure tests pass.
- Live IP provider/schema/availability validation and color customization remain gaps.
- No performance, RSS, fuzz, cross-platform, or legacy-execution validation.
