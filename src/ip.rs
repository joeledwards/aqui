//! Public IP lookup using HTTPS ipwho.is. Decoding and presentation need no network.
use crate::colors::{Color, Style};
use serde_json::{Value, json};
use std::{
    io::{self, Read, Write},
    net::IpAddr,
};

pub const ENDPOINT: &str = "https://ipwho.is/";
pub const MAX_BYTES: u64 = 64 * 1024;

/// Read a successful provider response with a fixed byte bound and validate it.
pub fn read_provider(
    reader: impl io::Read,
    status: reqwest::StatusCode,
) -> crate::request::Result<Value> {
    if !status.is_success() {
        return Err(io::Error::other(format!("IP provider returned HTTP {status}")).into());
    }
    let mut body = Vec::new();
    reader.take(MAX_BYTES + 1).read_to_end(&mut body)?;
    if body.len() as u64 > MAX_BYTES {
        return Err(io::Error::other("IP provider response exceeds 65536 bytes").into());
    }
    let data = serde_json::from_slice(&body)?;
    decode(&data).map_err(io::Error::other)?;
    Ok(data)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Format {
    #[default]
    Human,
    Cli,
    CliPretty,
    Json,
    JsonPretty,
    Raw,
    RawPretty,
}

/// Validate the provider response and normalize to the Node aquí field names.
/// Missing optional fields are null; malformed supplied fields are errors.
pub fn decode(data: &Value) -> Result<Value, String> {
    if data.get("success").and_then(Value::as_bool) != Some(true) {
        return Err("IP provider did not report success".into());
    }
    for field in ["connection", "timezone"] {
        if data
            .get(field)
            .is_some_and(|value| !value.is_null() && !value.is_object())
        {
            return Err(format!("IP provider returned invalid {field}"));
        }
    }
    let ip = data
        .get("ip")
        .and_then(Value::as_str)
        .ok_or("IP provider omitted ip")?;
    ip.parse::<IpAddr>()
        .map_err(|_| "IP provider returned an invalid IP address")?;
    let mut record = json!({"ip": ip, "metroCode": null});
    for (name, pointer) in [
        ("isp", "/connection/isp"),
        ("org", "/connection/org"),
        ("countryCode", "/country_code"),
        ("countryName", "/country"),
        ("regionCode", "/region_code"),
        ("regionName", "/region"),
        ("city", "/city"),
        ("zipCode", "/postal"),
        ("timeZone", "/timezone/id"),
    ] {
        let value = data.pointer(pointer).cloned().unwrap_or(Value::Null);
        if !value.is_null() && !value.is_string() {
            return Err(format!("IP provider returned invalid {name}"));
        }
        record[name] = value;
    }
    let asn = data.pointer("/connection/asn").unwrap_or(&Value::Null);
    record["as"] = match asn {
        Value::Null => Value::Null,
        Value::Number(n) if n.as_u64().is_some() => {
            Value::String(format!("AS{}", n.as_u64().unwrap()))
        }
        _ => return Err("IP provider returned invalid ASN".into()),
    };
    for (name, bound) in [("latitude", 90.0), ("longitude", 180.0)] {
        let value = data.get(name).cloned().unwrap_or(Value::Null);
        if !value.is_null()
            && !value
                .as_f64()
                .is_some_and(|n| n.is_finite() && n.abs() <= bound)
        {
            return Err(format!("IP provider returned invalid {name}"));
        }
        record[name] = value;
    }
    Ok(record)
}

fn text(value: &Value) -> String {
    match value {
        Value::Null => "unknown".into(),
        Value::String(s) => s.clone(),
        _ => value.to_string(),
    }
}

/// Compact JSON and CLI modes stay plain; all pretty modes use the selected style.
pub fn write_info(
    mut out: impl Write,
    data: &Value,
    geo: bool,
    format: Format,
    style: Style,
) -> crate::request::Result<()> {
    let record = decode(data).map_err(io::Error::other)?;
    match format {
        Format::Json | Format::JsonPretty | Format::Raw | Format::RawPretty => {
            let value = if matches!(format, Format::Raw | Format::RawPretty) {
                data.clone()
            } else if geo {
                record
            } else {
                json!({"ip":record["ip"]})
            };
            let pretty = matches!(format, Format::JsonPretty | Format::RawPretty);
            let bytes = if pretty {
                serde_json::to_vec_pretty(&value)?
            } else {
                serde_json::to_vec(&value)?
            };
            if pretty {
                style.json(&mut out, &bytes)?;
            } else {
                out.write_all(&bytes)?;
            }
            writeln!(out)?;
        }
        Format::Cli | Format::CliPretty => {
            let style = if format == Format::Cli {
                Style::Plain
            } else {
                style
            };
            style.paint(&mut out, Color::Green, text(&record["ip"]).as_bytes())?;
            if geo {
                for (separator, key, color) in [
                    (" ", "countryCode", Color::Blue),
                    (",", "regionCode", Color::Yellow),
                    (",", "zipCode", Color::Yellow),
                    (" ", "latitude", Color::Orange),
                    (",", "longitude", Color::Orange),
                ] {
                    out.write_all(separator.as_bytes())?;
                    style.paint(&mut out, color, text(&record[key]).as_bytes())?;
                }
            }
            writeln!(out)?;
        }
        Format::Human => {
            for (label, key, color) in [
                ("IP Address", "ip", Color::Green),
                ("AS", "as", Color::Yellow),
                ("ISP", "isp", Color::Yellow),
                ("Country", "countryName", Color::Blue),
                ("Country Code", "countryCode", Color::Yellow),
                ("Region", "regionName", Color::Blue),
                ("Region Code", "regionCode", Color::Yellow),
                ("City", "city", Color::Blue),
                ("Zip Code", "zipCode", Color::Yellow),
                ("Metro Code", "metroCode", Color::Yellow),
                ("Latitude", "latitude", Color::Orange),
                ("Longitude", "longitude", Color::Orange),
                ("Time Zone", "timeZone", Color::Blue),
            ] {
                if key != "ip" && !geo {
                    continue;
                }
                write!(out, "{label:>12} : ")?;
                style.paint(
                    &mut out,
                    if record[key].is_null() {
                        Color::Grey
                    } else {
                        color
                    },
                    text(&record[key]).as_bytes(),
                )?;
                writeln!(out)?;
            }
        }
    }
    out.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Value {
        json!({"success":true,"ip":"2001:db8::1","country_code":"US","region_code":"OR","postal":"97818","latitude":45.0,"longitude":-119.0,"connection":{"asn":123,"isp":"Example"},"timezone":{"id":"Etc/UTC"}})
    }
    #[test]
    fn provider_read_errors_limits_and_status() {
        let body = serde_json::to_vec(&fixture()).unwrap();
        assert_eq!(
            read_provider(body.as_slice(), reqwest::StatusCode::OK).unwrap(),
            fixture()
        );
        for status in [
            reqwest::StatusCode::FOUND,
            reqwest::StatusCode::TOO_MANY_REQUESTS,
            reqwest::StatusCode::INTERNAL_SERVER_ERROR,
        ] {
            assert!(read_provider(body.as_slice(), status).is_err());
        }
        for body in [b"{".as_slice(), b"{}", b"", b"\xff"] {
            assert!(read_provider(body, reqwest::StatusCode::OK).is_err());
        }
        let mut exact = body.clone();
        exact.resize(MAX_BYTES as usize, b' ');
        assert!(read_provider(exact.as_slice(), reqwest::StatusCode::OK).is_ok());
        exact.push(b' ');
        assert!(read_provider(exact.as_slice(), reqwest::StatusCode::OK).is_err());
        struct Failed;
        impl io::Read for Failed {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                Err(io::ErrorKind::UnexpectedEof.into())
            }
        }
        assert!(read_provider(Failed, reqwest::StatusCode::OK).is_err());
    }

    #[test]
    fn validates_and_normalizes_provider_schema() {
        let record = decode(&fixture()).unwrap();
        assert_eq!(record["as"], "AS123");
        assert_eq!(record["isp"], "Example");
        assert!(record["city"].is_null());
        for bad in [
            json!(null),
            json!({}),
            json!({"success":false,"ip":"1.2.3.4"}),
            json!({"success":true,"ip":"bad"}),
        ] {
            assert!(decode(&bad).is_err());
        }
        for (key, value) in [
            ("latitude", json!(91)),
            ("longitude", json!(-181)),
            ("country", json!(1)),
            ("connection", json!(1)),
            ("timezone", json!("bad")),
            ("ip", json!(null)),
            ("success", json!("true")),
        ] {
            let mut data = fixture();
            data[key] = value;
            assert!(decode(&data).is_err(), "{key}");
        }
        let mut data = fixture();
        data["connection"]["asn"] = json!(-1);
        assert!(decode(&data).is_err());
        for ip in ["1.2.3.4", "::1"] {
            assert!(decode(&json!({"success":true,"ip":ip})).is_ok());
        }
        for bytes in [b"{bad".as_slice(), b"{", b"\xff"] {
            assert!(serde_json::from_slice::<Value>(bytes).is_err());
        }
    }
    #[test]
    fn output_modes_exact_and_color() {
        for (geo, format, expected) in [
            (false, Format::Cli, "2001:db8::1\n"),
            (true, Format::Cli, "2001:db8::1 US,OR,97818 45.0,-119.0\n"),
            (false, Format::Json, "{\"ip\":\"2001:db8::1\"}\n"),
            (
                false,
                Format::JsonPretty,
                "{\n  \"ip\": \"2001:db8::1\"\n}\n",
            ),
            (false, Format::Human, "  IP Address : 2001:db8::1\n"),
        ] {
            let mut out = Vec::new();
            write_info(&mut out, &fixture(), geo, format, Style::Plain).unwrap();
            assert_eq!(out, expected.as_bytes());
        }
        for format in [
            Format::Human,
            Format::Cli,
            Format::CliPretty,
            Format::Json,
            Format::JsonPretty,
            Format::Raw,
            Format::RawPretty,
        ] {
            let mut plain = Vec::new();
            let mut styled = Vec::new();
            write_info(&mut plain, &fixture(), true, format, Style::Plain).unwrap();
            write_info(&mut styled, &fixture(), true, format, Style::Colorful).unwrap();
            assert_eq!(crate::colors::tests::strip_ansi(&styled), plain);
            if matches!(
                format,
                Format::Json | Format::JsonPretty | Format::Raw | Format::RawPretty
            ) {
                if matches!(format, Format::JsonPretty | Format::RawPretty) {
                    assert!(styled.windows(2).any(|bytes| bytes == b"\x1b["));
                } else {
                    assert_eq!(styled, plain);
                }
                let value: Value = serde_json::from_slice(&plain).unwrap();
                assert_eq!(
                    value,
                    if matches!(format, Format::Raw | Format::RawPretty) {
                        fixture()
                    } else {
                        decode(&fixture()).unwrap()
                    }
                );
            }
        }
    }
    #[test]
    fn partial_writes_and_failures() {
        struct Sink {
            bytes: Vec<u8>,
            fail: bool,
            flush: bool,
        }
        impl Write for Sink {
            fn write(&mut self, b: &[u8]) -> io::Result<usize> {
                if self.fail {
                    return Err(io::ErrorKind::BrokenPipe.into());
                }
                let n = b.len().min(1);
                self.bytes.extend_from_slice(&b[..n]);
                Ok(n)
            }
            fn flush(&mut self) -> io::Result<()> {
                if self.flush {
                    Err(io::Error::other("flush"))
                } else {
                    Ok(())
                }
            }
        }
        for format in [
            Format::Human,
            Format::CliPretty,
            Format::Json,
            Format::RawPretty,
        ] {
            for (fail, flush) in [(false, false), (true, false), (false, true)] {
                let mut out = Sink {
                    bytes: vec![],
                    fail,
                    flush,
                };
                assert_eq!(
                    write_info(&mut out, &fixture(), true, format, Style::Colorful).is_err(),
                    fail || flush
                );
            }
        }
    }
}
