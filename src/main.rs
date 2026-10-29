mod args;

use aqui::{output, request};
use args::{Cli, Input};
use clap::Parser;
use std::{
    io::{self, IsTerminal},
    process::ExitCode,
    time::{Duration, Instant},
};

fn run(cli: Cli) -> request::Result<bool> {
    let (method, args, input) = cli.command.into_request();
    let payload = match input {
        Input::None => request::Payload::None,
        Input::Literal(value) => request::Payload::Bytes(value.into_bytes()),
        Input::Stdin => request::Payload::Stream(Box::new(io::stdin())),
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
    let stdout = io::stdout();
    let style = cli.color.style(stdout.is_terminal());
    let mut stdout = stdout.lock();
    if args.only_output_data {
        output::write_raw(&mut response, &mut stdout)?;
    } else {
        let body = output::read_body(&mut response, args.max_body_bytes)?;
        output::write_exchange(
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
