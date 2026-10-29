//! HTTP request construction. Building a request never contacts its endpoint.
use reqwest::{
    Method, Url,
    blocking::{Body, Client, Request},
    header::{
        ACCEPT, AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE, HOST, HeaderMap, HeaderName,
        HeaderValue, TRANSFER_ENCODING, USER_AGENT,
    },
    redirect::Policy,
};
use std::{error::Error, io::Read, time::Duration};

pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

/// Normalize a scheme-less address to HTTP, rejecting other protocols and URL credentials.
/// Supply basic-auth options or an explicit Authorization header instead of URL credentials.
pub fn normalize_url(input: &str) -> Result<Url> {
    if input.is_empty() || input.chars().any(char::is_whitespace) {
        return Err(
            "URL must be nonempty and contain no whitespace (percent-encode spaces)".into(),
        );
    }
    let text = if input.contains("://") {
        input.to_owned()
    } else {
        format!("http://{input}")
    };
    let mut url = Url::parse(&text)?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err("only HTTP(S) URLs with a host are supported".into());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("URL credentials are unsupported; supply an Authorization header".into());
    }
    url.set_fragment(None);
    Ok(url)
}

/// Parse headers without merging duplicate values. Split only at the first colon.
pub fn parse_headers(inputs: &[String]) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    for input in inputs {
        let (name, value) = input.split_once(':').ok_or("header requires NAME:VALUE")?;
        let name = HeaderName::from_bytes(name.trim_matches([' ', '\t']).as_bytes())?;
        let value = HeaderValue::from_str(value.trim_matches([' ', '\t']))?;
        headers.append(name, value);
    }
    if !headers.contains_key(USER_AGENT) {
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(concat!("aqui/", env!("CARGO_PKG_VERSION"))),
        );
    }
    if !headers.contains_key(ACCEPT) {
        headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
    }
    Ok(headers)
}

/// Snapshot the prepared initial request for display, including derived HTTP/1 Host.
/// Authorization, Proxy-Authorization and Cookie values are replaced with `[redacted]`.
/// This is not a trace of proxy headers or requests rewritten during redirects.
pub fn outbound_headers(request: &Request) -> Result<HeaderMap> {
    let mut headers = request.headers().clone();
    if !headers.contains_key(HOST) {
        let url = request.url();
        let host = url.host().ok_or("request URL requires a host")?.to_string();
        let authority = match url.port() {
            Some(port) => format!("{host}:{port}"),
            None => host,
        };
        headers.insert(HOST, HeaderValue::from_str(&authority)?);
    }
    for name in ["authorization", "proxy-authorization", "cookie"] {
        let count = headers.get_all(name).iter().count();
        headers.remove(name);
        for _ in 0..count {
            headers.append(name, HeaderValue::from_static("[redacted]"));
        }
    }
    Ok(headers)
}

/// Build an isolated client and GET request with a finite timeout and redirect budget.
/// A zero redirect budget disables following redirects rather than treating them as errors.
pub fn build_get(
    url: &str,
    headers: &[String],
    timeout: Duration,
    max_redirects: usize,
) -> Result<(Client, Request)> {
    build_request(
        Method::GET,
        url,
        headers,
        Payload::None,
        timeout,
        max_redirects,
    )
}

/// Explicit absence, known bytes, or an unbuffered streaming request body.
pub enum Payload {
    None,
    Bytes(Vec<u8>),
    Stream(Box<dyn Read + Send>),
}

/// Basic auth sends UTF-8 credentials. Password colons are allowed; username colons are not.
pub struct BasicAuth<'a> {
    pub username: &'a str,
    pub password: &'a str,
}

/// Per-client transport settings; certificate verification is enabled by default.
pub struct RequestOptions<'a> {
    pub timeout: Duration,
    pub max_redirects: usize,
    pub basic_auth: Option<BasicAuth<'a>>,
    pub allow_invalid_certs: bool,
}

impl Default for RequestOptions<'_> {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(5),
            max_redirects: 5,
            basic_auth: None,
            allow_invalid_certs: false,
        }
    }
}

/// Build one of the seven supported methods without contacting the endpoint.
/// GET/HEAD/OPTIONS reject bodies. Body framing is owned by the client.
/// Known JSON bytes are detected without changing their representation.
pub fn build_request(
    method: Method,
    url: &str,
    headers: &[String],
    payload: Payload,
    timeout: Duration,
    max_redirects: usize,
) -> Result<(Client, Request)> {
    build_request_with_options(
        method,
        url,
        headers,
        payload,
        RequestOptions {
            timeout,
            max_redirects,
            ..RequestOptions::default()
        },
    )
}

