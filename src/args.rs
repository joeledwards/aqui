use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum ColorChoice {
    Always,
    #[default]
    Auto,
    Never,
}

impl ColorChoice {
    pub fn style(self, is_terminal: bool) -> aqui::colors::Style {
        match self {
            Self::Always => aqui::colors::Style::Colorful,
            Self::Auto if is_terminal => aqui::colors::Style::Colorful,
            _ => aqui::colors::Style::Plain,
        }
    }
}

#[derive(Debug, Parser)]
#[command(version, about = "Human-friendly HTTP tools")]
pub struct Cli {
    /// Response colors: auto (default, terminal only), always, or never; raw output is unaffected
    #[arg(long, global = true, value_enum, default_value_t = ColorChoice::Auto)]
    pub color: ColorChoice,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// HTTP GET
    Get(HttpArgs),
    /// HTTP HEAD (response headers only)
    Head(HttpArgs),
    /// HTTP OPTIONS
    Options(HttpArgs),
    /// HTTP DELETE, optionally with a payload
    Delete(WriteArgs),
    /// HTTP PATCH, optionally with a payload
    Patch(WriteArgs),
    /// HTTP POST, optionally with a payload
    Post(WriteArgs),
    /// HTTP PUT, optionally with a payload
    Put(WriteArgs),
}

pub enum Input {
    None,
    Literal(String),
    Stdin,
}

impl Command {
    pub fn into_request(self) -> (reqwest::Method, HttpArgs, Input) {
        use reqwest::Method;
        match self {
            Self::Get(args) => (Method::GET, args, Input::None),
            Self::Head(args) => (Method::HEAD, args, Input::None),
            Self::Options(args) => (Method::OPTIONS, args, Input::None),
            Self::Delete(args) => args.into_request(Method::DELETE),
            Self::Patch(args) => args.into_request(Method::PATCH),
            Self::Post(args) => args.into_request(Method::POST),
            Self::Put(args) => args.into_request(Method::PUT),
        }
    }
}

#[derive(Debug, Args)]
#[command(disable_help_flag = true)]
pub struct WriteArgs {
    #[command(flatten)]
    pub http: HttpArgs,
    /// Exact UTF-8 payload bytes; mutually exclusive with stdin
    #[arg(index = 2, conflicts_with = "payload_from_stdin")]
    pub payload: Option<String>,
    /// Stream the payload from stdin without buffering or JSON detection
    #[arg(short = 'I', long, conflicts_with = "payload")]
    pub payload_from_stdin: bool,
}

impl WriteArgs {
    fn into_request(self, method: reqwest::Method) -> (reqwest::Method, HttpArgs, Input) {
        let input = if self.payload_from_stdin {
            Input::Stdin
        } else if let Some(payload) = self.payload {
            Input::Literal(payload)
        } else {
            Input::None
        };
        (method, self.http, input)
    }
}

