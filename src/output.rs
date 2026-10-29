//! Styled output and bounded response buffering, independent of HTTP transport.
use crate::colors::{Color, Style, status_color};
use reqwest::{StatusCode, Url, header::HeaderMap};
use std::{
    io::{self, Read, Write},
    time::Duration,
};

/// Copy response bytes without decoding or adding a newline, then flush the destination.
/// Reader, writer, and final flush errors (including broken pipes) propagate.
pub fn write_raw(mut reader: impl Read, mut writer: impl Write) -> io::Result<u64> {
    let bytes = io::copy(&mut reader, &mut writer)?;
    writer.flush()?;
    Ok(bytes)
}

/// Buffer at most `limit + 1` bytes to distinguish an exact fit from overflow.
/// No display output is written if buffering fails or exceeds the limit.
pub fn read_body(reader: impl Read, limit: u64) -> io::Result<Vec<u8>> {
    let cap = limit
        .checked_add(1)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "body limit too large"))?;
    let mut body = Vec::new();
    reader.take(cap).read_to_end(&mut body)?;
    if body.len() as u64 > limit {
        return Err(io::Error::other(
            "response exceeds --max-body-bytes; use -D to stream it",
        ));
    }
    Ok(body)
}

/// Render body, sorted response headers, final URL, status, and elapsed time.
/// Valid JSON is indented; all other bodies are written as received. No ANSI styling.
pub fn write_response(
    writer: impl Write,
    body: &[u8],
    headers: &HeaderMap,
    url: &Url,
    status: StatusCode,
    elapsed: Duration,
) -> io::Result<()> {
    write_response_styled(writer, body, headers, url, status, elapsed, Style::Plain)
}

/// Render a response with explicit styling. ANSI bytes do not affect header alignment.
/// Raw/unparsed bodies are never syntax highlighted.
pub fn write_response_styled(
    writer: impl Write,
    body: &[u8],
    headers: &HeaderMap,
    url: &Url,
    status: StatusCode,
    elapsed: Duration,
    style: Style,
) -> io::Result<()> {
    write_exchange(
        writer,
        ResponseDetails {
            body,
            headers,
            url,
            status,
            elapsed,
        },
        None,
        style,
    )
}

/// The response body and metadata, including the final response URL.
pub struct ResponseDetails<'a> {
    pub body: &'a [u8],
    pub headers: &'a HeaderMap,
    pub url: &'a Url,
    pub status: StatusCode,
    pub elapsed: Duration,
}

/// Prepared initial request metadata, not a redirect/proxy wire trace.
pub struct RequestDetails<'a> {
    pub method: &'a reqwest::Method,
    pub url: &'a Url,
    pub headers: &'a HeaderMap,
}

/// Render response details followed by the initial method and outbound headers.
/// If redirected, show the final URL separately to avoid attributing initial headers to it.
pub fn write_exchange(
    mut writer: impl Write,
    response: ResponseDetails<'_>,
    request: Option<RequestDetails<'_>>,
    style: Style,
) -> io::Result<()> {
    let ResponseDetails {
        body,
        headers,
        url,
        status,
        elapsed,
    } = response;
    if !body.is_empty() {
        let formatted = serde_json::from_slice::<serde_json::Value>(body)
            .ok()
            .and_then(|value| serde_json::to_vec_pretty(&value).ok());
        let data = formatted.as_deref().unwrap_or(body);
        if formatted.is_some() {
            style.json(&mut writer, data)?;
        } else {
            writer.write_all(data)?;
        }
        if !data.ends_with(b"\n") {
            writer.write_all(b"\n")?;
        }
        writer.write_all(b"\n")?;
    }
    write_headers(&mut writer, headers, style)?;
    writer.write_all(b"\n")?;
    style.paint(
        &mut writer,
        Color::Orange,
        request.as_ref().map_or(b"GET".as_slice(), |request| {
            request.method.as_str().as_bytes()
        }),
    )?;
    writer.write_all(b" ")?;
    style.url(
        &mut writer,
        request.as_ref().map_or(url, |request| request.url),
    )?;
    writer.write_all(b"\n\n")?;
    if let Some(request) = request {
        write_headers(&mut writer, request.headers, style)?;
        writer.write_all(b"\n")?;
        if request.url != url {
            writer.write_all(b"Final URL: ")?;
            style.url(&mut writer, url)?;
            writer.write_all(b"\n\n")?;
        }
    }
    writer.write_all(b"[")?;
    let status_style = status_color(status);
    style.paint(
        &mut writer,
        status_style,
        status.as_u16().to_string().as_bytes(),
    )?;
    writer.write_all(b"] ")?;
    style.paint(
        &mut writer,
        status_style,
        status.canonical_reason().unwrap_or("Unknown").as_bytes(),
    )?;
    writer.write_all(b" (")?;
    style.paint(
        &mut writer,
        Color::Orange,
        format!("{:.3} ms", elapsed.as_secs_f64() * 1000.0).as_bytes(),
    )?;
    writer.write_all(b")\n")?;
    writer.flush()
}

