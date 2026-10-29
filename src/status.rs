//! Offline status catalog retained from node-http, with current registered names.
use crate::colors::{Style, status_color};
use std::io::{self, Write};

/// Historical catalog provenance; unofficial entries are informational, not standards.
pub const SOURCE: &str = "https://en.wikipedia.org/wiki/List_of_HTTP_status_codes";
/// (code, unofficial, description). Duplicate codes represent distinct usages.
pub const CODES: &[(u16, bool, &str)] = &[
    (100, false, "Continue"),
    (101, false, "Switching Protocols"),
    (102, false, "Processing"),
    (103, false, "Early Hints"),
    (103, true, "Checkpoint"),
    (200, false, "OK"),
    (201, false, "Created"),
    (202, false, "Accepted"),
    (203, false, "Non-Authoritative Information"),
    (204, false, "No Content"),
    (205, false, "Reset Content"),
    (206, false, "Partial Content"),
    (207, false, "Multi-Status"),
    (208, false, "Already Reported"),
    (218, true, "This is fine (Apache Web Server)"),
    (226, false, "IM Used"),
    (299, true, "Disappointed (https://bit.ly/35FRKNu)"),
    (300, false, "Multiple Choices"),
    (301, false, "Moved Permanently"),
    (302, false, "Found"),
    (303, false, "See Other"),
    (304, false, "Not Modified"),
    (305, false, "Use Proxy"),
    (306, false, "(Unused)"),
    (307, false, "Temporary Redirect"),
    (308, false, "Permanent Redirect"),
    (400, false, "Bad Request"),
    (401, false, "Unauthorized"),
    (402, false, "Payment Required"),
    (403, false, "Forbidden"),
    (404, false, "Not Found"),
    (405, false, "Method Not Allowed"),
    (406, false, "Not Acceptable"),
    (407, false, "Proxy Authentication Required"),
    (408, false, "Request Timeout"),
    (409, false, "Conflict"),
    (410, false, "Gone"),
    (411, false, "Length Required"),
    (412, false, "Precondition Failed"),
    (413, false, "Content Too Large"),
    (414, false, "URI Too Long"),
    (415, false, "Unsupported Media Type"),
    (416, false, "Range Not Satisfiable"),
    (417, false, "Expectation Failed"),
    (418, false, "(Unused) — historically I'm a teapot"),
    (419, true, "Page Expired (Laravel Framework)"),
    (420, true, "Method Failure (Spring Framework)"),
    (420, true, "Enhance your calm (Twitter)"),
    (421, false, "Misdirected Request"),
    (422, false, "Unprocessable Content"),
    (423, false, "Locked"),
    (424, false, "Failed Dependency"),
    (425, false, "Too Early"),
    (426, false, "Upgrade Required"),
    (428, false, "Precondition Required"),
    (429, false, "Too Many Requests"),
    (430, true, "Request Header Fields Too Large (Shopify)"),
    (431, false, "Request Header Fields Too Large"),
    (440, true, "Login Time-out"),
    (444, true, "No Response (nginx)"),
    (449, true, "Retry With"),
    (
        450,
        true,
        "Blocked by Windows Parental Controls (Microsoft)",
    ),
    (451, false, "Unavailable For Legal Reasons"),
    (451, true, "Redirect (Exchange ActiveSync)"),
    (
        460,
        true,
        "Client closed connection before idle timeout (AWS ELB)",
    ),
    (
        463,
        true,
        "More than 30 IPs in X-Forwarded-For header (AWS ELB)",
    ),
    (494, true, "Request header too large (nginx)"),
    (495, true, "SSL Certificate Error (nginx)"),
    (496, true, "SSL Certificate Required (nginx)"),
    (497, true, "HTTP Request Sent to HTTPS Port (nginx)"),
    (498, true, "Invalid Token (Esri)"),
    (499, true, "Token Required (Esri)"),
    (499, true, "Client Closed Request (nginx)"),
    (500, false, "Internal Server Error"),
    (501, false, "Not Implemented"),
    (502, false, "Bad Gateway"),
    (503, false, "Service Unavailable"),
    (504, false, "Gateway Timeout"),
    (505, false, "HTTP Version Not Supported"),
    (506, false, "Variant Also Negotiates"),
    (507, false, "Insufficient Storage"),
    (508, false, "Loop Detected"),
    (
        509,
        true,
        "Bandwidth Limit Exceeded (Apache Web Server/cPanel)",
    ),
    (510, false, "Not Extended"),
    (511, false, "Network Authentication Required"),
    (520, true, "Unknown Error (Cloudflare)"),
    (521, true, "Web Server Is Down (Cloudflare)"),
    (522, true, "Connection Timed Out (Cloudflare)"),
    (523, true, "Origin Is Unreachable (Cloudflare)"),
    (524, true, "A Timeout Occurred (Cloudflare)"),
    (525, true, "SSL Handshake Failed (Cloudflare)"),
    (526, true, "Invalid SSL Certificate (Cloudflare)"),
    (527, true, "Railgun Error (Cloudflare)"),
    (529, true, "Site is overloaded (Qualsys SSLLabs)"),
    (530, true, "Site is frozen (Pantheon)"),
    (530, true, "Error extension indicator 1xxx (CLoudflare)"),
    (
        598,
        true,
        "(Informal convention) Network read timeout error",
    ),
];

