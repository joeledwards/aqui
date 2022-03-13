use clap::{App, Arg, ArgMatches, SubCommand};

pub fn parseArgs() -> ArgMatches<'static> {
    let matches = App::new("aqui")
        .version("1.0")
        .author("Joel Edwards <joeledwards@gmail.com>")
        .about("CLI HTTP client aimed at maximum human friendliness")
        .subcommand(subcommandWithBody("delete"))
        .subcommand(subcommandNoBody("get"))
        .subcommand(subcommandNoBody("head"))
        .subcommand(subcommandNoBody("options"))
        .subcommand(subcommandWithBody("post"))
        .subcommand(subcommandWithBody("put"))
        .get_matches();

    matches
}

pub fn subcommandNoBody(name: &str) -> App {
    SubCommand::with_name(name).arg(Arg::with_name(name))
}

pub fn subcommandWithBody(name: &str) -> App {
    subcommandNoBody(name)
}
