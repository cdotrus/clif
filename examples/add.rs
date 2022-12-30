use crayon::*;
use std::env::args;

use clif::arg::*;
use clif::cmd::{Command, FromCli, Runner};
use clif::Cli;
use clif::Error;
use clif::ErrorKind;
use clif::Help;

fn main() {
    std::process::exit(go() as i32)
}

/// Glues the interface layer and backend logic for a smooth hand-off of data.
fn go() -> u8 {
    // parse the command-line arguments
    let mut cli = Cli::new().threshold(2).tokenize(args());

    match Addrs::from_cli(&mut cli) {
        // construct the application
        Ok(app) => {
            std::mem::drop(cli);
            app.exec(&())
        }
        // report cli error
        Err(err) => {
            match err.kind() {
                ErrorKind::Help => println!("{}", &err),
                _ => eprintln!("{}: {}", "error".red().bold(), &err),
            }
            err.code()
        }
    }
}

/// `Addrs` is 'add-rust' that can add two unsigned 8-bit values together.
#[derive(PartialEq, Debug)]
struct Addrs {
    lhs: u8,
    rhs: u8,
    verbose: bool,
    count: Vec<u8>,
}

impl Addrs {
    /// Adds `lhs` and `rhs` together.
    fn run(&self) -> u16 {
        self.lhs as u16 + self.rhs as u16
    }
}

// enforce the `Addrs` struct to implement `FromCli` and `Command<T>` traits
impl Runner<()> for Addrs {}

impl FromCli for Addrs {
    fn from_cli<'c>(cli: &'c mut Cli) -> Result<Self, Error<'c>>
    where
        Self: Sized,
    {
        // set short help text in case of an error
        cli.check_help(
            Help::new()
                .quick_text(HELP)
                .flag(Flag::new("help").switch('h'))
                .ref_usage(USAGE_LINE..USAGE_LINE + 2),
        )?;
        let radd = Addrs {
            verbose: cli.check_flag(Flag::new("verbose"))?,
            count: cli
                .check_option_n(Optional::new("count").switch('c').value("n"), 3)?
                .unwrap_or(vec![]),
            lhs: cli.require_positional(Positional::new("lhs"))?,
            rhs: cli.require_positional(Positional::new("rhs"))?,
        };
        // verify the cli has no additional arguments if this is the top-level command being parsed
        cli.is_empty()?;
        Ok(radd)
    }
}

impl Command<()> for Addrs {
    type Status = u8;

    fn exec(&self, _: &()) -> Self::Status {
        let sum: u16 = self.run();
        if self.verbose == true {
            println!("{} + {} = {}", self.lhs, self.rhs, sum);
        } else {
            println!("{}", sum);
        }
        0
    }
}

// 0-indexed from `HELP` string
const USAGE_LINE: usize = 2;

const HELP: &str = "\
Adds two numbers together.

Usage:
    addrs [options] <lhs> <rhs> 

Args:
    <lhs>       left-hand operand
    <rhs>       right-hand operand

Options:
    --verbose   display computation work
";

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn backend_logic() {
        let app = Addrs {
            lhs: 10,
            rhs: 9,
            verbose: false,
            count: None,
        };

        assert_eq!(app.run(), 19);
    }
}
