use crayon::*;
use std::env::args;

use clif::arg::*;
use clif::cmd::{Command, FromCli, Runner};
use clif::Cli;
use clif::Error;
use clif::Help;

fn main() {
    std::process::exit(go() as i32)
}

/// Glues the interface layer and backend logic for a smooth hand-off of data.
fn go() -> u8 {
    // parse the command-line arguments
    let mut cli = Cli::new().threshold(2).tokenize(args());

    match Sum::from_cli(&mut cli) {
        // construct the application
        Ok(app) => {
            std::mem::drop(cli);
            app.exec(&())
        }
        // report cli error
        Err(err) => {
            match err.as_quick_help() {
                Some(text) => println!("{}", text),
                None => eprintln!("{}: {}", "error".red().bold(), &err),
            }
            err.code()
        }
    }
}

type Digit = f32;

/// [Sum]` is a summation program that can add multiple unsigned 32-bit values together.
#[derive(PartialEq, Debug)]
struct Sum {
    nums: Vec<Digit>,
    verbose: bool,
}

impl Sum {
    /// Adds `lhs` and `rhs` together.
    fn run(&self) -> Digit {
        self.nums.iter().fold(Digit::default(), |acc, x| { acc + x })
    }
}

// enforce the `Addrs` struct to implement `FromCli` and `Command<T>` traits
impl Runner<()> for Sum {}

impl FromCli for Sum {
    fn from_cli<'c>(cli: &'c mut Cli) -> Result<Self, Error<'c>>
    where
        Self: Sized,
    {
        // set short help text in case of an error
        cli.help(
            Help::new()
                .quick_text(HELP)
                .flag(Flag::new("help").switch('h'))
                .ref_usage(USAGE_LINE..USAGE_LINE + 2),
        )?;
        let radd = Sum {
            verbose: cli.check_flag(Flag::new("verbose"))?,
            nums: cli.require_positional_all(Positional::new("num"))?,
        };
        // verify the cli has no additional arguments if this is the top-level command being parsed
        cli.is_empty()?;
        Ok(radd)
    }
}

impl Command<()> for Sum {
    type Status = u8;

    fn exec(&self, _: &()) -> Self::Status {
        let sum: Digit = self.run();
        if self.verbose == true {
            println!("{:?} = {}", self.nums, sum);
        } else {
            println!("{}", sum);
        }
        0
    }
}

// 0-indexed from `HELP` string
const USAGE_LINE: usize = 2;

const HELP: &str = "\
Computes the summation.

Usage:
    sum [options] <num>... 

Args:
    <num>       positive number

Options:
    --verbose   display computation work
";

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn backend_logic() {
        let app = Sum {
            nums: vec![1, 2, 3],
            verbose: false,
        };

        assert_eq!(app.run(), 6);
    }
}
