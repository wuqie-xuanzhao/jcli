#[path = "tests/aliases.rs"]
mod aliases;
#[path = "tests/parsing.rs"]
mod parsing;
#[path = "tests/routing.rs"]
mod routing;

use crate::cli::Cli;
use clap::Parser;

pub(crate) fn parse_cli(args: &[&str]) -> Cli {
    let full: Vec<String> = std::iter::once("j".to_string())
        .chain(args.iter().map(|s| s.to_string()))
        .collect();
    Cli::parse_from(full)
}

pub(crate) fn parse_cli_err(args: &[&str]) -> clap::Error {
    let full: Vec<String> = std::iter::once("j".to_string())
        .chain(args.iter().map(|s| s.to_string()))
        .collect();
    Cli::try_parse_from(full).unwrap_err()
}
