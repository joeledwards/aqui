//! Node aquí-inspired truecolor palette. Styling never changes the underlying text.
use reqwest::Url;
use std::io::{self, Write};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Style {
    Plain,
    #[default]
    Colorful,
}

#[derive(Clone, Copy)]
pub(crate) enum Color {
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Purple,
    Grey,
}

impl Style {
    pub(crate) fn paint(self, mut out: impl Write, color: Color, text: &[u8]) -> io::Result<()> {
        if self == Self::Colorful && !text.is_empty() {
            // Bright variants of the reference's dark-background palette.
            let rgb = match color {
                Color::Red => "221;68;51",
                Color::Orange => "238;136;51",
                Color::Yellow => "221;204;51",
                Color::Green => "102;187;51",
                Color::Blue => "51;170;221",
                Color::Purple => "153;85;255",
                Color::Grey => "170;170;170",
            };
            write!(out, "\x1b[38;2;{rgb}m")?;
            out.write_all(text)?;
            out.write_all(b"\x1b[0m")
        } else {
            out.write_all(text)
        }
    }

    pub(crate) fn url(self, mut out: impl Write, url: &Url) -> io::Result<()> {
        if self == Self::Plain || url.host_str().is_none() {
            return out.write_all(url.as_str().as_bytes());
        }
        let scheme_color = if url.scheme() == "https" {
            Color::Green
        } else {
            Color::Grey
        };
        self.paint(&mut out, scheme_color, url.scheme().as_bytes())?;
        out.write_all(b"://")?;
        // Slice the serialized URL so IPv6 brackets, escapes and repeated queries survive.
        let serialized = url.as_str();
        let authority_start = url.scheme().len() + 3;
        let authority_end = serialized[authority_start..]
            .find(['/', '?', '#'])
            .map(|n| authority_start + n)
            .unwrap_or(serialized.len());
        let authority = &serialized[authority_start..authority_end];
        // URL serialization includes a port only when it is not the scheme default.
        let host_end = if let Some(port) = url.port() {
            authority.len() - port.to_string().len() - 1
        } else {
            authority.len()
        };
        self.paint(&mut out, Color::Blue, &authority.as_bytes()[..host_end])?;
        if host_end < authority.len() {
            out.write_all(b":")?;
            self.paint(
                &mut out,
                Color::Orange,
                &authority.as_bytes()[host_end + 1..],
            )?;
        }
        for (index, segment) in url.path().split('/').enumerate() {
            if index != 0 {
                out.write_all(b"/")?;
            }
            self.paint(&mut out, Color::Purple, segment.as_bytes())?;
        }
        if let Some(query) = url.query() {
            out.write_all(b"?")?;
            for (index, field) in query.split('&').enumerate() {
                if index != 0 {
                    out.write_all(b"&")?;
                }
                if let Some((key, value)) = field.split_once('=') {
                    self.paint(&mut out, Color::Yellow, key.as_bytes())?;
                    out.write_all(b"=")?;
                    self.paint(&mut out, Color::Green, value.as_bytes())?;
                } else {
                    self.paint(&mut out, Color::Yellow, field.as_bytes())?;
                }
            }
        }
        if let Some(fragment) = url.fragment() {
            out.write_all(b"#")?;
            self.paint(&mut out, Color::Grey, fragment.as_bytes())?;
        }
        Ok(())
    }

