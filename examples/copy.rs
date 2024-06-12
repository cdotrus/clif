#![allow(dead_code)]

use cliproc::{cli, proc, stage::*, ExitCode};
use cliproc::{Arg, Cli, Command, Help};
use std::env;
use std::path::PathBuf;

type AnyError = Box<dyn std::error::Error>;

#[derive(Debug, PartialEq)]
pub struct Copy {
    src: PathBuf,
    dest: PathBuf,
    force: bool,
    verbose: bool,
    version: bool,
    list: bool,
    ignore_home: bool,
    ignore_work: bool,
    shells: Vec<String>,
}

impl Command for Copy {
    fn interpret(cli: &mut Cli<Memory>) -> cli::Result<Self> {
        // logic for interface priority to user manual and version shortcuts

        cli.help(Help::with("a short description of this program"))?;
        let verbose = cli.check(Arg::flag("verbose"))?;
        if verbose == true {
            cli.help(Help::with("a long description of this program"))?;
        }
        cli.raise_help()?;
        cli.lower_help();

        let version = cli.check(Arg::flag("version"))?;
        let list = cli.check(Arg::flag("list"))?;

        cli.help(Help::with("a short description of this program"))?;
        Ok(Self {
            verbose: cli.check(Arg::flag("verbose"))?,
            version: cli.check(Arg::flag("version"))?,
            force: cli.check(Arg::flag("force"))?,
            list: cli.check(Arg::flag("list"))?,
            ignore_work: cli.check(Arg::flag("ignore-work"))?,
            ignore_home: cli.check(Arg::flag("ignore-home"))?,
            shells: cli
                .get_all(Arg::option("shell").switch('s').value("key=value"))?
                .unwrap_or_default(),
            src: match list | version {
                false => cli.require(Arg::positional("src"))?,
                true => {
                    let _ = cli.get::<PathBuf>(Arg::positional("src"));
                    PathBuf::new()
                }
            },
            dest: match list | version {
                false => cli.require(Arg::positional("dest"))?,
                true => {
                    let _ = cli.get::<PathBuf>(Arg::positional("dest"));
                    PathBuf::new()
                }
            },
        })
    }

    fn execute(self) -> proc::Result {
        Ok(())
    }
}

fn main() -> ExitCode {
    Cli::default().parse(env::args()).go::<Copy>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_no_hazards() {
        let _ = Cli::default()
            .parse(vec!["copy", "--list"].into_iter().map(|f| f.to_string()))
            .go::<Copy>();
    }
}
