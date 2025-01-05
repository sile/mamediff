use std::io::Write;

use crossterm::style::{Attribute, Attributes, ContentStyle, PrintStyledContent, StyledContent};
use mamediff::app::App;
use orfail::OrFail;

fn main() -> orfail::Result<()> {
    check_args().or_fail()?;

    let app = App::new().or_fail()?;
    app.run().or_fail()?;
    Ok(())
}

fn check_args() -> orfail::Result<()> {
    let Some(arg) = std::env::args().nth(1) else {
        return Ok(());
    };

    match arg.as_str() {
        "-h" | "--help" => {
            println!("Git diff editor");
            println!();
            println!(
                "{} {} [OPTIONS]",
                bold_underline("Usage:"),
                bold("mamediff"),
            );
            println!();
            println!("{}", bold_underline("Options:"));
            println!(" {}  Print help", bold(" -h, --help"));
            println!(" {}   Print version", bold(" --version"));

            std::process::exit(0);
        }
        "--version" => {
            println!("mamediff {}", env!("CARGO_PKG_VERSION"));
            std::process::exit(0);
        }
        _ => {
            let mut stderr = std::io::stderr();
            writeln!(
                stderr,
                "{} unexpected argment '{arg}' found",
                bold("error:"),
            )
            .or_fail()?;
            writeln!(stderr).or_fail()?;
            writeln!(
                stderr,
                "{} {} [OPTIONS]",
                bold_underline("Usage:"),
                bold("mamediff"),
            )
            .or_fail()?;
            writeln!(stderr).or_fail()?;
            writeln!(stderr, "For more information, try '--help'.").or_fail()?;

            std::process::exit(1);
        }
    }
}

fn bold(s: &str) -> PrintStyledContent<&str> {
    let content = StyledContent::new(
        ContentStyle {
            attributes: Attributes::default().with(Attribute::Bold),
            ..Default::default()
        },
        s,
    );
    PrintStyledContent(content)
}

fn bold_underline(s: &str) -> PrintStyledContent<&str> {
    let content = StyledContent::new(
        ContentStyle {
            attributes: Attributes::default()
                .with(Attribute::Bold)
                .with(Attribute::Underlined),
            ..Default::default()
        },
        s,
    );
    PrintStyledContent(content)
}
