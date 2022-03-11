extern crate clap;

use clap::{App, Arg, ArgMatches, SubCommand};

pub fn parseArgs() -> ArgMatches<'static> {
    let delete_command = SubCommand::with_name("delete")
        .arg(Arg::with_name("delete"));

    let get_command = SubCommand::with_name("get")
        .arg(Arg::with_name("get"));

    let head_command = SubCommand::with_name("head")
        .arg(Arg::with_name("head"));

    let options_command = SubCommand::with_name("options")
        .arg(Arg::with_name("options"));

    let post_command = SubCommand::with_name("patch")
        .arg(Arg::with_name("patch"));

    let post_command = SubCommand::with_name("post")
        .arg(Arg::with_name("post"));

    let put_command = SubCommand::with_name("put")
        .arg(Arg::with_name("put"));

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