/// Build a request with optional Basic auth and a per-client TLS verification opt-out.
/// Authentication options conflict with any explicit Authorization header.
pub fn build_request_with_options(
    method: Method,
    url: &str,
    headers: &[String],
    payload: Payload,
    options: RequestOptions<'_>,
) -> Result<(Client, Request)> {
    let RequestOptions {
        timeout,
        max_redirects,
        basic_auth,
        allow_invalid_certs,
    } = options;
    if timeout.is_zero() {
        return Err("timeout must be positive".into());
    }
    let accepts_payload = matches!(
        method,
        Method::DELETE | Method::PATCH | Method::POST | Method::PUT
    );
    if !accepts_payload && !matches!(method, Method::GET | Method::HEAD | Method::OPTIONS) {
        return Err("unsupported HTTP method".into());
    }
    if !accepts_payload && !matches!(payload, Payload::None) {
        return Err("GET, HEAD and OPTIONS do not accept payloads".into());
    }
    let url = normalize_url(url)?;
    let mut headers = parse_headers(headers)?;
    if let Some(auth) = &basic_auth {
        if headers.contains_key(AUTHORIZATION) {
            return Err(
                "basic-auth options cannot be combined with an Authorization header".into(),
            );
        }
        if auth.username.contains(':')
            || auth
                .username
                .chars()
                .chain(auth.password.chars())
                .any(char::is_control)
        {
            return Err("basic-auth username must not contain a colon; credentials must not contain control characters".into());
        }
    }
    if headers.contains_key(CONTENT_LENGTH) || headers.contains_key(TRANSFER_ENCODING) {
        return Err("Content-Length and Transfer-Encoding are managed by the HTTP client".into());
    }
    let body = match payload {
        Payload::None => None,
        Payload::Bytes(bytes) => {
            if !headers.contains_key(CONTENT_TYPE) {
                let kind = if serde_json::from_slice::<serde_json::Value>(&bytes).is_ok() {
                    "application/json"
                } else {
                    "application/octet-stream"
                };
                headers.insert(CONTENT_TYPE, HeaderValue::from_static(kind));
            }
            headers.insert(
                CONTENT_LENGTH,
                HeaderValue::from_str(&bytes.len().to_string())?,
            );
            Some(Body::from(bytes))
        }
        Payload::Stream(reader) => {
            if !headers.contains_key(CONTENT_TYPE) {
                headers.insert(
                    CONTENT_TYPE,
                    HeaderValue::from_static("application/octet-stream"),
                );
            }
            Some(Body::new(reader))
        }
    };
    let policy = if max_redirects == 0 {
        Policy::none()
    } else {
        Policy::limited(max_redirects)
    };
    let client = Client::builder()
        .timeout(timeout)
        .redirect(policy)
        .danger_accept_invalid_certs(allow_invalid_certs)
        .build()?;
    let mut builder = client.request(method, url).headers(headers);
    if let Some(auth) = basic_auth {
        builder = builder.basic_auth(auth.username, Some(auth.password));
    }
    if let Some(body) = body {
        builder = builder.body(body);
    }
    Ok((client, builder.build()?))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn basic_auth_encoding_and_conflicts() {
        for (username, password, expected) in [
            ("user", "pass", "Basic dXNlcjpwYXNz"),
            ("user", "", "Basic dXNlcjo="),
            ("", "", "Basic Og=="),
            ("user", "p:a", "Basic dXNlcjpwOmE="),
            ("é", "😀", "Basic w6k68J+YgA=="),
        ] {
            let (_, request) = build_request_with_options(
                Method::GET,
                "host",
                &[],
                Payload::None,
                RequestOptions {
                    basic_auth: Some(BasicAuth { username, password }),
                    ..RequestOptions::default()
                },
            )
            .unwrap();
            assert_eq!(request.headers()[AUTHORIZATION], expected);
            assert_eq!(
                outbound_headers(&request).unwrap()[AUTHORIZATION],
                "[redacted]"
            );
            assert_eq!(request.headers()[AUTHORIZATION], expected);
        }
        for (username, password, headers) in [
            ("bad:user", "pass", vec![]),
            ("user\n", "pass", vec![]),
            ("user", "pass\0", vec![]),
            ("user", "pass", vec!["aUtHoRiZaTiOn: Bearer token".into()]),
        ] {
            assert!(
                build_request_with_options(
                    Method::GET,
                    "host",
                    &headers,
                    Payload::None,
                    RequestOptions {
                        basic_auth: Some(BasicAuth { username, password }),
                        ..RequestOptions::default()
                    }
                )
                .is_err()
            );
        }
        for insecure in [false, true, false] {
            assert!(
                build_request_with_options(
                    Method::GET,
                    "https://host",
                    &[],
                    Payload::None,
                    RequestOptions {
                        allow_invalid_certs: insecure,
                        ..RequestOptions::default()
                    }
                )
                .is_ok()
            );
        }
    }

    #[test]
    fn outbound_secrets_are_redacted_without_changing_request() {
        let (_, request) = build_get(
            "host",
            &[
                "Authorization: Bearer token".into(),
                "Cookie: a=1".into(),
                "Cookie: b=2".into(),
                "Proxy-Authorization: Basic secret".into(),
                "X-Visible: value".into(),
            ],
            Duration::from_secs(1),
            0,
        )
        .unwrap();
        let headers = outbound_headers(&request).unwrap();
        assert_eq!(headers["authorization"], "[redacted]");
        assert_eq!(headers["proxy-authorization"], "[redacted]");
        assert_eq!(
            headers
                .get_all("cookie")
                .iter()
                .map(|v| v.to_str().unwrap())
                .collect::<Vec<_>>(),
            ["[redacted]", "[redacted]"]
        );
        assert_eq!(headers["x-visible"], "value");
        assert_eq!(request.headers()[AUTHORIZATION], "Bearer token");
        assert_eq!(request.headers()["cookie"], "a=1");
    }
    #[test]
    fn url_corpus() {
        for (input, expected) in [
            ("example.com", "http://example.com/"),
            (
                "localhost:8080/a?q=1#fragment",
                "http://localhost:8080/a?q=1",
            ),
            (
                "https://example.com/a%20b?x=1&x=2",
                "https://example.com/a%20b?x=1&x=2",
            ),
            ("http://[::1]:8080", "http://[::1]:8080/"),
        ] {
            assert_eq!(normalize_url(input).unwrap().as_str(), expected);
        }
        for input in [
            "",
            " ",
            "http://",
            "https://a b",
            "ftp://example.com",
            "file:///tmp/a",
            "http://u:p@host",
            "http://host:99999",
            "http://[::1",
        ] {
            assert!(normalize_url(input).is_err(), "{input}");
        }
    }
    #[test]
    fn headers_preserve_values_and_override_default() {
        let headers = parse_headers(&[
            " X-Test : a:b ".into(),
            "x-test:".into(),
            "user-AGENT: custom".into(),
        ])
        .unwrap();
        assert_eq!(
            headers
                .get_all("x-test")
                .iter()
                .map(|v| v.to_str().unwrap())
                .collect::<Vec<_>>(),
            ["a:b", ""]
        );
        assert_eq!(headers[USER_AGENT], "custom");
        assert_eq!(
            parse_headers(&[]).unwrap()[USER_AGENT],
            concat!("aqui/", env!("CARGO_PKG_VERSION"))
        );
        for input in ["missing", ":x", "Bad Name:x", "X:a\r\nb:c", "X:\0"] {
            assert!(parse_headers(&[input.into()]).is_err(), "{input:?}");
        }
    }
    #[test]
    fn get_request_without_network() {
        let (_client, request) = build_get(
            "example.invalid/a",
            &["X: y".into()],
            Duration::from_millis(17),
            0,
        )
        .unwrap();
        assert_eq!(request.method(), reqwest::Method::GET);
        assert_eq!(request.url().as_str(), "http://example.invalid/a");
        assert_eq!(request.headers()["x"], "y");
        assert!(request.body().is_none());
        assert!(build_get("host", &[], Duration::ZERO, 5).is_err());
    }

    #[test]
    fn methods_and_exact_payloads() {
        for method in [
            Method::GET,
            Method::HEAD,
            Method::OPTIONS,
            Method::DELETE,
            Method::PATCH,
            Method::POST,
            Method::PUT,
        ] {
            let write = matches!(
                method,
                Method::DELETE | Method::PATCH | Method::POST | Method::PUT
            );
            let (_, request) = build_request(
                method.clone(),
                "host",
                &[],
                Payload::None,
                Duration::from_secs(1),
                0,
            )
            .unwrap();
            assert_eq!(request.method(), method);
            assert!(request.body().is_none());
            assert!(!request.headers().contains_key(CONTENT_TYPE));
            for (bytes, expected) in [
                (b"".as_slice(), "application/octet-stream"),
                (b"null", "application/json"),
                (b"false", "application/json"),
                (b"123456789012345678901234567890", "application/json"),
                (br#" {"z":1, "z":2} "#, "application/json"),
                (b"{bad", "application/octet-stream"),
                ("héllo".as_bytes(), "application/octet-stream"),
                (b"\0\xff", "application/octet-stream"),
            ] {
                let result = build_request(
                    method.clone(),
                    "host",
                    &[],
                    Payload::Bytes(bytes.to_vec()),
                    Duration::from_secs(1),
                    0,
                );
                assert_eq!(result.is_ok(), write);
                if let Ok((_, request)) = result {
                    assert_eq!(request.body().unwrap().as_bytes().unwrap(), bytes);
                    assert_eq!(request.headers()[CONTENT_LENGTH], bytes.len().to_string());
                    assert_eq!(request.headers()[CONTENT_TYPE], expected);
                }
            }
        }
        let (_, request) = build_request(
            Method::POST,
            "host",
            &["Content-Type: text/plain".into()],
            Payload::Bytes(b"{}".to_vec()),
            Duration::from_secs(1),
            0,
        )
        .unwrap();
        assert_eq!(request.headers()[CONTENT_TYPE], "text/plain");
        for header in ["Content-Length: 999", "Transfer-Encoding: chunked"] {
            assert!(
                build_request(
                    Method::POST,
                    "host",
                    &[header.into()],
                    Payload::Bytes(vec![]),
                    Duration::from_secs(1),
                    0
                )
                .is_err()
            );
        }
        assert!(
            build_request(
                Method::TRACE,
                "host",
                &[],
                Payload::None,
                Duration::from_secs(1),
                0
            )
            .is_err()
        );
    }

    #[test]
    fn streaming_payload_is_lazy_and_propagates_read_errors() {
        use std::io::{self, Read};
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        struct Reader {
            reads: Arc<AtomicUsize>,
            interrupted: bool,
            fail: bool,
            bytes: &'static [u8],
        }
        impl Read for Reader {
            fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
                self.reads.fetch_add(1, Ordering::SeqCst);
                if !self.interrupted {
                    self.interrupted = true;
                    return Err(io::ErrorKind::Interrupted.into());
                }
                if buf.is_empty() {
                    return Ok(0);
                }
                if let Some((first, rest)) = self.bytes.split_first() {
                    buf[0] = *first;
                    self.bytes = rest;
                    return Ok(1);
                }
                if self.fail {
                    Err(io::Error::new(io::ErrorKind::UnexpectedEof, "late EOF"))
                } else {
                    Ok(0)
                }
            }
        }
        for fail in [false, true] {
            let reads = Arc::new(AtomicUsize::new(0));
            let payload = Payload::Stream(Box::new(Reader {
                reads: reads.clone(),
                interrupted: false,
                fail,
                bytes: b"\0\xff{}",
            }));
            let (_, mut request) =
                build_request(Method::PUT, "host", &[], payload, Duration::from_secs(1), 0)
                    .unwrap();
            assert_eq!(reads.load(Ordering::SeqCst), 0);
            assert_eq!(request.headers()[CONTENT_TYPE], "application/octet-stream");
            assert!(!request.headers().contains_key(CONTENT_LENGTH));
            assert!(request.body().unwrap().as_bytes().is_none());
            let result = request.body_mut().as_mut().unwrap().buffer();
            if fail {
                assert!(result.is_err());
            } else {
                assert_eq!(result.unwrap(), b"\0\xff{}");
            }
        }
        assert!(
            build_request(
                Method::GET,
                "host",
                &[],
                Payload::Stream(Box::new(io::empty())),
                Duration::from_secs(1),
                0
            )
            .is_err()
        );
    }

    #[test]
    fn outbound_defaults_custom_headers_and_authority() {
        for (url, host) in [
            ("http://example.invalid/a", "example.invalid"),
            ("https://example.invalid:8443/a", "example.invalid:8443"),
            ("http://[::1]:8080/a", "[::1]:8080"),
            ("https://[::1]/", "[::1]"),
        ] {
            let (_client, request) = build_get(
                url,
                &["X-Test: one".into(), "X-Test: two".into()],
                Duration::from_secs(1),
                0,
            )
            .unwrap();
            let headers = outbound_headers(&request).unwrap();
            assert_eq!(headers[HOST], host);
            assert_eq!(headers[ACCEPT], "*/*");
            assert_eq!(
                headers[USER_AGENT],
                concat!("aqui/", env!("CARGO_PKG_VERSION"))
            );
            assert_eq!(
                headers
                    .get_all("x-test")
                    .iter()
                    .map(|v| v.to_str().unwrap())
                    .collect::<Vec<_>>(),
                ["one", "two"]
            );
            assert!(!request.headers().contains_key(HOST));
        }
        let (_client, request) = build_get(
            "https://example.invalid",
            &[
                "Host: custom.invalid".into(),
                "Accept: application/json".into(),
                "User-Agent: custom".into(),
            ],
            Duration::from_secs(1),
            0,
        )
        .unwrap();
        let headers = outbound_headers(&request).unwrap();
        assert_eq!(headers[HOST], "custom.invalid");
        assert_eq!(headers[ACCEPT], "application/json");
        assert_eq!(headers[USER_AGENT], "custom");
    }
}
