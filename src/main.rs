mod args;

use aqui::{output, request};
use args::{Cli, Command, Input};
use clap::Parser;
use std::{
    io::{self, IsTerminal},
    process::ExitCode,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

fn run(cli: Cli) -> request::Result<bool> {
    let stdout = io::stdout();
    let style = cli.color.style(stdout.is_terminal());
    let command = match cli.command {
        Command::StatusCodes(args) => {
            aqui::status::write_codes(
                stdout.lock(),
                args.filter.as_deref(),
                args.include_unofficial,
                args.only_unofficial,
                style,
            )?;
            return Ok(true);
        }
        Command::Ip(args) => {
            let (client, request) = request::build_get(
                aqui::ip::ENDPOINT,
                &[],
                Duration::from_millis(args.timeout),
                0,
            )?;
            let mut response = client.execute(request)?;
            let status = response.status();
            let data = aqui::ip::read_provider(&mut response, status)?;
            aqui::ip::write_info(stdout.lock(), &data, args.geo, args.format(), style)?;
            return Ok(true);
        }
        command => command,
    };
    let (method, args, input) = command.into_request();
    let preview = match &input {
        Input::Literal(value) => {
            value.as_bytes()[..value.len().min(output::PAYLOAD_PREVIEW_BYTES)].to_vec()
        }
        _ => vec![],
    };
    let literal_length = match &input {
        Input::Literal(value) => Some(value.len()),
        _ => None,
    };
    let from_stdin = matches!(input, Input::Stdin);
    let count = Arc::new(AtomicU64::new(0));
    let payload = match input {
        Input::None => request::Payload::None,
        Input::Literal(value) => request::Payload::Bytes(value.into_bytes()),
        Input::Stdin => request::Payload::Stream(Box::new(aqui::upload::Counted {
            reader: io::stdin(),
            count: count.clone(),
        })),
    };
    let basic_auth = args.username.as_deref().map(|username| request::BasicAuth {
        username,
        password: args.password.as_deref().unwrap_or(""),
    });
    let (client, request) = request::build_request_with_options(
        method.clone(),
        &args.url,
        &args.headers,
        payload,
        request::RequestOptions {
            timeout: Duration::from_millis(args.timeout),
            max_redirects: args.max_redirects,
            basic_auth,
            allow_invalid_certs: args.allow_invalid_certs,
        },
    )?;
    let request_url = request.url().clone();
    let outbound_headers = request::outbound_headers(&request)?;
    let started = Instant::now();
    let mut response = client.execute(request)?;
    let status = response.status();
    let mut stdout = stdout.lock();
    if args.only_output_data {
        output::write_raw(&mut response, &mut stdout)?;
    } else {
        let body = output::read_body(&mut response, args.max_body_bytes)?;
        output::write_exchange_with_payload(
            &mut stdout,
            output::ResponseDetails {
                body: &body,
                headers: response.headers(),
                url: response.url(),
                status,
                elapsed: started.elapsed(),
            },
            Some(output::RequestDetails {
                method: &method,
                url: &request_url,
                headers: &outbound_headers,
            }),
            if let Some(total) = literal_length {
                output::PayloadDetails::Literal {
                    preview: &preview,
                    total,
                }
            } else if from_stdin {
                output::PayloadDetails::Stdin(count.load(Ordering::Relaxed))
            } else {
                output::PayloadDetails::None
            },
            style,
        )?;
    }
    Ok(output::status_success(status, args.fail))
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            if error
                .downcast_ref::<io::Error>()
                .is_none_or(|e| e.kind() != io::ErrorKind::BrokenPipe)
            {
                eprintln!("aqui: {error}");
            }
            ExitCode::FAILURE
        }
    }
}
