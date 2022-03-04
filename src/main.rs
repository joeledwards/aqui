mod args;

use colored::*;

fn main() {
    let args = args::parseArgs();

    print_colors();

    println!("TODO:");
    println!("- select argument parser (sub-command support)");
    println!("- select {} client crate", "HTTP".color("red"));
    println!("- select JSON parser crate");
    println!("- select CLI color crate");
    println!("- handle partial URLs (url crate?)");
    println!("- implement `aqui get`");

}

fn print_colors() {
    let colors = vec![
        "black",
        "red",
        "orange",
        "yellow",
        "green",
        "cyan",
        "blue",
        "purple",
        "magenta",
        "white",
    ];

    println!();
    println!("Colors supported by the {} crate:", "colored".yellow());
    for color in colors {
        println!("  {} -> {}", color, color.color(color))
    }

    println!();
    println!("And with backgrounds:");
    println!("  {}", "red on green".red().on_green());
    println!("  {}", "green on red".green().on_red());
    println!("  {}", "yellow on blue".yellow().on_blue());
    println!("  {}", "blue on yellow".blue().on_yellow());
    println!("  {}", "cyan on magenta".cyan().on_magenta());
    println!("  {}", "magenta on cyan".magenta().on_cyan());

    println!();
    println!("And true color support:");
    println!(" {}", "orange".truecolor(255, 127, 0));

    println!();
}