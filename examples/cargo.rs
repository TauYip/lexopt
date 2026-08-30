//! A very partial unfaithful implementation of cargo's command line.
//!
//! This showcases some hairier patterns, like subcommands and custom value parsing.

use std::{path::PathBuf, str::FromStr};

use lexopt::{Arg, Parser};

use Arg::*;

type Error = Box<dyn core::error::Error>;

const HELP: &str = "\
Usage: cargo [+toolchain] [OPTIONS] [SUBCOMMAND]

OPTIONS:
    -h, --help         print usage.
    --color <COLOR>    set color
    --offline          set offline
    --quiet            execute in quiet mode
    --verbose          execute in verbose mode
";

fn main() -> Result<(), Error> {
    let mut settings = GlobalSettings {
        toolchain: "stable".to_owned(),
        color: Color::Auto,
        offline: false,
        quiet: false,
        verbose: false,
    };

    let mut parser = Parser::new(std::env::args());
    while let Some(arg) = parser.next()? {
        match arg {
            Long("help") | Short('h') => {
                println!("{}", HELP);
                std::process::exit(0);
            }
            Long("color") => {
                let mut value = parser.value()?;
                value.make_ascii_lowercase();
                settings.color = value.parse()?;
            }
            Long("offline") => {
                settings.offline = true;
            }
            Long("quiet") => {
                settings.quiet = true;
                settings.verbose = false;
            }
            Long("verbose") => {
                settings.verbose = true;
                settings.quiet = false;
            }
            Value(value) => match value.as_str() {
                value if value.starts_with('+') => {
                    settings.toolchain = value[1..].to_owned();
                }
                "install" => {
                    return install(settings, parser);
                }
                value => {
                    return Err(format!("unknown subcommand '{}'", value).into());
                }
            },
            _ => return Err(arg.unexpected().into()),
        }
    }

    println!("{}", HELP);
    Ok(())
}

#[derive(Debug)]
struct GlobalSettings {
    toolchain: String,
    color: Color,
    offline: bool,
    quiet: bool,
    verbose: bool,
}

// Subcommand.
fn install(settings: GlobalSettings, mut parser: Parser) -> Result<(), Error> {
    // Subcommand settings.
    let mut package: Option<String> = None;
    let mut root: Option<PathBuf> = None;
    let mut jobs: u16 = get_no_of_cpus();

    while let Some(arg) = parser.next()? {
        match arg {
            Long("help") | Short('h') => {
                println!("cargo install [OPTIONS] CRATE");
                std::process::exit(0);
            }
            Value(value) if package.is_none() => {
                package = Some(value);
            }
            Long("root") => {
                root = Some(parser.value()?.into());
            }
            Short('j') | Long("jobs") => {
                jobs = parser.value()?.parse()?;
            }
            _ => return Err(arg.unexpected().into()),
        }
    }

    println!("Settings: {:#?}", settings);
    println!(
        "Installing {} into {:?} with {} jobs",
        package.ok_or("missing CRATE argument")?,
        root,
        jobs
    );

    Ok(())
}

#[derive(Debug)]
enum Color {
    Auto,
    Always,
    Never,
}

// clap has a macro for this: https://docs.rs/clap/2.33.3/clap/macro.arg_enum.html
// We have to do it manually.
impl FromStr for Color {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "auto" => Ok(Color::Auto),
            "always" => Ok(Color::Always),
            "never" => Ok(Color::Never),
            _ => Err(format!(
                "Invalid style '{}' [pick from: auto, always, never]",
                s
            )),
        }
    }
}

fn get_no_of_cpus() -> u16 {
    4
}