/// Render a filtered catalog and its source. A filter includes unofficial entries;
/// only-unofficial still restricts the result when combined with a filter.
pub fn write_codes(
    mut out: impl Write,
    filter: Option<&str>,
    include: bool,
    only: bool,
    style: Style,
) -> io::Result<()> {
    let filter = filter.map(str::to_lowercase);
    writeln!(out, "HTTP Status Codes")?;
    for &(code, unofficial, description) in CODES {
        if (only && !unofficial) || (!only && !include && filter.is_none() && unofficial) {
            continue;
        }
        if filter.as_ref().is_some_and(|f| {
            !code.to_string().contains(f) && !description.to_lowercase().contains(f)
        }) {
            continue;
        }
        out.write_all(b"  ")?;
        style.paint(
            &mut out,
            status_color(reqwest::StatusCode::from_u16(code).unwrap()),
            code.to_string().as_bytes(),
        )?;
        writeln!(
            out,
            " {description}{}",
            if unofficial { " [unofficial]" } else { "" }
        )?;
    }
    writeln!(out, "Source: {SOURCE}")?;
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_filters_and_styling() {
        for (pattern, include, only, expected) in [
            (Some("420"), false, false, vec![420, 420]),
            (Some("NOT FOUND"), false, false, vec![404]),
            (Some("["), false, false, vec![]),
            (Some("201"), false, false, vec![201]),
            (Some("404"), true, true, vec![]),
            (Some("no matches"), false, false, vec![]),
        ] {
            let mut plain = Vec::new();
            write_codes(&mut plain, pattern, include, only, Style::Plain).unwrap();
            let text = String::from_utf8(plain.clone()).unwrap();
            let codes: Vec<u16> = text
                .lines()
                .filter(|l| l.starts_with("  "))
                .map(|l| l.trim().split(' ').next().unwrap().parse().unwrap())
                .collect();
            assert_eq!(codes, expected);
            let mut styled = Vec::new();
            write_codes(&mut styled, pattern, include, only, Style::Colorful).unwrap();
            assert_eq!(crate::colors::tests::strip_ansi(&styled), plain);
        }
        for (include, only) in [(false, false), (true, false), (false, true), (true, true)] {
            let mut out = Vec::new();
            write_codes(&mut out, None, include, only, Style::Plain).unwrap();
            let text = String::from_utf8(out).unwrap();
            assert_eq!(
                text.lines().filter(|s| s.starts_with("  ")).count(),
                CODES
                    .iter()
                    .filter(|(_, u, _)| if only { *u } else { include || !u })
                    .count()
            );
        }
        assert!(CODES.windows(2).all(|pair| pair[0].0 <= pair[1].0));
    }
    #[test]
    fn exact_output_and_errors() {
        let filter = "404";
        let mut out = Vec::new();
        write_codes(&mut out, Some(filter), false, false, Style::Plain).unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap(),
            format!("HTTP Status Codes\n  404 Not Found\nSource: {SOURCE}\n")
        );
        struct Sink(bool);
        impl Write for Sink {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                if self.0 {
                    Ok(bytes.len().min(1))
                } else {
                    Err(io::ErrorKind::BrokenPipe.into())
                }
            }
            fn flush(&mut self) -> io::Result<()> {
                Err(io::Error::other("flush"))
            }
        }
        for flush in [false, true] {
            assert!(write_codes(Sink(flush), None, false, false, Style::Plain).is_err());
        }
    }
}