fn write_headers(mut writer: impl Write, headers: &HeaderMap, style: Style) -> io::Result<()> {
    let mut names: Vec<_> = headers.keys().collect();
    names.sort_by_key(|name| name.as_str());
    let width = names
        .iter()
        .map(|name| name.as_str().len())
        .max()
        .unwrap_or(0);
    for name in names {
        for value in headers.get_all(name) {
            write!(writer, "{}", " ".repeat(width - name.as_str().len()))?;
            style.paint(&mut writer, Color::Yellow, name.as_str().as_bytes())?;
            writer.write_all(b" : ")?;
            style.paint(&mut writer, Color::Blue, value.as_bytes())?;
            writeln!(writer)?;
        }
    }
    Ok(())
}

/// Completed HTTP responses succeed in either output mode unless `--fail` is set.
/// With `--fail`, HTTP 4xx/5xx fail after output; transport and I/O errors always fail.
pub fn status_success(status: StatusCode, fail_http: bool) -> bool {
    !fail_http || !(status.is_client_error() || status.is_server_error())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct FailingReader;
    impl Read for FailingReader {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::UnexpectedEof, "truncated"))
        }
    }
    struct Writer {
        bytes: Vec<u8>,
        fail_write: bool,
        fail_flush: bool,
    }
    impl Write for Writer {
        fn write(&mut self, data: &[u8]) -> io::Result<usize> {
            if self.fail_write {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed"));
            }
            let n = data.len().min(1);
            self.bytes.extend_from_slice(&data[..n]);
            Ok(n)
        }
        fn flush(&mut self) -> io::Result<()> {
            if self.fail_flush {
                Err(io::Error::other("flush failed"))
            } else {
                Ok(())
            }
        }
    }
    fn writer() -> Writer {
        Writer {
            bytes: vec![],
            fail_write: false,
            fail_flush: false,
        }
    }
    #[test]
    fn raw_exact_bytes_partial_writes_and_errors() {
        for data in [b"".as_slice(), b"\0\xff\n{}"] {
            let mut output = writer();
            assert_eq!(write_raw(data, &mut output).unwrap(), data.len() as u64);
            assert_eq!(output.bytes, data);
        }
        assert_eq!(
            write_raw(FailingReader, writer()).unwrap_err().kind(),
            io::ErrorKind::UnexpectedEof
        );
        let mut output = writer();
        output.fail_write = true;
        assert_eq!(
            write_raw(b"x".as_slice(), output).unwrap_err().kind(),
            io::ErrorKind::BrokenPipe
        );
        let mut output = writer();
        output.fail_flush = true;
        assert!(write_raw(b"".as_slice(), output).is_err());
    }

    #[test]
    fn short_interrupted_reads_and_late_eof_error() {
        struct Reader<'a> {
            bytes: &'a [u8],
            interrupted: bool,
            fail_at_eof: bool,
        }
        impl Read for Reader<'_> {
            fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
                if !self.interrupted {
                    self.interrupted = true;
                    return Err(io::ErrorKind::Interrupted.into());
                }
                if buffer.is_empty() {
                    return Ok(0);
                }
                match self.bytes.split_first() {
                    Some((byte, rest)) => {
                        buffer[0] = *byte;
                        self.bytes = rest;
                        Ok(1)
                    }
                    None if self.fail_at_eof => Err(io::ErrorKind::UnexpectedEof.into()),
                    None => Ok(0),
                }
            }
        }
        let bytes = "héllo\0".as_bytes();
        for fail_at_eof in [false, true] {
            let make_reader = || Reader {
                bytes,
                interrupted: false,
                fail_at_eof,
            };
            let mut output = writer();
            let result = write_raw(make_reader(), &mut output);
            assert_eq!(output.bytes, bytes);
            assert_eq!(result.is_err(), fail_at_eof);
            let result = read_body(make_reader(), bytes.len() as u64);
            if fail_at_eof {
                assert_eq!(result.unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
            } else {
                assert_eq!(result.unwrap(), bytes);
            }
        }
    }
    #[test]
    fn bounded_body() {
        for limit in [0, 1, 8] {
            let body = vec![b'x'; limit as usize];
            assert_eq!(read_body(body.as_slice(), limit).unwrap(), body);
            assert!(read_body(vec![b'x'; limit as usize + 1].as_slice(), limit).is_err());
        }
        assert_eq!(
            read_body(FailingReader, 8).unwrap_err().kind(),
            io::ErrorKind::UnexpectedEof
        );
        assert!(read_body(b"".as_slice(), u64::MAX).is_err());
    }
    #[test]
    fn exact_plain_response() {
        let mut headers = HeaderMap::new();
        headers.append("z", "last".parse().unwrap());
        headers.append("a", "one".parse().unwrap());
        headers.append("a", "two".parse().unwrap());
        let mut output = writer();
        write_response(
            &mut output,
            br#"{"z":123456789012345678901234567890,"a":true}"#,
            &headers,
            &Url::parse("http://example.invalid/").unwrap(),
            StatusCode::OK,
            Duration::from_micros(1234),
        )
        .unwrap();
        assert_eq!(
            String::from_utf8(output.bytes).unwrap(),
            "{\n  \"z\": 123456789012345678901234567890,\n  \"a\": true\n}\n\na : one\na : two\nz : last\n\nGET http://example.invalid/\n\n[200] OK (1.234 ms)\n"
        );
    }
    #[test]
    fn plain_body_corpus_and_failures() {
        let url = Url::parse("http://example.invalid/").unwrap();
        let suffix = b"\nGET http://example.invalid/\n\n[404] Not Found (0.000 ms)\n";
        for (body, prefix) in [
            (b"".as_slice(), b"".as_slice()),
            (b"text", b"text\n\n"),
            (b"text\n", b"text\n\n"),
            (b"\xff", b"\xff\n\n"),
            (b"{bad", b"{bad\n\n"),
        ] {
            let mut output = writer();
            write_response(
                &mut output,
                body,
                &HeaderMap::new(),
                &url,
                StatusCode::NOT_FOUND,
                Duration::ZERO,
            )
            .unwrap();
            assert_eq!(output.bytes, [prefix, suffix].concat());
        }
        for flush in [false, true] {
            let mut output = writer();
            output.fail_write = !flush;
            output.fail_flush = flush;
            assert!(
                write_response(
                    output,
                    b"x",
                    &HeaderMap::new(),
                    &url,
                    StatusCode::OK,
                    Duration::ZERO
                )
                .is_err()
            );
        }
    }
    #[test]
    fn exit_status_contract() {
        for code in [
            100, 199, 200, 201, 204, 299, 301, 304, 399, 400, 404, 499, 500, 599,
        ] {
            let status = StatusCode::from_u16(code).unwrap();
            assert!(status_success(status, false));
            assert_eq!(status_success(status, true), code < 400, "status {code}");
        }
    }

    #[test]
    fn exchange_displays_actual_method() {
        let url = Url::parse("https://example.invalid/").unwrap();
        for method in [
            reqwest::Method::GET,
            reqwest::Method::HEAD,
            reqwest::Method::OPTIONS,
            reqwest::Method::DELETE,
            reqwest::Method::PATCH,
            reqwest::Method::POST,
            reqwest::Method::PUT,
        ] {
            for style in [Style::Plain, Style::Colorful] {
                let mut bytes = Vec::new();
                write_exchange(
                    &mut bytes,
                    ResponseDetails {
                        body: b"",
                        headers: &HeaderMap::new(),
                        url: &url,
                        status: StatusCode::NO_CONTENT,
                        elapsed: Duration::ZERO,
                    },
                    Some(RequestDetails {
                        method: &method,
                        url: &url,
                        headers: &HeaderMap::new(),
                    }),
                    style,
                )
                .unwrap();
                let plain = String::from_utf8(crate::colors::tests::strip_ansi(&bytes)).unwrap();
                assert!(plain.contains(&format!("\n{method} https://example.invalid/\n")));
                if method != reqwest::Method::GET {
                    assert!(!plain.contains("GET "));
                }
            }
        }
    }

    #[test]
    fn outbound_headers_in_exchange() {
        let url = Url::parse("https://example.invalid/a/b").unwrap();
        let (_, request) = crate::request::build_get(
            url.as_str(),
            &["X-Test: a".into(), "X-Test: b".into()],
            Duration::from_secs(1),
            0,
        )
        .unwrap();
        let outbound = crate::request::outbound_headers(&request).unwrap();
        let mut headers = HeaderMap::new();
        headers.insert("server", "test".parse().unwrap());
        let expected = concat!(
            "server : test\n\nGET https://example.invalid/a/b\n\n",
            "    accept : */*\n      host : example.invalid\nuser-agent : aqui/",
            env!("CARGO_PKG_VERSION"),
            "\n    x-test : a\n    x-test : b\n\n[200] OK (0.000 ms)\n"
        );
        for style in [Style::Plain, Style::Colorful] {
            let mut output = writer();
            write_exchange(
                &mut output,
                ResponseDetails {
                    body: b"",
                    headers: &headers,
                    url: &url,
                    status: StatusCode::OK,
                    elapsed: Duration::ZERO,
                },
                Some(RequestDetails {
                    method: &reqwest::Method::GET,
                    url: &url,
                    headers: &outbound,
                }),
                style,
            )
            .unwrap();
            assert_eq!(
                crate::colors::tests::strip_ansi(&output.bytes),
                expected.as_bytes()
            );
            if style == Style::Colorful {
                assert!(
                    String::from_utf8_lossy(&output.bytes)
                        .contains("/\x1b[38;2;153;85;255ma\x1b[0m/\x1b[38;2;153;85;255mb\x1b[0m")
                );
                assert!(String::from_utf8_lossy(&output.bytes).contains(
                    "\x1b[38;2;221;204;51mhost\x1b[0m : \x1b[38;2;51;170;221mexample.invalid\x1b[0m"
                ));
            }
        }
        let final_url = Url::parse("https://other.invalid/final").unwrap();
        let mut output = Vec::new();
        write_exchange(
            &mut output,
            ResponseDetails {
                body: b"",
                headers: &headers,
                url: &final_url,
                status: StatusCode::OK,
                elapsed: Duration::ZERO,
            },
            Some(RequestDetails {
                method: &reqwest::Method::GET,
                url: &url,
                headers: &outbound,
            }),
            Style::Plain,
        )
        .unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("GET https://example.invalid/a/b\n\n"));
        assert!(text.contains("\nFinal URL: https://other.invalid/final\n\n[200]"));
        for flush in [false, true] {
            let mut output = writer();
            output.fail_write = !flush;
            output.fail_flush = flush;
            assert!(
                write_exchange(
                    output,
                    ResponseDetails {
                        body: b"",
                        headers: &headers,
                        url: &url,
                        status: StatusCode::OK,
                        elapsed: Duration::ZERO
                    },
                    Some(RequestDetails {
                        method: &reqwest::Method::GET,
                        url: &url,
                        headers: &outbound
                    }),
                    Style::Colorful
                )
                .is_err()
            );
        }
    }

    #[test]
    fn styled_response_alignment_status_and_io() {
        use crate::colors::tests::strip_ansi;
        let mut headers = HeaderMap::new();
        headers.append("a", "one".parse().unwrap());
        headers.append("long-name", "two".parse().unwrap());
        let url = Url::parse("https://example.invalid:8443/a?q=1&q=2").unwrap();
        for (code, rgb) in [
            (100, "51;170;221"),
            (200, "102;187;51"),
            (302, "153;85;255"),
            (404, "221;68;51"),
            (503, "221;204;51"),
        ] {
            let status = StatusCode::from_u16(code).unwrap();
            for body in [
                r#"{"x":"é", "n":null}"#.as_bytes(),
                b"plain text",
                b"\xff",
                b"",
            ] {
                let mut plain = writer();
                let mut colored = writer();
                write_response(&mut plain, body, &headers, &url, status, Duration::ZERO).unwrap();
                write_response_styled(
                    &mut colored,
                    body,
                    &headers,
                    &url,
                    status,
                    Duration::ZERO,
                    Style::Colorful,
                )
                .unwrap();
                assert_eq!(strip_ansi(&colored.bytes), plain.bytes);
                let text = String::from_utf8_lossy(&colored.bytes);
                assert!(text.contains(&format!("[\x1b[38;2;{rgb}m{code}\x1b[0m]")));
                assert!(text.contains(
                    "        \x1b[38;2;221;204;51ma\x1b[0m : \x1b[38;2;51;170;221mone\x1b[0m"
                ));
                assert!(text.contains("\x1b[38;2;238;136;51mGET\x1b[0m"));
            }
        }
        for flush in [false, true] {
            let mut output = writer();
            output.fail_write = !flush;
            output.fail_flush = flush;
            assert!(
                write_response_styled(
                    output,
                    b"{}",
                    &headers,
                    &url,
                    StatusCode::OK,
                    Duration::ZERO,
                    Style::Colorful
                )
                .is_err()
            );
        }
    }
}