    /// Highlight only JSON already validated and serialized by serde_json.
    /// Scanning serialized tokens avoids reformatting numbers, escapes or whitespace.
    pub(crate) fn json(self, mut out: impl Write, data: &[u8]) -> io::Result<()> {
        if self == Self::Plain {
            return out.write_all(data);
        }
        let mut i = 0;
        while i < data.len() {
            let start = i;
            let color = match data[i] {
                b'"' => {
                    i += 1;
                    while i < data.len() {
                        match data[i] {
                            b'\\' => i += 2,
                            b'"' => {
                                i += 1;
                                break;
                            }
                            _ => i += 1,
                        }
                    }
                    let next = data[i..].iter().find(|b| !b.is_ascii_whitespace());
                    if next == Some(&b':') {
                        Color::Yellow
                    } else {
                        Color::Green
                    }
                }
                b'-' | b'0'..=b'9' => {
                    i += 1;
                    while i < data.len()
                        && matches!(data[i], b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-')
                    {
                        i += 1;
                    }
                    Color::Orange
                }
                b't' | b'f' | b'n' => {
                    i += 1;
                    while i < data.len() && data[i].is_ascii_alphabetic() {
                        i += 1;
                    }
                    if data[start] == b'n' {
                        Color::Purple
                    } else {
                        Color::Blue
                    }
                }
                _ => {
                    out.write_all(&data[i..i + 1])?;
                    i += 1;
                    continue;
                }
            };
            self.paint(&mut out, color, &data[start..i])?;
        }
        Ok(())
    }
}

pub(crate) fn status_color(status: reqwest::StatusCode) -> Color {
    match status.as_u16() {
        100..=199 => Color::Blue,
        200..=299 => Color::Green,
        300..=399 => Color::Purple,
        400..=499 => Color::Red,
        _ => Color::Yellow,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn strip_ansi(bytes: &[u8]) -> Vec<u8> {
        let mut plain = Vec::new();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i..].starts_with(b"\x1b[") {
                i += 2;
                while bytes[i] != b'm' {
                    i += 1;
                }
                i += 1;
            } else {
                plain.push(bytes[i]);
                i += 1;
            }
        }
        plain
    }

    #[test]
    fn json_palette_and_serialized_tokens() {
        for (input, expected) in [
            ("true", "\x1b[38;2;51;170;221mtrue\x1b[0m"),
            ("false", "\x1b[38;2;51;170;221mfalse\x1b[0m"),
            ("null", "\x1b[38;2;153;85;255mnull\x1b[0m"),
            ("-12.50e+100", "\x1b[38;2;238;136;51m-12.50e+100\x1b[0m"),
            ("\"héllo\"", "\x1b[38;2;102;187;51m\"héllo\"\x1b[0m"),
            (
                "{\"k\": 1}",
                "{\x1b[38;2;221;204;51m\"k\"\x1b[0m: \x1b[38;2;238;136;51m1\x1b[0m}",
            ),
        ] {
            let mut output = Vec::new();
            Style::Colorful.json(&mut output, input.as_bytes()).unwrap();
            assert_eq!(output, expected.as_bytes());
            assert_eq!(strip_ansi(&output), input.as_bytes());
        }
        for input in [
            r#"{"a\"b\\c":"é😀\n\u0000", "arr": [true, false, null, 123456789012345678901234567890]}"#,
            "[]",
            "{}",
            "\"\"",
            r#"["colon:","comma,", "slash\\", "quote\""]"#,
        ] {
            let value: serde_json::Value = serde_json::from_str(input).unwrap();
            let bytes = serde_json::to_vec_pretty(&value).unwrap();
            let mut output = Vec::new();
            Style::Colorful.json(&mut output, &bytes).unwrap();
            assert_eq!(strip_ansi(&output), bytes);
            let mut output = Vec::new();
            Style::Plain.json(&mut output, &bytes).unwrap();
            assert_eq!(output, bytes);
        }
    }

    #[test]
    fn urls_preserve_serialization_and_components() {
        for input in [
            "http://example.com/",
            "https://example.com:8443/a%20b?x=1&x=2&bare&empty=&escaped=%26#frag",
            "http://[::1]:8080/a?",
            "https://example.com/é",
            "data:text/plain,hello",
        ] {
            let url = Url::parse(input).unwrap();
            for style in [Style::Plain, Style::Colorful] {
                let mut output = Vec::new();
                style.url(&mut output, &url).unwrap();
                assert_eq!(strip_ansi(&output), url.as_str().as_bytes());
                if style == Style::Plain {
                    assert_eq!(output, url.as_str().as_bytes());
                }
            }
        }
        let mut output = Vec::new();
        Style::Colorful
            .url(&mut output, &Url::parse("https://host:8443/a?q=x").unwrap())
            .unwrap();
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "\x1b[38;2;102;187;51mhttps\x1b[0m://\x1b[38;2;51;170;221mhost\x1b[0m:\x1b[38;2;238;136;51m8443\x1b[0m/\x1b[38;2;153;85;255ma\x1b[0m?\x1b[38;2;221;204;51mq\x1b[0m=\x1b[38;2;102;187;51mx\x1b[0m"
        );
    }
}