#[derive(Debug, Args)]
#[command(disable_help_flag = true)]
pub struct HttpArgs {
    /// Print help
    #[arg(long, action = clap::ArgAction::Help)]
    pub help: Option<bool>,
    /// HTTP(S) URL; missing schemes default to http://
    #[arg(index = 1)]
    pub url: String,
    /// Custom request header (repeatable); empty values are allowed
    #[arg(short = 'h', long = "header", action = clap::ArgAction::Append)]
    pub headers: Vec<String>,
    /// Total request timeout in milliseconds, including response body
    #[arg(short = 't', long, default_value_t = 5000, value_parser = clap::value_parser!(u64).range(1..))]
    pub timeout: u64,
    /// Maximum redirects to follow; zero returns the first response
    #[arg(long, default_value_t = 5)]
    pub max_redirects: usize,
    /// Basic auth username (password defaults to empty); cannot contain a colon
    #[arg(short = 'u', long)]
    pub username: Option<String>,
    /// Basic auth password; requires --username
    #[arg(short = 'p', long, requires = "username")]
    pub password: Option<String>,
    /// Disable TLS certificate verification for this request's client only
    #[arg(short = 'C', long)]
    pub allow_invalid_certs: bool,
    /// Exit unsuccessfully for HTTP 4xx/5xx, retaining the response body
    #[arg(long)]
    pub fail: bool,
    /// Stream exact response bytes without formatting or added newline
    #[arg(short = 'D', long)]
    pub only_output_data: bool,
    /// Maximum buffered body size in normal display mode (bytes)
    #[arg(long, default_value_t = 8 * 1024 * 1024, value_parser = clap::value_parser!(u64).range(1..))]
    pub max_body_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn auth_tls_and_http_failure_options() {
        for method in ["get", "head", "options", "delete", "patch", "post", "put"] {
            let cli = Cli::try_parse_from(["aqui", method, "host"]).unwrap();
            let (_, args, _) = cli.command.into_request();
            assert!(args.username.is_none() && args.password.is_none());
            assert!(!args.allow_invalid_certs && !args.fail);
            for flags in [
                vec!["-u", "user", "-p", "p:a:ss", "-C", "--fail", "-D"],
                vec![
                    "--username",
                    "user",
                    "--password",
                    "",
                    "--allow-invalid-certs",
                    "--fail",
                ],
            ] {
                let mut command = vec!["aqui", method, "host"];
                command.extend(flags);
                let (_, args, _) = Cli::try_parse_from(command).unwrap().command.into_request();
                assert_eq!(args.username.as_deref(), Some("user"));
                assert!(args.password.is_some() && args.allow_invalid_certs && args.fail);
            }
            assert!(Cli::try_parse_from(["aqui", method, "host", "-p", "secret"]).is_err());
            let (_, args, _) = Cli::try_parse_from(["aqui", method, "host", "-u", "user"])
                .unwrap()
                .command
                .into_request();
            assert!(args.password.is_none());
        }
    }
    #[test]
    fn methods_and_payload_sources() {
        for name in ["get", "head", "options", "delete", "patch", "post", "put"] {
            let cli = Cli::try_parse_from(["aqui", name, "host", "-h", "X: y"]).unwrap();
            let (method, args, input) = cli.command.into_request();
            assert_eq!(method.as_str(), name.to_ascii_uppercase());
            assert_eq!(args.headers, ["X: y"]);
            assert!(matches!(input, Input::None));
            let writes = matches!(name, "delete" | "patch" | "post" | "put");
            for payload in ["", "{}", "héllo"] {
                let parsed = Cli::try_parse_from(["aqui", name, "host", payload]);
                assert_eq!(parsed.is_ok(), writes);
                if let Ok(cli) = parsed {
                    let (_, _, input) = cli.command.into_request();
                    assert!(matches!(input, Input::Literal(value) if value == payload));
                }
            }
            for flag in ["-I", "--payload-from-stdin"] {
                let parsed = Cli::try_parse_from(["aqui", name, "host", flag]);
                assert_eq!(parsed.is_ok(), writes);
                if let Ok(cli) = parsed {
                    assert!(matches!(cli.command.into_request().2, Input::Stdin));
                }
                assert!(Cli::try_parse_from(["aqui", name, "host", "data", flag]).is_err());
            }
            assert!(Cli::try_parse_from(["aqui", name]).is_err());
            assert_eq!(
                Cli::try_parse_from(["aqui", name, "--help"])
                    .unwrap_err()
                    .kind(),
                clap::error::ErrorKind::DisplayHelp
            );
        }
    }
    #[test]
    fn color_modes_and_terminal_detection() {
        use aqui::colors::Style;
        let cli = Cli::try_parse_from(["aqui", "get", "host"]).unwrap();
        assert_eq!(cli.color, ColorChoice::Auto);
        for (mode, terminal, expected) in [
            ("always", false, Style::Colorful),
            ("always", true, Style::Colorful),
            ("auto", false, Style::Plain),
            ("auto", true, Style::Colorful),
            ("never", false, Style::Plain),
            ("never", true, Style::Plain),
        ] {
            for args in [
                vec!["aqui", "--color", mode, "get", "host"],
                vec!["aqui", "get", "host", "--color", mode, "-D"],
            ] {
                let cli = Cli::try_parse_from(args).unwrap();
                assert_eq!(cli.color.style(terminal), expected);
            }
        }
        assert!(Cli::try_parse_from(["aqui", "get", "host", "--color", "invalid"]).is_err());
    }
    #[test]
    fn command_contract() {
        for args in [
            vec!["aqui"],
            vec!["aqui", "get"],
            vec!["aqui", "trace", "host"],
            vec!["aqui", "get", "host", "--timeout", "0"],
            vec!["aqui", "get", "host", "--timeout", "-1"],
            vec!["aqui", "get", "host", "--max-body-bytes", "0"],
            vec!["aqui", "get", "host", "--unknown"],
        ] {
            assert!(Cli::try_parse_from(args).is_err());
        }
        let cli = Cli::try_parse_from([
            "aqui",
            "get",
            "localhost:8080",
            "-h",
            "X: a",
            "--header",
            "X: b",
            "-D",
            "--max-redirects",
            "0",
        ])
        .unwrap();
        let (_, args, _) = cli.command.into_request();
        assert_eq!(args.headers, ["X: a", "X: b"]);
        assert_eq!(args.timeout, 5000);
        assert_eq!(args.max_redirects, 0);
        assert_eq!(args.max_body_bytes, 8 * 1024 * 1024);
        assert!(args.only_output_data);
    }
    #[test]
    fn help_and_version() {
        let version = Cli::try_parse_from(["aqui", "--version"]).unwrap_err();
        assert_eq!(
            version.to_string(),
            format!("aqui {}\n", env!("CARGO_PKG_VERSION"))
        );
        for args in [
            vec!["aqui", "--help"],
            vec!["aqui", "get", "--help"],
            vec!["aqui", "--version"],
        ] {
            let error = Cli::try_parse_from(args).unwrap_err();
            assert!(matches!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ));
        }
    }
}
